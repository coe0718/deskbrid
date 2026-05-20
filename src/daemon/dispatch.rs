use crate::DaemonState;
use crate::protocol::Action;
use tracing::warn;

use super::execute::execute_action;
use super::helpers::{not_supported_response, permission_denied_response};

pub async fn dispatch_action(
    action: Action,
    state: &DaemonState,
    peer_uid: u32,
    seq: u64,
) -> serde_json::Value {
    // Check permissions first
    if !state.permissions.check(peer_uid, &action) {
        return permission_denied_response(seq);
    }
    for implied_action in implied_permission_actions(&action) {
        if !state.permissions.check(peer_uid, &implied_action) {
            return permission_denied_response(seq);
        }
    }
    if let Action::WindowsActivateOrLaunch {
        command,
        workdir,
        env,
        ..
    } = &action
    {
        let process_start = Action::ProcessStart {
            command: command.clone(),
            workdir: workdir.clone(),
            env: env.clone(),
        };
        if !state.permissions.check(peer_uid, &process_start) {
            return permission_denied_response(seq);
        }
    }

    let backend = state.backend.read().await;
    let backend = match backend.as_ref() {
        Some(b) => b,
        None => {
            return not_supported_response(
                "no backend loaded (start daemon in a GNOME 46+ session)",
                seq,
            );
        }
    };

    let result = execute_action(action.clone(), backend.as_ref()).await;

    match result {
        Ok(data) => {
            emit_action_event(state, &action, &data);
            serde_json::json!({
                "type": "response", "id": "action", "seq": seq, "status": "ok", "data": data
            })
        }
        Err(e) => {
            warn!("Action failed: {}", e);
            serde_json::json!({
                "type": "response", "id": "action", "seq": seq, "status": "error",
                "error": { "code": "INTERNAL_ERROR", "message": format!("{}", e) }
            })
        }
    }
}

pub fn implied_permission_actions(action: &Action) -> Vec<Action> {
    match action {
        Action::LayoutProfileSave { .. } => {
            vec![
                Action::WindowsList,
                Action::WorkspacesList,
                Action::SystemInfo,
            ]
        }
        Action::LayoutProfileRestore { .. } => vec![
            Action::WindowsMoveResize {
                window_id: "profile".into(),
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            },
            Action::WindowsMinimize("profile".into()),
            Action::WorkspaceSwitch(0),
            Action::WorkspaceMoveWindow {
                window_id: "profile".into(),
                workspace_id: 0,
                follow: false,
            },
        ],
        _ => Vec::new(),
    }
}

pub fn emit_action_event(state: &DaemonState, action: &Action, data: &serde_json::Value) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let event = match action {
        // Use the resolved window ID from the response data when available,
        // so subscribers get the canonical ID, not the caller-provided selector.
        Action::WindowsFocus(_) => {
            let window_id = data
                .get("focused")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            Some(crate::protocol::DeskbridEvent::WindowFocused {
                window_id,
                timestamp: now,
            })
        }
        Action::WorkspaceSwitch(id) => Some(crate::protocol::DeskbridEvent::WorkspaceChanged {
            workspace_id: *id,
            timestamp: now,
        }),
        Action::WorkspaceMoveWindow {
            window_id,
            workspace_id,
            ..
        } => Some(crate::protocol::DeskbridEvent::WorkspaceWindowMoved {
            window_id: window_id.clone(),
            workspace_id: *workspace_id,
            timestamp: now,
        }),
        _ => None,
    };
    if let Some(evt) = event {
        let _ = state.event_tx.send(evt);
    }
    let _ = data;
}
