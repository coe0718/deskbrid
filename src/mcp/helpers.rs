use crate::DaemonState;
use crate::protocol::{Action, RequestOptions};
use serde_json::{Value, json};

/// Synthetic UID for MCP clients. UID 0 is reserved by the dispatcher for
/// internal rule/schedule automation, so MCP needs a distinct policy bucket.
pub(crate) const MCP_EFFECTIVE_UID: u32 = 0xFFFF_FFFD;
const MCP_SESSION_ID: &str = "mcp";

// --- Action helpers ---

pub(super) async fn do_execute(
    state: &DaemonState,
    action_type: &str,
    args: Value,
) -> anyhow::Result<Value> {
    do_execute_with(state, action_type, args).await
}

/// Like do_execute but merges params into the action JSON.
pub(super) async fn do_execute_with(
    state: &DaemonState,
    action_type: &str,
    args: Value,
) -> anyhow::Result<Value> {
    let mut map = serde_json::Map::new();
    map.insert("type".into(), action_type.into());
    map.insert("id".into(), "mcp".into());
    if let Value::Object(obj) = args {
        map.extend(obj);
    }
    let action = Action::from_json(&serde_json::to_string(&Value::Object(map))?).map(|(_, a)| a)?;
    dispatch_mcp_action(state, action).await
}

pub(super) async fn dispatch_mcp_action(
    state: &DaemonState,
    action: Action,
) -> anyhow::Result<Value> {
    let request_id = "mcp";
    let seq = crate::daemon::helpers::unix_timestamp();
    let response = crate::daemon::dispatch_action_with_options(
        request_id,
        action,
        state,
        MCP_EFFECTIVE_UID,
        seq,
        RequestOptions::default(),
        MCP_SESSION_ID,
    )
    .await;

    match response.get("status").and_then(|v| v.as_str()) {
        Some("ok") => Ok(response.get("data").cloned().unwrap_or(Value::Null)),
        Some(status) => {
            let message = response
                .get("error")
                .and_then(|error| error.get("message"))
                .and_then(|message| message.as_str())
                .unwrap_or(status);
            anyhow::bail!("{message}");
        }
        None => anyhow::bail!("invalid dispatcher response"),
    }
}

pub(super) async fn do_focus_window(state: &DaemonState, window_id: &str) -> anyhow::Result<Value> {
    dispatch_mcp_action(state, Action::WindowsFocus(window_id.to_string())).await
}

pub(super) async fn do_focused_window(state: &DaemonState) -> anyhow::Result<Value> {
    let result = dispatch_mcp_action(state, Action::WindowsList).await?;
    // Filter for the focused/active window
    if let Some(windows) = result.as_array() {
        for w in windows {
            if w.get("focused").and_then(|v| v.as_bool()).unwrap_or(false)
                || w.get("active").and_then(|v| v.as_bool()).unwrap_or(false)
            {
                return Ok(w.clone());
            }
        }
    }
    Ok(json!({"error": "no focused window found"}))
}

pub(super) async fn do_type_text(state: &DaemonState, text: &str) -> anyhow::Result<Value> {
    dispatch_mcp_action(
        state,
        Action::InputKeyboardType {
            text: text.to_string(),
        },
    )
    .await
}

pub(super) async fn do_press_keys(state: &DaemonState, keys: &[String]) -> anyhow::Result<Value> {
    dispatch_mcp_action(
        state,
        Action::InputKeyboardCombo {
            keys: keys.to_vec(),
        },
    )
    .await
}

pub(super) async fn do_mouse_move(state: &DaemonState, x: f64, y: f64) -> anyhow::Result<Value> {
    dispatch_mcp_action(
        state,
        Action::InputMouse {
            action: "move".into(),
            x: Some(x),
            y: Some(y),
            button: None,
            dx: None,
            dy: None,
        },
    )
    .await
}

pub(super) async fn do_mouse_click(state: &DaemonState, button: &str) -> anyhow::Result<Value> {
    dispatch_mcp_action(
        state,
        Action::InputMouse {
            action: "click".into(),
            x: None,
            y: None,
            button: Some(button.to_string()),
            dx: None,
            dy: None,
        },
    )
    .await
}

pub(super) async fn do_clipboard_write(state: &DaemonState, text: &str) -> anyhow::Result<Value> {
    dispatch_mcp_action(
        state,
        Action::ClipboardWrite {
            text: text.to_string(),
        },
    )
    .await
}

