use super::*;
use crate::protocol;

pub(super) async fn workspaces_list(
    _backend: &WayfireBackend,
) -> anyhow::Result<Vec<protocol::WorkspaceInfo>> {
    Ok(parse_wayfire_workspaces(&serde_json::Value::Null))
}

pub(super) async fn workspace_switch(backend: &WayfireBackend, id: u32) -> anyhow::Result<()> {
    backend
        .sh("wf-ipc", &["set-workspace", &id.to_string()])
        .await
        .map(|_| ())
}

pub(super) async fn workspace_move_window(
    backend: &WayfireBackend,
    window_id: &str,
    workspace_id: u32,
    _follow: bool,
) -> anyhow::Result<()> {
    backend
        .sh(
            "wf-ipc",
            &["set-view-workspace", window_id, &workspace_id.to_string()],
        )
        .await
        .map(|_| ())
}

// ─── Shared wlroots infra ───

pub(super) async fn keyboard_type(backend: &WayfireBackend, text: &str) -> anyhow::Result<()> {
    backend.ydotool(&["type", text]).await
}

pub(super) async fn keyboard_key(backend: &WayfireBackend, key: &str) -> anyhow::Result<()> {
    backend.ydotool(&["key", key]).await
}

pub(super) async fn keyboard_combo(
    backend: &WayfireBackend,
    keys: &[String],
) -> anyhow::Result<()> {
    for k in keys {
        backend.ydotool(&["key", &format!("{}:1", k)]).await?;
    }
    for k in keys.iter().rev() {
        backend.ydotool(&["key", &format!("{}:0", k)]).await?;
    }
    Ok(())
}

pub(super) async fn mouse_move(backend: &WayfireBackend, x: f64, y: f64) -> anyhow::Result<()> {
    backend
        .ydotool(&["mousemove", "--absolute", &x.to_string(), &y.to_string()])
        .await
}

pub(super) async fn mouse_click(backend: &WayfireBackend, button: &str) -> anyhow::Result<()> {
    let code: &str = match button {
        "left" => "0xC0",
        "middle" => "0xC1",
        "right" => "0xC2",
        _ => "0xC0",
    };
    backend.ydotool(&["click", code]).await
}

pub(super) async fn mouse_scroll(backend: &WayfireBackend, dx: f64, dy: f64) -> anyhow::Result<()> {
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

pub(super) async fn mouse_drag(
    backend: &WayfireBackend,
    from_x: f64,
    from_y: f64,
    to_x: f64,
    to_y: f64,
    button: &str,
    duration_ms: Option<u64>,
) -> anyhow::Result<()> {
    let (down_mask, up_mask) = ydotool_drag_masks(button)?;
    mouse_move(backend, from_x, from_y).await?;
    backend.ydotool(&["click", down_mask]).await?;
    if let Some(duration_ms) = duration_ms.filter(|duration| *duration > 0) {
        tokio::time::sleep(std::time::Duration::from_millis(duration_ms.min(5_000))).await;
    }
    mouse_move(backend, to_x, to_y).await?;
    backend.ydotool(&["click", up_mask]).await
}

fn ydotool_drag_masks(button: &str) -> anyhow::Result<(&'static str, &'static str)> {
    match button {
        "left" => Ok(("0x40", "0x80")),
        "middle" => Ok(("0x42", "0x82")),
        "right" => Ok(("0x41", "0x81")),
        _ => anyhow::bail!("unknown button: {}", button),
    }
}

pub(super) async fn clipboard_read(backend: &WayfireBackend) -> anyhow::Result<String> {
    backend.sh("wl-paste", &[]).await
}

pub(super) async fn clipboard_write(backend: &WayfireBackend, text: &str) -> anyhow::Result<()> {
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
