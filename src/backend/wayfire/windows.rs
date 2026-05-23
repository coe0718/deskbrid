use super::*;
use crate::protocol;

pub(super) async fn windows_list(
    backend: &WayfireBackend,
) -> anyhow::Result<Vec<protocol::WindowInfo>> {
    let raw = backend.wf_ipc_json(&["-j"]).await?;
    Ok(parse_wayfire_views(&raw))
}

pub(super) async fn window_focus(backend: &WayfireBackend, id: &str) -> anyhow::Result<()> {
    backend.sh("wf-ipc", &["focus-view", id]).await.map(|_| ())
}

pub(super) async fn window_get(
    backend: &WayfireBackend,
    id: &str,
) -> anyhow::Result<protocol::WindowInfo> {
    backend
        .windows_list()
        .await?
        .into_iter()
        .find(|w| w.id == id)
        .ok_or_else(|| anyhow::anyhow!("window not found: {}", id))
}

pub(super) async fn window_close(backend: &WayfireBackend, id: &str) -> anyhow::Result<()> {
    backend.sh("wf-ipc", &["close-view", id]).await.map(|_| ())
}

pub(super) async fn window_minimize(backend: &WayfireBackend, id: &str) -> anyhow::Result<()> {
    backend
        .sh("wf-ipc", &["set-view-options", id, "minimized"])
        .await
        .map(|_| ())
}

pub(super) async fn window_maximize(backend: &WayfireBackend, id: &str) -> anyhow::Result<()> {
    backend
        .sh("wf-ipc", &["set-view-options", id, "fullscreen"])
        .await
        .map(|_| ())
}

pub(super) async fn window_move_resize(
    _backend: &WayfireBackend,
    _id: &str,
    _x: i32,
    _y: i32,
    _w: u32,
    _h: u32,
) -> anyhow::Result<()> {
    anyhow::bail!("window move/resize is not supported by wf-ipc")
}
