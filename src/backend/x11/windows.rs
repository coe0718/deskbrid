use super::*;
use crate::protocol;

pub(super) async fn windows_list(
    backend: &X11Backend,
) -> anyhow::Result<Vec<protocol::WindowInfo>> {
    let active_window_id = backend.sh("xdotool", &["getactivewindow"]).await.ok();
    let raw = backend.sh("wmctrl", &["-lGpx"]).await?;
    Ok(parse_wmctrl_windows(&raw, active_window_id.as_deref()))
}

pub(super) async fn window_focus(backend: &X11Backend, id: &str) -> anyhow::Result<()> {
    X11Backend::ensure_window_id(id)?;
    let normalized = normalize_window_id(id);
    backend
        .sh("xdotool", &["windowactivate", &normalized])
        .await
        .map(|_| ())
}

pub(super) async fn window_get(
    backend: &X11Backend,
    id: &str,
) -> anyhow::Result<protocol::WindowInfo> {
    X11Backend::ensure_window_id(id)?;
    let normalized = normalize_window_id(id);
    if let Ok(windows) = windows_list(backend).await
        && let Some(window) = windows
            .into_iter()
            .find(|window| normalize_window_id(&window.id) == normalized)
    {
        return Ok(window);
    }
    let title = backend
        .sh("xdotool", &["getwindowname", &normalized])
        .await
        .map_err(|_| anyhow::anyhow!("window not found: {}", id))?;
    Ok(protocol::WindowInfo {
        id: normalized,
        title,
        app_id: String::new(),
        workspace_id: 0,
        is_focused: false,
        is_minimized: false,
        geometry: None,
        pid: None,
    })
}

pub(super) async fn window_close(backend: &X11Backend, id: &str) -> anyhow::Result<()> {
    X11Backend::ensure_window_id(id)?;
    backend
        .sh("xdotool", &["windowclose", id])
        .await
        .map(|_| ())
}

pub(super) async fn window_minimize(backend: &X11Backend, id: &str) -> anyhow::Result<()> {
    X11Backend::ensure_window_id(id)?;
    backend
        .sh("xdotool", &["windowminimize", id])
        .await
        .map(|_| ())
}

pub(super) async fn window_maximize(backend: &X11Backend, id: &str) -> anyhow::Result<()> {
    X11Backend::ensure_window_id(id)?;
    backend
        .sh(
            "wmctrl",
            &["-ir", id, "-b", "add,maximized_vert,maximized_horz"],
        )
        .await
        .map(|_| ())
}

pub(super) async fn window_move_resize(
    backend: &X11Backend,
    id: &str,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> anyhow::Result<()> {
    X11Backend::ensure_window_id(id)?;
    backend
        .sh(
            "xdotool",
            &["windowmove", id, &x.to_string(), &y.to_string()],
        )
        .await?;
    backend
        .sh(
            "xdotool",
            &["windowsize", id, &width.to_string(), &height.to_string()],
        )
        .await
        .map(|_| ())
}

pub(super) async fn workspaces_list(
    backend: &X11Backend,
) -> anyhow::Result<Vec<protocol::WorkspaceInfo>> {
    let count = backend
        .sh("xdotool", &["get_num_desktops"])
        .await?
        .trim()
        .parse::<u32>()
        .unwrap_or(1);
    let active = backend
        .sh("xdotool", &["get_desktop"])
        .await
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .unwrap_or(0);
    Ok((0..count)
        .map(|id| protocol::WorkspaceInfo {
            id,
            name: format!("Desktop {}", id + 1),
            is_active: id == active,
        })
        .collect())
}

pub(super) async fn workspace_switch(backend: &X11Backend, id: u32) -> anyhow::Result<()> {
    backend
        .sh("xdotool", &["set_desktop", &id.to_string()])
        .await
        .map(|_| ())
}

pub(super) async fn workspace_move_window(
    backend: &X11Backend,
    window_id: &str,
    workspace_id: u32,
    follow: bool,
) -> anyhow::Result<()> {
    X11Backend::ensure_window_id(window_id)?;
    backend
        .sh(
            "xdotool",
            &[
                "set_desktop_for_window",
                window_id,
                &workspace_id.to_string(),
            ],
        )
        .await?;
    if follow {
        backend
            .sh("xdotool", &["set_desktop", &workspace_id.to_string()])
            .await?;
    }
    Ok(())
}

pub(super) async fn keyboard_type(backend: &X11Backend, text: &str) -> anyhow::Result<()> {
    backend.sh("xdotool", &["type", text]).await.map(|_| ())
}

pub(super) async fn keyboard_key(backend: &X11Backend, key: &str) -> anyhow::Result<()> {
    backend.sh("xdotool", &["key", key]).await.map(|_| ())
}

pub(super) async fn keyboard_combo(backend: &X11Backend, keys: &[String]) -> anyhow::Result<()> {
    backend
        .sh("xdotool", &["key", &keys.join("+")])
        .await
        .map(|_| ())
}

pub(super) async fn mouse_move(backend: &X11Backend, x: f64, y: f64) -> anyhow::Result<()> {
    backend
        .sh(
            "xdotool",
            &[
                "mousemove",
                &(x as i32).to_string(),
                &(y as i32).to_string(),
            ],
        )
        .await
        .map(|_| ())
}

pub(super) async fn mouse_click(backend: &X11Backend, button: &str) -> anyhow::Result<()> {
    let b = match button {
        "left" => "1",
        "middle" => "2",
        "right" => "3",
        _ => "1",
    };
    backend.sh("xdotool", &["click", b]).await.map(|_| ())
}

pub(super) async fn mouse_scroll(_backend: &X11Backend, _dx: f64, dy: f64) -> anyhow::Result<()> {
    let b = if dy >= 0.0 { "4" } else { "5" };
    _backend.sh("xdotool", &["click", b]).await.map(|_| ())
}

pub(super) async fn mouse_drag(
    backend: &X11Backend,
    from_x: f64,
    from_y: f64,
    to_x: f64,
    to_y: f64,
    button: &str,
    duration_ms: Option<u64>,
) -> anyhow::Result<()> {
    let b = match button {
        "left" => "1",
        "middle" => "2",
        "right" => "3",
        _ => anyhow::bail!("unknown button: {}", button),
    };
    backend
        .sh(
            "xdotool",
            &[
                "mousemove",
                &(from_x as i32).to_string(),
                &(from_y as i32).to_string(),
            ],
        )
        .await?;
    backend.sh("xdotool", &["mousedown", b]).await?;
    if let Some(duration_ms) = duration_ms.filter(|duration| *duration > 0) {
        tokio::time::sleep(std::time::Duration::from_millis(duration_ms.min(5_000))).await;
    }
    backend
        .sh(
            "xdotool",
            &[
                "mousemove",
                &(to_x as i32).to_string(),
                &(to_y as i32).to_string(),
            ],
        )
        .await?;
    backend.sh("xdotool", &["mouseup", b]).await.map(|_| ())
}

pub(super) async fn clipboard_read(backend: &X11Backend) -> anyhow::Result<String> {
    backend
        .sh("xclip", &["-o", "-selection", "clipboard"])
        .await
}

pub(super) async fn clipboard_write(backend: &X11Backend, text: &str) -> anyhow::Result<()> {
    backend
        .sh(
            "sh",
            &[
                "-c",
                &format!(
                    "printf %s {} | xclip -selection clipboard",
                    shell_escape(text)
                ),
            ],
        )
        .await
        .map(|_| ())
}
