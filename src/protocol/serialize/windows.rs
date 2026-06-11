use super::Action;
use serde_json::json;

pub(super) fn serialize_windows(action: &Action, id: &str) -> serde_json::Value {
    match action {
        // Windows
        Action::WindowsList => json!({"type": "windows.list", "id": id}),
        Action::WindowsFocus(window_id) => {
            json!({"type": "windows.focus", "id": id, "window_id": window_id})
        }
        Action::WindowsGet(window_id) => {
            json!({"type": "windows.get", "id": id, "window_id": window_id})
        }
        Action::WindowsClose(window_id) => {
            json!({"type":"windows.close","id":id,"window_id":window_id})
        }
        Action::WindowsMinimize(window_id) => {
            json!({"type":"windows.minimize","id":id,"window_id":window_id})
        }
        Action::WindowsMaximize(window_id) => {
            json!({"type":"windows.maximize","id":id,"window_id":window_id})
        }
        Action::WindowsMoveResize {
            window_id,
            x,
            y,
            width,
            height,
        } => {
            json!({"type":"windows.move_resize","id":id,"window_id":window_id,"x":x,"y":y,"width":width,"height":height})
        }
        Action::WindowsTile {
            window_id,
            preset,
            monitor,
            padding,
        } => {
            let mut obj =
                json!({"type":"windows.tile","id":id,"window_id":window_id,"preset":preset});
            if let Some(monitor) = monitor {
                obj["monitor"] = json!(monitor);
            }
            if let Some(padding) = padding {
                obj["padding"] = json!(padding);
            }
            obj
        }
        Action::WindowsActivateOrLaunch {
            app_id,
            command,
            workdir,
            env,
        } => {
            let mut obj = json!({"type":"windows.activate_or_launch","id":id,"app_id":app_id});
            if !command.is_empty() {
                obj["command"] = json!(command);
            }
            if let Some(wd) = workdir {
                obj["workdir"] = json!(wd);
            }
            if let Some(e) = env {
                obj["env"] = json!(e);
            }
            obj
        }

        // Workspaces
        Action::WorkspacesList => json!({"type": "workspaces.list", "id": id}),
        Action::WorkspaceSwitch(workspace_id) => {
            json!({"type": "workspaces.switch", "id": id, "workspace_id": workspace_id})
        }
        Action::WorkspaceMoveWindow {
            window_id,
            workspace_id,
            follow,
        } => {
            json!({"type": "workspaces.move_window", "id": id, "window_id": window_id, "workspace_id": workspace_id, "follow": follow})
        }

        // Layout profiles
        Action::LayoutProfilesList => json!({"type": "layout_profiles.list", "id": id}),
        Action::LayoutProfileGet { name } => {
            json!({"type": "layout_profiles.get", "id": id, "name": name})
        }
        Action::LayoutProfileSave { name, overwrite } => {
            json!({"type": "layout_profiles.save", "id": id, "name": name, "overwrite": overwrite})
        }
        Action::LayoutProfileDelete { name } => {
            json!({"type": "layout_profiles.delete", "id": id, "name": name})
        }
        Action::LayoutProfileRestore { name } => {
            json!({"type": "layout_profiles.restore", "id": id, "name": name})
        }
        _ => serde_json::json!({"error": "not a windows action"}),
    }
}