pub(super) async fn do_list_apps(state: &DaemonState) -> anyhow::Result<Value> {
    dispatch_mcp_action(state, Action::A11yListApps { limit: Some(50) }).await
}

pub(super) async fn do_get_accessibility_tree(
    state: &DaemonState,
    app_name: Option<&str>,
    pid: Option<u32>,
    max_nodes: Option<usize>,
    max_depth: Option<u32>,
) -> anyhow::Result<Value> {
    dispatch_mcp_action(
        state,
        Action::A11ySnapshotTree {
            app_name: app_name.map(String::from),
            pid,
            max_nodes,
            max_depth,
        },
    )
    .await
}

pub(super) async fn do_perform_action(
    state: &DaemonState,
    object_ref: &str,
    action_name: Option<&str>,
) -> anyhow::Result<Value> {
    dispatch_mcp_action(
        state,
        Action::A11yPerformAction {
            object_ref: object_ref.to_string(),
            action_name: action_name.map(String::from),
        },
    )
    .await
}

pub(super) async fn do_set_element_value(
    state: &DaemonState,
    object_ref: &str,
    value: &str,
) -> anyhow::Result<Value> {
    dispatch_mcp_action(
        state,
        Action::A11ySetValue {
            object_ref: object_ref.to_string(),
            value: value.to_string(),
        },
    )
    .await
}

pub(super) async fn do_get_element_text(
    state: &DaemonState,
    object_ref: &str,
    max_chars: Option<i32>,
) -> anyhow::Result<Value> {
    dispatch_mcp_action(
        state,
        Action::A11yGetElementText {
            object_ref: object_ref.to_string(),
            max_chars,
        },
    )
    .await
}

pub(super) async fn do_click_element(
    state: &DaemonState,
    object_ref: &str,
) -> anyhow::Result<Value> {
    dispatch_mcp_action(
        state,
        Action::A11yClickElementByRef {
            object_ref: object_ref.to_string(),
        },
    )
    .await
}

pub(super) async fn do_doctor(state: &DaemonState) -> anyhow::Result<Value> {
    dispatch_mcp_action(state, Action::A11yDoctor).await
}

pub(super) async fn do_setup_accessibility(state: &DaemonState) -> anyhow::Result<Value> {
    dispatch_mcp_action(state, Action::A11ySetupAccessibility).await
}

pub(super) async fn do_capabilities(state: &DaemonState) -> anyhow::Result<Value> {
    let backend = state.backend.read().await;
    let has_backend = backend.is_some();
    Ok(json!({
        "backend_loaded": has_backend,
        "tools": crate::protocol::Action::public_action_types(),
        "mcp_enabled": true,
    }))
}

// --- Absolute Pointer tools ---

pub(super) async fn do_click_coordinate(
    state: &DaemonState,
    x: f64,
    y: f64,
    button: &str,
) -> anyhow::Result<Value> {
    dispatch_mcp_action(
        state,
        Action::InputMouse {
            action: "move".into(),
            x: Some(x),
            y: Some(y),
            button: None,
            dx: None,
            dy: None,
        },
    )
    .await?;
    dispatch_mcp_action(
        state,
        Action::InputMouse {
            action: "click".into(),
            x: None,
            y: None,
            button: Some(button.to_string()),
            dx: None,
            dy: None,
        },
    )
    .await?;
    Ok(json!({"clicked": true, "x": x, "y": y, "button": button}))
}

pub(super) async fn do_drag(
    state: &DaemonState,
    from_x: f64,
    from_y: f64,
    to_x: f64,
    to_y: f64,
    button: &str,
) -> anyhow::Result<Value> {
    dispatch_mcp_action(
        state,
        Action::InputMouseDrag {
            from_x,
            from_y,
            to_x,
            to_y,
            button: Some(button.to_string()),
            duration_ms: None,
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::Permissions;

    #[tokio::test]
    async fn mcp_dispatch_honors_permissions_before_backend() {
        let mut state = DaemonState::new();
        state.permissions = Permissions::deny_all();

        let err = dispatch_mcp_action(&state, Action::WindowsList)
            .await
            .unwrap_err()
            .to_string();

        assert!(err.contains("action not permitted: windows.list"));
    }

    #[tokio::test]
    async fn mcp_execute_merges_args_before_dispatch() {
        let mut state = DaemonState::new();
        state.permissions = Permissions::allow_all();

        let err = do_execute_with(
            &state,
            "windows.close",
            serde_json::json!({"window_id": "0x1"}),
        )
        .await
        .unwrap_err()
        .to_string();

        assert!(err.contains("no desktop backend loaded"));
    }
}
