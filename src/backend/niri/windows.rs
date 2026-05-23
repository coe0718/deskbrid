use super::*;
use crate::protocol;

pub(super) async fn windows_list(
    backend: &NiriBackend,
) -> anyhow::Result<Vec<protocol::WindowInfo>> {
    let raw = backend.niri_json(&["windows"]).await?;
    Ok(parse_niri_windows(&raw))
}

pub(super) async fn window_focus(backend: &NiriBackend, id: &str) -> anyhow::Result<()> {
    backend.niri_cmd(&["focus-window", id]).await
}

pub(super) async fn window_get(
    backend: &NiriBackend,
    id: &str,
) -> anyhow::Result<protocol::WindowInfo> {
    backend
        .windows_list()
        .await?
        .into_iter()
        .find(|w| w.id == id)
        .ok_or_else(|| anyhow::anyhow!("window not found: {}", id))
}

pub(super) async fn window_close(backend: &NiriBackend, id: &str) -> anyhow::Result<()> {
    backend.niri_cmd(&["close-window", id]).await
}

pub(super) async fn window_minimize(_backend: &NiriBackend, _id: &str) -> anyhow::Result<()> {
    anyhow::bail!("Niri does not expose a minimize concept")
}

pub(super) async fn window_maximize(backend: &NiriBackend, id: &str) -> anyhow::Result<()> {
    backend
        .niri_cmd(&["set-window-column-width", id, "1.fr"])
        .await
}

pub(super) async fn window_move_resize(
    backend: &NiriBackend,
    id: &str,
    _x: i32,
    _y: i32,
    width: u32,
    _height: u32,
) -> anyhow::Result<()> {
    backend
        .niri_cmd(&["set-window-column-width", id, &format!("{}px", width)])
        .await
}
