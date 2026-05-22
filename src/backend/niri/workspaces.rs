use super::*;
use crate::protocol;

pub(super) async fn workspaces_list(
    backend: &NiriBackend,
) -> anyhow::Result<Vec<protocol::WorkspaceInfo>> {
    let raw = backend.niri_json(&["workspaces"]).await?;
    Ok(parse_niri_workspaces(&raw))
}

pub(super) async fn workspace_switch(backend: &NiriBackend, id: u32) -> anyhow::Result<()> {
    backend
        .niri_cmd(&["switch-workspace", &id.to_string()])
        .await
}

pub(super) async fn workspace_move_window(
    backend: &NiriBackend,
    window_id: &str,
    workspace_id: u32,
    _follow: bool,
) -> anyhow::Result<()> {
    backend
        .niri_cmd(&[
            "move-window-to-workspace",
            window_id,
            &workspace_id.to_string(),
        ])
        .await
}

// ─── Shared wlroots infra (identical to Sway) ───
// keyboard, mouse, clipboard, screenshot, notifications, system, network,
// bluetooth, files, audio, monitor methods below are identical to SwayBackend

pub(super) async fn keyboard_type(backend: &NiriBackend, text: &str) -> anyhow::Result<()> {
    backend.ydotool(&["type", text]).await
}

pub(super) async fn keyboard_key(backend: &NiriBackend, key: &str) -> anyhow::Result<()> {
    backend.ydotool(&["key", key]).await
}

pub(super) async fn keyboard_combo(backend: &NiriBackend, keys: &[String]) -> anyhow::Result<()> {
    for key in keys {
        backend.ydotool(&["key", &format!("{}:1", key)]).await?;
    }
    for key in keys.iter().rev() {
        backend.ydotool(&["key", &format!("{}:0", key)]).await?;
    }
    Ok(())
}

pub(super) async fn mouse_move(backend: &NiriBackend, x: f64, y: f64) -> anyhow::Result<()> {
    backend
        .ydotool(&["mousemove", "--absolute", &x.to_string(), &y.to_string()])
        .await
}

pub(super) async fn mouse_click(backend: &NiriBackend, button: &str) -> anyhow::Result<()> {
    let btn: u8 = match button.to_lowercase().as_str() {
        "left" => 1,
        "middle" => 2,
        "right" => 3,
        _ => 1,
    };
    backend.ydotool(&["click", &btn.to_string()]).await
}

pub(super) async fn mouse_scroll(backend: &NiriBackend, dx: f64, dy: f64) -> anyhow::Result<()> {
    if dy != 0.0 {
        backend
            .ydotool(&["mousemove", "--wheel", "0", &format!("{}", dy as i32)])
            .await?;
    }
    if dx != 0.0 {
        backend
            .ydotool(&["mousemove", "--wheel", &format!("{}", dx as i32), "0"])
            .await?;
    }
    Ok(())
}

pub(super) async fn clipboard_read(backend: &NiriBackend) -> anyhow::Result<String> {
    backend.sh("wl-paste", &[]).await
}

pub(super) async fn clipboard_write(backend: &NiriBackend, text: &str) -> anyhow::Result<()> {
    let mut cmd = Command::new("wl-copy");
    cmd.stdin(Stdio::piped()).stderr(Stdio::piped());
    backend.apply_env(&mut cmd);
    let mut child = cmd.spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        stdin.write_all(text.as_bytes()).await?;
    }
    let output = child.wait_with_output().await?;
    if !output.status.success() {
        anyhow::bail!(
            "wl-copy failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}
