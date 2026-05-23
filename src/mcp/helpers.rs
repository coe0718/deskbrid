use crate::DaemonState;
use anyhow::Context;
use serde_json::{Value, json};

// --- Action helpers ---

pub(super) async fn do_execute(
    state: &DaemonState,
    action_type: &str,
    _args: Value,
) -> anyhow::Result<Value> {
    let action =
        crate::protocol::Action::from_json(&format!(r#"{{"type":"{}","id":"mcp"}}"#, action_type))
            .map(|(_, a)| a)?;
    let backend = state.backend.read().await;
    let backend = backend
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no backend loaded"))?;
    crate::daemon::execute::execute_action(action, backend.as_ref(), state).await
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
    let action = crate::protocol::Action::from_json(&serde_json::to_string(&Value::Object(map))?)
        .map(|(_, a)| a)?;
    let backend = state.backend.read().await;
    let backend = backend
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no backend loaded"))?;
    crate::daemon::execute::execute_action(action, backend.as_ref(), state).await
}

pub(super) async fn do_focus_window(state: &DaemonState, window_id: &str) -> anyhow::Result<Value> {
    let action = crate::protocol::Action::WindowsFocus(window_id.to_string());
    let backend = state.backend.read().await;
    let backend = backend
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no backend loaded"))?;
    crate::daemon::execute::execute_action(action, backend.as_ref(), state).await
}

pub(super) async fn do_focused_window(state: &DaemonState) -> anyhow::Result<Value> {
    let action = crate::protocol::Action::WindowsList;
    let backend = state.backend.read().await;
    let backend = backend
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no backend loaded"))?;
    let result = crate::daemon::execute::execute_action(action, backend.as_ref(), state).await?;
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
    let action = crate::protocol::Action::InputKeyboardType {
        text: text.to_string(),
    };
    let backend = state.backend.read().await;
    let backend = backend
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no backend loaded"))?;
    crate::daemon::execute::execute_action(action, backend.as_ref(), state).await
}

pub(super) async fn do_press_keys(state: &DaemonState, keys: &[String]) -> anyhow::Result<Value> {
    let action = crate::protocol::Action::InputKeyboardCombo {
        keys: keys.to_vec(),
    };
    let backend = state.backend.read().await;
    let backend = backend
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no backend loaded"))?;
    crate::daemon::execute::execute_action(action, backend.as_ref(), state).await
}

pub(super) async fn do_mouse_move(state: &DaemonState, x: f64, y: f64) -> anyhow::Result<Value> {
    let action = crate::protocol::Action::InputMouse {
        action: "move".into(),
        x: Some(x),
        y: Some(y),
        button: None,
        dx: None,
        dy: None,
    };
    let backend = state.backend.read().await;
    let backend = backend
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no backend loaded"))?;
    crate::daemon::execute::execute_action(action, backend.as_ref(), state).await
}

pub(super) async fn do_mouse_click(state: &DaemonState, button: &str) -> anyhow::Result<Value> {
    let action = crate::protocol::Action::InputMouse {
        action: "click".into(),
        x: None,
        y: None,
        button: Some(button.to_string()),
        dx: None,
        dy: None,
    };
    let backend = state.backend.read().await;
    let backend = backend
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no backend loaded"))?;
    crate::daemon::execute::execute_action(action, backend.as_ref(), state).await
}

pub(super) async fn do_clipboard_write(state: &DaemonState, text: &str) -> anyhow::Result<Value> {
    let action = crate::protocol::Action::ClipboardWrite {
        text: text.to_string(),
    };
    let backend = state.backend.read().await;
    let backend = backend
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no backend loaded"))?;
    crate::daemon::execute::execute_action(action, backend.as_ref(), state).await
}

pub(super) async fn do_list_apps(_state: &DaemonState) -> anyhow::Result<Value> {
    let conn = crate::a11y::bus::connect_a11y().await?;
    let root = zbus::zvariant::ObjectPath::try_from(crate::a11y::bus::ROOT)?;
    let mut apps = Vec::new();
    for i in 0..50i32 {
        if let Some(ref child) = crate::a11y::bus::child_path(&conn, &root, i).await {
            let info = crate::a11y::bus::element_json(&conn, child).await;
            apps.push(info);
        } else {
            break;
        }
    }
    Ok(json!({ "apps": apps, "count": apps.len() }))
}

pub(super) async fn do_get_accessibility_tree(
    _state: &DaemonState,
    app_name: Option<&str>,
    pid: Option<u32>,
    max_nodes: Option<usize>,
    max_depth: Option<u32>,
) -> anyhow::Result<Value> {
    crate::a11y::tree::snapshot_tree(app_name, pid, max_nodes, max_depth).await
}

pub(super) async fn do_perform_action(
    _state: &DaemonState,
    object_ref: &str,
    action_name: Option<&str>,
) -> anyhow::Result<Value> {
    crate::a11y::actions::perform_action(object_ref, action_name).await
}

pub(super) async fn do_set_element_value(
    _state: &DaemonState,
    object_ref: &str,
    value: &str,
) -> anyhow::Result<Value> {
    crate::a11y::value::set_element_value(object_ref, value).await
}

pub(super) async fn do_get_element_text(
    _state: &DaemonState,
    object_ref: &str,
    max_chars: Option<i32>,
) -> anyhow::Result<Value> {
    crate::a11y::value::get_element_text(object_ref, max_chars).await
}

pub(super) async fn do_click_element(
    _state: &DaemonState,
    object_ref: &str,
) -> anyhow::Result<Value> {
    crate::a11y::actions::click_element(object_ref).await
}

pub(super) async fn do_doctor(_state: &DaemonState) -> anyhow::Result<Value> {
    Ok(crate::a11y::setup::doctor_report().await)
}

pub(super) async fn do_setup_accessibility(_state: &DaemonState) -> anyhow::Result<Value> {
    let enabled = crate::a11y::setup::enable_accessibility().await?;
    Ok(json!({"enabled": enabled}))
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

pub(super) async fn do_click_coordinate(x: f64, y: f64, button: &str) -> anyhow::Result<Value> {
    let btn = crate::abs_pointer::button_code(button)
        .map_err(|e| anyhow::anyhow!(e))?;
    let button = button.to_string();
    tokio::task::spawn_blocking(move || {
        let mut pointer = crate::abs_pointer::create_for_screen_sync()
            .context("uinput not available — is the uinput kernel module loaded?")?;
        pointer.click_at(x, y, btn)?;
        Ok(json!({"clicked": true, "x": x, "y": y, "button": button}))
    })
    .await?
}

pub(super) async fn do_drag(
    from_x: f64,
    from_y: f64,
    to_x: f64,
    to_y: f64,
    button: &str,
) -> anyhow::Result<Value> {
    let btn = crate::abs_pointer::button_code(button)
        .map_err(|e| anyhow::anyhow!(e))?;
    let button = button.to_string();
    tokio::task::spawn_blocking(move || {
        let mut pointer = crate::abs_pointer::create_for_screen_sync()
            .context("uinput not available — is the uinput kernel module loaded?")?;
        pointer.drag(from_x, from_y, to_x, to_y, btn)?;
        Ok(
            json!({"dragged": true, "from": {"x": from_x, "y": from_y}, "to": {"x": to_x, "y": to_y}, "button": button}),
        )
    })
    .await?
}
