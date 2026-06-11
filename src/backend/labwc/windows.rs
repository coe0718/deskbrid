use super::*;
use crate::protocol;

pub(super) async fn windows_list(
    backend: &LabwcBackend,
) -> anyhow::Result<Vec<protocol::WindowInfo>> {
    if backend.has_labwc_helper {
        let raw = backend.helper_json(&["list-windows"]).await?;
        return Ok(parse_labwc_windows_json(&raw));
    }
    // wlrctl fallback
    let raw = backend.sh("wlrctl", &["toplevel", "list"]).await?;
    let focused = backend
        .sh("wlrctl", &["toplevel", "get-focus"])
        .await
        .ok()
        .map(|s| s.trim().to_string());
    Ok(parse_wlrctl_windows(&raw, focused.as_deref()))
}

pub(super) async fn window_focus(backend: &LabwcBackend, id: &str) -> anyhow::Result<()> {
    if backend.has_labwc_helper {
        backend
            .helper_json(&["activate", "--window-id", id])
            .await
            .map(|_| ())
    } else {
        backend
            .sh("wlrctl", &["toplevel", "focus", id])
            .await
            .map(|_| ())
    }
}

pub(super) async fn window_get(
    backend: &LabwcBackend,
    id: &str,
) -> anyhow::Result<protocol::WindowInfo> {
    backend
        .windows_list()
        .await?
        .into_iter()
        .find(|w| w.id == id)
        .ok_or_else(|| anyhow::anyhow!("window not found: {}", id))
}

pub(super) async fn window_close(backend: &LabwcBackend, id: &str) -> anyhow::Result<()> {
    if backend.has_labwc_helper {
        backend
            .helper_json(&["close", "--window-id", id])
            .await
            .map(|_| ())
    } else {
        backend
            .sh("wlrctl", &["toplevel", "close", id])
            .await
            .map(|_| ())
    }
}

pub(super) async fn window_minimize(backend: &LabwcBackend, id: &str) -> anyhow::Result<()> {
    if backend.has_labwc_helper {
        backend
            .helper_json(&["minimize", "--window-id", id])
            .await
            .map(|_| ())
    } else {
        anyhow::bail!("minimize not available via wlrctl; install labwc-helper for full support")
    }
}

pub(super) async fn window_maximize(backend: &LabwcBackend, id: &str) -> anyhow::Result<()> {
    if backend.has_labwc_helper {
        backend
            .helper_json(&["maximize", "--window-id", id])
            .await
            .map(|_| ())
    } else {
        backend
            .sh("wlrctl", &["toplevel", "maximize", id])
            .await
            .map(|_| ())
    }
}

pub(super) async fn window_move_resize(
    _backend: &LabwcBackend,
    _id: &str,
    _x: i32,
    _y: i32,
    _w: u32,
    _h: u32,
) -> anyhow::Result<()> {
    // PROTOCOL LIMITATION: wlr-foreign-toplevel-management-unstable-v1
    // does not expose move/resize. wlrctl toplevel has no geometry commands.
    // Mouse simulation is the only path but fragile across window decorations.
    anyhow::bail!(
        "window move/resize is not supported by the Wayland toplevel protocol — use input.mouse.drag as a workaround"
    )
}
