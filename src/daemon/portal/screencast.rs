//! Portal screencast — public start/stop API + wf-recorder + GStreamer paths.

use serde_json::{Value, json};
use std::os::unix::io::AsRawFd;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::Mutex;
use zbus::Connection;

use super::helpers::{
    build_response_path, create_screencast_session, select_screencast_sources, start_screencast,
    wait_for_portal_response,
};

/// Active portal screencast session — holds the GStreamer child process.
pub(crate) struct ActiveScreencast {
    pub child: tokio::process::Child,
    pub output_path: String,
}

/// Start a screencast session via the XDG ScreenCast portal.
///
/// Flow:
/// 1. CreateSession → get session handle
/// 2. SelectSources(session, {types: 1=monitor}) → wait for Response
/// 3. Start(session, "", {}) → get PipeWire fd + stream nodes
/// 4. Spawn gst-launch-1.0 with pipewiresrc reading the fd
/// 5. Store the child process for later stop
///
/// On wlroots compositors (Hyprland, Sway), uses wf-recorder directly.
/// On other environments, attempts the XDG ScreenCast portal API.
pub(crate) async fn portal_screencast_start(
    output_path: &str,
    active: &Arc<Mutex<Option<ActiveScreencast>>>,
) -> anyhow::Result<Value> {
    // Check if already recording
    {
        let guard = active.lock().await;
        if guard.is_some() {
            anyhow::bail!("a screencast is already active — stop it first");
        }
    }

    // Detect wlroots compositor for wf-recorder path (most reliable)
    let is_wlroots = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok()
        || std::env::var("SWAYSOCK").is_ok()
        || std::env::var("LABWC_PID").is_ok();

    if is_wlroots {
        return start_wf_recorder(output_path, active).await;
    }

    // Fallback: XDG ScreenCast portal
    start_portal_screencast(output_path, active).await
}

/// Stop a running portal screencast.
pub(crate) async fn portal_screencast_stop(
    active: &Arc<Mutex<Option<ActiveScreencast>>>,
) -> anyhow::Result<Value> {
    let mut guard = active.lock().await;
    match guard.take() {
        Some(mut session) => {
            let pid = session.child.id().unwrap_or(0) as i32;
            tracing::info!("Stopping screencast (pid={})", pid);

            // Send SIGINT for clean MP4 muxing (wf-recorder and gst-launch both handle it)
            unsafe {
                libc::kill(pid, libc::SIGINT);
            }

            // Wait up to 5 seconds for graceful exit, then force kill
            let wait_result =
                tokio::time::timeout(std::time::Duration::from_secs(5), session.child.wait()).await;

            if wait_result.is_err() {
                let _ = session.child.start_kill();
                let _ = session.child.wait().await;
            }

            Ok(json!({
                "ok": true,
                "output": session.output_path,
                "message": "Portal screencast stopped"
            }))
        }
        None => Ok(json!({
            "ok": true,
            "message": "No active screencast to stop"
        })),
    }
}

/// Spawn wf-recorder for wlroots compositors (reliable, no portal needed).
async fn start_wf_recorder(
    output_path: &str,
    active: &Arc<Mutex<Option<ActiveScreencast>>>,
) -> anyhow::Result<Value> {
    tracing::info!("Starting wf-recorder → {}", output_path);

    let mut cmd = Command::new("wf-recorder");
    cmd.arg("-f").arg(output_path);
    cmd.arg("-c").arg("libx264");
    cmd.arg("-p").arg("preset=ultrafast");
    cmd.arg("-x");

    let child = cmd
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to spawn wf-recorder: {}", e))?;

    let pid = child.id().unwrap_or(0);
    tracing::info!("wf-recorder started (pid={})", pid);

    {
        let mut guard = active.lock().await;
        *guard = Some(ActiveScreencast {
            child,
            output_path: output_path.to_string(),
        });
    }

    Ok(json!({
        "ok": true,
        "method": "wf-recorder",
        "output": output_path,
        "pid": pid,
    }))
}

/// Portal-based screencast via XDG ScreenCast + GStreamer pipewiresrc.
async fn start_portal_screencast(
    output_path: &str,
    active: &Arc<Mutex<Option<ActiveScreencast>>>,
) -> anyhow::Result<Value> {
    let conn = Connection::session().await?;
    let token = format!("deskbrid_sc_{}", std::process::id());

    // Step 1: CreateSession
    let session_handle = create_screencast_session(&conn, &token).await?;
    tracing::info!("ScreenCast session created: {}", session_handle);

    // Step 2: SelectSources
    select_screencast_sources(&conn, &session_handle, &token).await?;
    let response_path = build_response_path(&conn, &token).await?;
    let result = wait_for_portal_response(&conn, &response_path).await?;
    if result.0 != 0 {
        anyhow::bail!(
            "portal SelectSources was cancelled or failed (response={})",
            result.0
        );
    }
    tracing::info!("ScreenCast sources selected");

    // Step 3: Start
    let (pw_fd, stream_node_id) = start_screencast(&conn, &session_handle).await?;
    tracing::info!(
        "ScreenCast started — pw_fd={}, stream_node={}",
        pw_fd.as_raw_fd(),
        stream_node_id
    );

    // Step 4: Spawn GStreamer pipeline
    let fd_num = pw_fd.as_raw_fd();

    unsafe {
        let flags = libc::fcntl(fd_num, libc::F_GETFD);
        if flags >= 0 {
            libc::fcntl(fd_num, libc::F_SETFD, flags & !libc::FD_CLOEXEC);
        }
    }

    let pipeline = format!(
        "pipewiresrc fd={} path={} do-timestamp=true ! videoconvert ! x264enc tune=zerolatency ! mp4mux ! filesink location={}",
        fd_num, stream_node_id, output_path
    );

    tracing::info!("Launching GStreamer: {}", pipeline);

    let mut cmd = Command::new("gst-launch-1.0");
    cmd.arg("-e");
    for arg in pipeline.split_whitespace() {
        cmd.arg(arg);
    }

    unsafe {
        cmd.pre_exec(move || Ok(()));
    }

    let child = cmd
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to spawn gst-launch-1.0: {}", e))?;

    {
        let mut guard = active.lock().await;
        *guard = Some(ActiveScreencast {
            child,
            output_path: output_path.to_string(),
        });
    }

    Ok(json!({
        "ok": true,
        "method": "xdg_portal_screencast",
        "output": output_path,
        "pipeline": pipeline,
    }))
}
