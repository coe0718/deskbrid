use super::*;
use crate::protocol;

pub(super) async fn windows_list(
    backend: &SwayBackend,
) -> anyhow::Result<Vec<protocol::WindowInfo>> {
    let tree = backend.swaymsg_json(&["-t", "get_tree"]).await?;
    Ok(parse_sway_tree_windows(&tree))
}

pub(super) async fn window_focus(backend: &SwayBackend, id: &str) -> anyhow::Result<()> {
    backend.swaymsg_raw(&["[con_id=", id, "]", "focus"]).await
}

pub(super) async fn window_get(
    backend: &SwayBackend,
    id: &str,
) -> anyhow::Result<protocol::WindowInfo> {
    let windows = backend.windows_list().await?;
    windows
        .into_iter()
        .find(|w| w.id == id)
        .ok_or_else(|| anyhow::anyhow!("window not found: {}", id))
}

pub(super) async fn window_close(backend: &SwayBackend, id: &str) -> anyhow::Result<()> {
    backend.swaymsg_raw(&["[con_id=", id, "]", "kill"]).await
}

pub(super) async fn window_minimize(backend: &SwayBackend, id: &str) -> anyhow::Result<()> {
    backend
        .swaymsg_raw(&["[con_id=", id, "]", "move", "scratchpad"])
        .await
}

pub(super) async fn window_maximize(backend: &SwayBackend, id: &str) -> anyhow::Result<()> {
    backend
        .swaymsg_raw(&["[con_id=", id, "]", "fullscreen", "toggle"])
        .await
}

pub(super) async fn window_move_resize(
    backend: &SwayBackend,
    id: &str,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> anyhow::Result<()> {
    // Sway requires floating windows for absolute positioning
    backend
        .swaymsg_raw(&["[con_id=", id, "]", "floating", "enable"])
        .await?;
    backend
        .swaymsg_raw(&[
            "[con_id=",
            id,
            "]",
            "move",
            "absolute",
            "position",
            &x.to_string(),
            &y.to_string(),
        ])
        .await?;
    backend
        .swaymsg_raw(&[
            "[con_id=",
            id,
            "]",
            "resize",
            "set",
            &width.to_string(),
            &height.to_string(),
        ])
        .await
}

// ─── Workspaces ───────────────────────────────────
