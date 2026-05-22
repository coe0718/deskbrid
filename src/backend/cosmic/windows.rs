use super::*;
use crate::protocol;

pub(super) async fn windows_list(
    backend: &CosmicBackend,
) -> anyhow::Result<Vec<protocol::WindowInfo>> {
    let json = backend.helper_json(&["list-windows"]).await?;
    // Parse the JSON array, converting cosmic helper format to protocol format
    #[derive(serde::Deserialize)]
    struct HelperWindow {
        window_id: u64,
        title: Option<String>,
        app_id: Option<String>,
        pid: Option<u32>,
        x: Option<i32>,
        y: Option<i32>,
        width: Option<u32>,
        height: Option<u32>,
        focused: bool,
        minimized: bool,
        workspace_id: Option<u32>,
    }
    let helper_windows: Vec<HelperWindow> = serde_json::from_value(json)?;
    let windows = helper_windows
        .into_iter()
        .map(|w| protocol::WindowInfo {
            id: w.window_id.to_string(),
            title: w.title.unwrap_or_default(),
            app_id: w.app_id.unwrap_or_default(),
            workspace_id: w.workspace_id.unwrap_or(0),
            is_focused: w.focused,
            is_minimized: w.minimized,
            geometry: match (w.x, w.y, w.width, w.height) {
                (Some(x), Some(y), Some(width), Some(height)) => Some(protocol::Geometry {
                    x,
                    y,
                    width,
                    height,
                }),
                _ => None,
            },
            pid: w.pid,
        })
        .collect();
    Ok(windows)
}

pub(super) async fn window_focus(backend: &CosmicBackend, id: &str) -> anyhow::Result<()> {
    let nid: u64 = id.parse().unwrap_or(0);
    backend
        .helper_run(&["activate", "--window-id", &nid.to_string()])
        .await
}

pub(super) async fn window_get(
    backend: &CosmicBackend,
    id: &str,
) -> anyhow::Result<protocol::WindowInfo> {
    let windows = backend.windows_list().await?;
    windows
        .into_iter()
        .find(|w| w.id == id)
        .ok_or_else(|| anyhow::anyhow!("window {} not found", id))
}

pub(super) async fn window_close(backend: &CosmicBackend, id: &str) -> anyhow::Result<()> {
    let nid: u64 = id.parse().unwrap_or(0);
    backend
        .helper_run(&["close", "--window-id", &nid.to_string()])
        .await
}

pub(super) async fn window_minimize(backend: &CosmicBackend, id: &str) -> anyhow::Result<()> {
    let nid: u64 = id.parse().unwrap_or(0);
    backend
        .helper_run(&["minimize", "--window-id", &nid.to_string()])
        .await
}

pub(super) async fn window_maximize(backend: &CosmicBackend, id: &str) -> anyhow::Result<()> {
    let nid: u64 = id.parse().unwrap_or(0);
    backend
        .helper_run(&["maximize", "--window-id", &nid.to_string()])
        .await
}

pub(super) async fn window_move_resize(
    _backend: &CosmicBackend,
    _id: &str,
    _x: i32,
    _y: i32,
    _width: u32,
    _height: u32,
) -> anyhow::Result<()> {
    anyhow::bail!("window move/resize not yet supported on COSMIC")
}

// ─── Workspaces ─────────────────────────────────────
