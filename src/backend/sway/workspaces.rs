use super::*;
use crate::protocol;

pub(super) async fn workspaces_list(
    backend: &SwayBackend,
) -> anyhow::Result<Vec<protocol::WorkspaceInfo>> {
    let raw = backend.swaymsg_json(&["-t", "get_workspaces"]).await?;
    Ok(parse_sway_workspaces(&raw))
}

pub(super) async fn workspace_switch(backend: &SwayBackend, id: u32) -> anyhow::Result<()> {
    backend
        .swaymsg_raw(&["workspace", "number", &id.to_string()])
        .await
}

pub(super) async fn workspace_move_window(
    backend: &SwayBackend,
    window_id: &str,
    workspace_id: u32,
    _follow: bool,
) -> anyhow::Result<()> {
    backend
        .swaymsg_raw(&[
            "[con_id=",
            window_id,
            "]",
            "move",
            "container",
            "to",
            "workspace",
            "number",
            &workspace_id.to_string(),
        ])
        .await
}

// ─── Input (ydotool) ─────────────────────────────

pub(super) async fn keyboard_type(backend: &SwayBackend, text: &str) -> anyhow::Result<()> {
    backend.ydotool(&["type", text]).await
}

pub(super) async fn keyboard_key(backend: &SwayBackend, key: &str) -> anyhow::Result<()> {
    backend.ydotool(&["key", key]).await
}

pub(super) async fn keyboard_combo(backend: &SwayBackend, keys: &[String]) -> anyhow::Result<()> {
    for key in keys {
        backend.ydotool(&["key", &format!("{}:1", key)]).await?;
    }
    for key in keys.iter().rev() {
        backend.ydotool(&["key", &format!("{}:0", key)]).await?;
    }
    Ok(())
}

pub(super) async fn mouse_move(backend: &SwayBackend, x: f64, y: f64) -> anyhow::Result<()> {
    backend
        .ydotool(&["mousemove", "--absolute", &x.to_string(), &y.to_string()])
        .await
}

pub(super) async fn mouse_click(backend: &SwayBackend, button: &str) -> anyhow::Result<()> {
    let b = match button.to_lowercase().as_str() {
        "left" => "0xC0",
        "right" => "0xC1",
        "middle" => "0xC2",
        _ => anyhow::bail!("unknown button: {button}"),
    };
    backend.ydotool(&["click", b]).await
}

pub(super) async fn mouse_scroll(backend: &SwayBackend, dx: f64, dy: f64) -> anyhow::Result<()> {
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
    backend: &SwayBackend,
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
    // ydotool uses hex button masks: 0x40 = down, 0x80 = up
    // OR with button code: left=0x00, right=0x01, middle=0x02
    match button {
        "left" => Ok(("0x40", "0x80")),
        "right" => Ok(("0x41", "0x81")),
        "middle" => Ok(("0x42", "0x82")),
        _ => anyhow::bail!("unknown button: {button}"),
    }
}

// ─── Clipboard ────────────────────────────────────

pub(super) async fn clipboard_read(backend: &SwayBackend) -> anyhow::Result<String> {
    backend.sh("wl-paste", &[]).await
}

pub(super) async fn clipboard_write(backend: &SwayBackend, text: &str) -> anyhow::Result<()> {
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

// ─── Screenshot ───────────────────────────────────
