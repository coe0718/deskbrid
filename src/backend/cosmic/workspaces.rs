use super::*;
use crate::protocol;

pub(super) async fn workspaces_list(
    backend: &CosmicBackend,
) -> anyhow::Result<Vec<protocol::WorkspaceInfo>> {
    let json = backend.helper_json(&["workspace-list"]).await?;
    let workspaces: Vec<protocol::WorkspaceInfo> = serde_json::from_value(json)?;
    Ok(workspaces)
}

pub(super) async fn workspace_switch(backend: &CosmicBackend, id: u32) -> anyhow::Result<()> {
    backend
        .helper_run(&["workspace-activate", "--id", &id.to_string()])
        .await
}

pub(super) async fn workspace_move_window(
    backend: &CosmicBackend,
    window_id: &str,
    workspace_id: u32,
    _follow: bool,
) -> anyhow::Result<()> {
    let nid: u64 = window_id.parse().unwrap_or(0);
    backend
        .helper_run(&[
            "move-to-workspace",
            "--window-id",
            &nid.to_string(),
            "--workspace-id",
            &workspace_id.to_string(),
        ])
        .await
}

// ─── Input ──────────────────────────────────────────

pub(super) async fn keyboard_type(backend: &CosmicBackend, text: &str) -> anyhow::Result<()> {
    backend.sh("ydotool", &["type", text]).await?;
    Ok(())
}

pub(super) async fn keyboard_key(backend: &CosmicBackend, key: &str) -> anyhow::Result<()> {
    backend.sh("ydotool", &["key", key]).await?;
    Ok(())
}

pub(super) async fn keyboard_combo(backend: &CosmicBackend, keys: &[String]) -> anyhow::Result<()> {
    // ydotool uses + for combos like "ctrl+alt+t"
    let combo = keys.join("+");
    backend.sh("ydotool", &["key", &combo]).await?;
    Ok(())
}

pub(super) async fn mouse_move(backend: &CosmicBackend, x: f64, y: f64) -> anyhow::Result<()> {
    backend
        .sh(
            "ydotool",
            &["mousemove", "--absolute", &x.to_string(), &y.to_string()],
        )
        .await?;
    *backend.last_mouse.lock().unwrap() = (x, y);
    Ok(())
}

pub(super) async fn mouse_click(backend: &CosmicBackend, button: &str) -> anyhow::Result<()> {
    let b = match button {
        "left" => "0xC0",
        "middle" => "0xC2",
        "right" => "0xC1",
        _ => anyhow::bail!("unknown button: {}", button),
    };
    backend.sh("ydotool", &["click", b]).await?;
    Ok(())
}

pub(super) async fn mouse_scroll(backend: &CosmicBackend, _dx: f64, dy: f64) -> anyhow::Result<()> {
    if dy >= 0.0 {
        backend.sh("ydotool", &["click", "4"]).await.map(|_| ())
    } else {
        backend.sh("ydotool", &["click", "5"]).await.map(|_| ())
    }
}

pub(super) async fn mouse_drag(
    backend: &CosmicBackend,
    from_x: f64,
    from_y: f64,
    to_x: f64,
    to_y: f64,
    button: &str,
    duration_ms: Option<u64>,
) -> anyhow::Result<()> {
    let (down_mask, up_mask) = ydotool_drag_masks(button)?;
    mouse_move(backend, from_x, from_y).await?;
    backend.sh("ydotool", &["click", down_mask]).await?;
    if let Some(duration_ms) = duration_ms.filter(|duration| *duration > 0) {
        tokio::time::sleep(std::time::Duration::from_millis(duration_ms.min(5_000))).await;
    }
    mouse_move(backend, to_x, to_y).await?;
    backend.sh("ydotool", &["click", up_mask]).await?;
    Ok(())
}

fn ydotool_drag_masks(button: &str) -> anyhow::Result<(&'static str, &'static str)> {
    // ydotool uses hex button masks: 0x40 = down, 0x80 = up
    // OR with button code: left=0x00, right=0x01, middle=0x02
    match button {
        "left" => Ok(("0x40", "0x80")),
        "right" => Ok(("0x41", "0x81")),
        "middle" => Ok(("0x42", "0x82")),
        _ => anyhow::bail!("unknown button: {}", button),
    }
}

// ─── Clipboard ──────────────────────────────────────

pub(super) async fn clipboard_read(backend: &CosmicBackend) -> anyhow::Result<String> {
    backend
        .sh("wl-paste", &[])
        .await
        .map(|s| s.trim().to_string())
}

pub(super) async fn clipboard_write(backend: &CosmicBackend, text: &str) -> anyhow::Result<()> {
    backend.sh("wl-copy", &[text]).await?;
    Ok(())
}

// ─── Screenshot ─────────────────────────────────────
