use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::Action;
use serde_json::Value;

use super::{find_app_window, spawn_detached_process};

pub(crate) async fn execute_windows(
    action: Action,
    backend: &dyn DesktopBackend,
    _state: &DaemonState,
) -> anyhow::Result<Value> {
    use Action::*;
    Ok(match action {
        WindowsList => serde_json::json!(backend.windows_list().await?),
        WindowsFocus(ref id) => {
            backend.window_focus(id).await?;
            // Try to get the resolved window so events publish the canonical ID,
            // not the caller-provided selector. Falls back to the raw selector.
            let resolved = backend
                .window_get(id)
                .await
                .map(|w| w.id)
                .unwrap_or_else(|_| id.clone());
            serde_json::json!({"focused": resolved, "id": id})
        }
        WindowsGet(ref id) => serde_json::json!(backend.window_get(id).await?),
        WindowsClose(ref id) => {
            backend.window_close(id).await?;
            serde_json::json!({"closed": id})
        }
        WindowsMinimize(ref id) => {
            backend.window_minimize(id).await?;
            serde_json::json!({"minimized": id})
        }
        WindowsMaximize(ref id) => {
            backend.window_maximize(id).await?;
            serde_json::json!({"maximized": id})
        }
        WindowsMoveResize {
            ref window_id,
            x,
            y,
            width,
            height,
        } => {
            backend
                .window_move_resize(window_id, x, y, width, height)
                .await?;
            serde_json::json!({
                "window_id": window_id, "x": x, "y": y, "width": width, "height": height
            })
        }
        WindowsTile {
            ref window_id,
            ref preset,
            monitor,
            padding,
        } => {
            crate::tiling::tile_window(backend, window_id, preset, monitor, padding.unwrap_or(0))
                .await?
        }
        WindowsActivateOrLaunch {
            ref app_id,
            ref command,
            ref workdir,
            ref env,
        } => {
            if let Some(window) = find_app_window(backend, app_id).await? {
                backend.window_focus(&window.id).await?;
                serde_json::json!({
                    "app_id": app_id,
                    "activated": true,
                    "launched": false,
                    "window_id": window.id
                })
            } else {
                let launch_command = if command.is_empty() {
                    vec![app_id.clone()]
                } else {
                    command.clone()
                };
                let pid = spawn_detached_process(&launch_command, workdir.as_deref(), env.as_ref())
                    .await?;
                serde_json::json!({
                    "app_id": app_id,
                    "activated": false,
                    "launched": true,
                    "pid": pid,
                    "command": launch_command
                })
            }
        }

        _ => unreachable!("not a windows action"),
    })
}
