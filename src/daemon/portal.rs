// TESTING_NEEDED: This feature requires manual testing on a live desktop environment
//! XDG Desktop Portal integration for screenshots and screencasting.
//!
//! Talks to org.freedesktop.portal.Screenshot / ScreenCast via zbus on the session bus.
//! Uses the portal's request/response pattern: call method → get a handle →
//! listen for the Response signal → parse the result.

use serde_json::{Value, json};
use std::os::unix::io::AsRawFd;
use tokio::process::Command;
use tokio::sync::Mutex;
use zbus::Connection;

use std::sync::Arc;

const PORTAL_SERVICE: &str = "org.freedesktop.portal.Desktop";
const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
const SCREENSHOT_IFACE: &str = "org.freedesktop.portal.Screenshot";
const SCREENCAST_IFACE: &str = "org.freedesktop.portal.ScreenCast";

/// Active portal screencast session — holds the GStreamer child process.
pub struct ActiveScreencast {
    pub child: tokio::process::Child,
    pub output_path: String,
}

/// Take a screenshot via the XDG Screenshot portal.
///
/// Calls the Screenshot method on the portal, then listens for the Response
/// signal on the returned handle path to obtain the URI of the captured image.
pub async fn portal_screenshot(interactive: bool) -> anyhow::Result<Value> {
    let conn = Connection::session().await?;

    let token = format!("deskbrid_{}", std::process::id());
    let handle_token = zbus::zvariant::Value::new(token.as_str());

    let mut options: std::collections::HashMap<&str, zbus::zvariant::Value<'_>> =
        std::collections::HashMap::new();
    options.insert("handle_token", handle_token);
    options.insert("interactive", zbus::zvariant::Value::Bool(interactive));

    let reply = conn
        .call_method(
            Some(PORTAL_SERVICE),
            PORTAL_PATH,
            Some(SCREENSHOT_IFACE),
            "Screenshot",
            &("", options),
        )
        .await?;

    let _handle_path: zbus::zvariant::OwnedObjectPath = reply.body().deserialize()?;

    let sender = conn
        .unique_name()
        .map(|n| n.as_str().replace('.', "_"))
        .unwrap_or_default();
    let response_path = format!("/org/freedesktop/portal/desktop/request/{sender}/{token}");

    let result = wait_for_portal_response(&conn, &response_path).await?;

    if result.0 != 0 {
        anyhow::bail!(
            "portal screenshot request was cancelled or failed (response={})",
            result.0
        );
    }

    let uri = result
        .1
        .get("uri")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    Ok(json!({
        "ok": true,
        "method": "xdg_portal_screenshot",
        "uri": uri,
        "interactive": interactive,
    }))
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
pub async fn portal_screencast_start(
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

/// Spawn wf-recorder for wlroots compositors (reliable, no portal needed).
async fn start_wf_recorder(
    output_path: &str,
    active: &Arc<Mutex<Option<ActiveScreencast>>>,
) -> anyhow::Result<Value> {
    tracing::info!("Starting wf-recorder → {}", output_path);

    let mut cmd = Command::new("wf-recorder");
    cmd.arg("-f").arg(output_path);
    // wf-recorder uses SIGINT for clean stop with mp4 muxing
    cmd.arg("-c").arg("libx264"); // software encoding for reliability
    cmd.arg("-p").arg("preset=ultrafast"); // low latency
    cmd.arg("-x"); // use DMA-BUF if available

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

    // Step 2: SelectSources — request monitor capture (type 1 = screen)
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

    // Step 3: Start — get the PipeWire fd and stream info
    let (pw_fd, stream_node_id) = start_screencast(&conn, &session_handle).await?;
    tracing::info!(
        "ScreenCast started — pw_fd={}, stream_node={}",
        pw_fd.as_raw_fd(),
        stream_node_id
    );

    // Step 4: Spawn GStreamer pipeline
    let fd_num = pw_fd.as_raw_fd();

    // Clear CLOEXEC on the fd so the child process inherits it
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
    cmd.arg("-e"); // EOS on shutdown
    // Split the pipeline into args for gst-launch
    for arg in pipeline.split_whitespace() {
        cmd.arg(arg);
    }

    unsafe {
        cmd.pre_exec(move || Ok(()));
    }

    let child = cmd
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to spawn gst-launch-1.0: {}", e))?;

    // Store the process
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

/// Stop a running portal screencast.
pub async fn portal_screencast_stop(
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

/// Create a ScreenCast session, returns the session handle object path.
async fn create_screencast_session(conn: &Connection, token: &str) -> anyhow::Result<String> {
    let mut options: std::collections::HashMap<&str, zbus::zvariant::Value<'_>> =
        std::collections::HashMap::new();
    options.insert("session_handle_token", zbus::zvariant::Value::new(token));

    let reply = conn
        .call_method(
            Some(PORTAL_SERVICE),
            PORTAL_PATH,
            Some(SCREENCAST_IFACE),
            "CreateSession",
            &(options,),
        )
        .await?;

    let handle: zbus::zvariant::OwnedObjectPath = reply.body().deserialize()?;
    Ok(handle.to_string())
}

/// Select sources for the screencast session (monitor capture).
async fn select_screencast_sources(
    conn: &Connection,
    session_handle: &str,
    token: &str,
) -> anyhow::Result<()> {
    let mut options: std::collections::HashMap<&str, zbus::zvariant::Value<'_>> =
        std::collections::HashMap::new();
    options.insert("handle_token", zbus::zvariant::Value::new(token));
    // types: bitmask — 1 = monitor (screen), 2 = window, 4 = virtual
    options.insert("types", zbus::zvariant::Value::U32(1));
    // multiple: false — single source
    options.insert("multiple", zbus::zvariant::Value::Bool(false));

    let _reply = conn
        .call_method(
            Some(PORTAL_SERVICE),
            session_handle,
            Some(SCREENCAST_IFACE),
            "SelectSources",
            &(options,),
        )
        .await?;

    Ok(())
}

/// Start the screencast session, returns the PipeWire fd and first stream node ID.
async fn start_screencast(
    conn: &Connection,
    session_handle: &str,
) -> anyhow::Result<(std::os::unix::io::OwnedFd, u32)> {
    let options: std::collections::HashMap<&str, zbus::zvariant::Value<'_>> =
        std::collections::HashMap::new();

    let reply = conn
        .call_method(
            Some(PORTAL_SERVICE),
            session_handle,
            Some(SCREENCAST_IFACE),
            "Start",
            &("", options), // parent_window = "" (not needed for monitor)
        )
        .await?;

    // The reply body is: (handle: h, streams: a(uu))
    // handle is the PipeWire remote fd
    let body = reply.body();

    // Deserialize the fd first
    let pw_fd: zbus::zvariant::OwnedFd = body.deserialize()?;

    // Then the streams array
    let streams: Vec<(u32, u32)> = body.deserialize()?;
    let stream_node_id = streams.first().map(|(node_id, _)| *node_id).unwrap_or(0);

    Ok((pw_fd.into(), stream_node_id))
}

/// Build the response path for the given token.
async fn build_response_path(conn: &Connection, token: &str) -> anyhow::Result<String> {
    let sender = conn
        .unique_name()
        .map(|n| n.as_str().replace('.', "_"))
        .unwrap_or_default();
    Ok(format!(
        "/org/freedesktop/portal/desktop/request/{sender}/{token}"
    ))
}

/// Wait for a portal Response signal on the given object path.
///
/// Portal Response signals have signature (u, a{sv}): (response_code, results).
/// Response code 0 = success, 1 = cancelled by user, 2 = error.
async fn wait_for_portal_response(
    conn: &Connection,
    expected_path: &str,
) -> anyhow::Result<(u32, std::collections::HashMap<String, serde_json::Value>)> {
    let expected = expected_path.to_string();
    let mut stream = zbus::MessageStream::from(conn.clone());

    let result = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        use futures_util::StreamExt;
        while let Some(msg) = stream.next().await {
            let Ok(msg) = msg else { continue };
            let header = msg.header();
            if header.message_type() != zbus::message::Type::Signal {
                continue;
            }
            let Some(iface) = header.interface() else {
                continue;
            };
            if iface.as_str() != "org.freedesktop.portal.Request" {
                continue;
            }
            let Some(member) = header.member() else {
                continue;
            };
            if member.as_str() != "Response" {
                continue;
            }
            let Some(path) = header.path() else {
                continue;
            };
            if path.as_str() != expected {
                continue;
            }
            return Some(msg);
        }
        None
    })
    .await;

    match result {
        Ok(Some(msg)) => {
            let body = msg.body();
            let response_code: u32 = body.deserialize()?;
            let results: std::collections::HashMap<String, zbus::zvariant::OwnedValue> =
                body.deserialize()?;

            let json_results: std::collections::HashMap<String, serde_json::Value> = results
                .into_iter()
                .map(|(k, v)| (k, owned_value_to_json(&v)))
                .collect();

            Ok((response_code, json_results))
        }
        Ok(None) => anyhow::bail!("portal response stream ended unexpectedly"),
        Err(_) => anyhow::bail!("portal response timed out after 30 seconds"),
    }
}

/// Convert a zvariant OwnedValue to a serde_json Value (best effort).
fn owned_value_to_json(value: &zbus::zvariant::OwnedValue) -> serde_json::Value {
    match value.value_signature().to_string().as_str() {
        "s" => value
            .downcast_ref::<String>()
            .map(|s| json!(s.as_str()))
            .unwrap_or(json!(null)),
        "b" => value
            .downcast_ref::<bool>()
            .map(|b| json!(b))
            .unwrap_or(json!(null)),
        "u" => value
            .downcast_ref::<u32>()
            .map(|u| json!(u))
            .unwrap_or(json!(null)),
        "i" => value
            .downcast_ref::<i32>()
            .map(|i| json!(i))
            .unwrap_or(json!(null)),
        "d" => value
            .downcast_ref::<f64>()
            .map(|d| json!(d))
            .unwrap_or(json!(null)),
        _ => json!(null),
    }
}
