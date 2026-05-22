use super::*;
use crate::protocol;

pub(super) async fn windows_list(
    backend: &HyprBackend,
) -> anyhow::Result<Vec<protocol::WindowInfo>> {
    let json = backend.hyprctl_json(&["clients"]).await?;
    let arr = json
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("expected JSON array"))?;
    Ok(arr
        .iter()
        .map(HyprBackend::hyprctl_client_to_window)
        .collect())
}

pub(super) async fn window_focus(backend: &HyprBackend, id: &str) -> anyhow::Result<()> {
    let target = backend.resolve_window(id).await?;
    backend
        .hyprctl_dispatch(&format!("focuswindow address:{}", target.id))
        .await
}

pub(super) async fn window_get(
    backend: &HyprBackend,
    id: &str,
) -> anyhow::Result<protocol::WindowInfo> {
    backend.resolve_window(id).await
}

pub(super) async fn window_close(backend: &HyprBackend, id: &str) -> anyhow::Result<()> {
    let target = backend.resolve_window(id).await?;
    backend
        .hyprctl_dispatch(&format!("closewindow address:{}", target.id))
        .await
}

pub(super) async fn window_minimize(_backend: &HyprBackend, _id: &str) -> anyhow::Result<()> {
    anyhow::bail!("Hyprland does not expose a native minimize dispatcher")
}

pub(super) async fn window_maximize(backend: &HyprBackend, id: &str) -> anyhow::Result<()> {
    let target = backend.resolve_window(id).await?;
    backend
        .hyprctl_dispatch(&format!("focuswindow address:{}", target.id))
        .await?;
    if backend
        .hyprctl_dispatch("fullscreenstate 1 1 set")
        .await
        .is_ok()
    {
        return Ok(());
    }
    if backend
        .window_is_fullscreen(&target.id)
        .await
        .unwrap_or(false)
    {
        return Ok(());
    }
    backend.hyprctl_dispatch("fullscreenstate 1 1").await
}

pub(super) async fn window_move_resize(
    backend: &HyprBackend,
    id: &str,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> anyhow::Result<()> {
    let target = backend.resolve_window(id).await?;
    backend
        .hyprctl_dispatch(&format!(
            "movewindowpixel exact {} {},address:{}",
            x, y, target.id
        ))
        .await?;
    backend
        .hyprctl_dispatch(&format!(
            "resizewindowpixel exact {} {},address:{}",
            width, height, target.id
        ))
        .await
}
