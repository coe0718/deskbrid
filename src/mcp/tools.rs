//! MCP tool implementations — no external MCP crate dependencies.
//! Pure serde_json + tokio bridging to Deskbrid's backend and a11y modules.

use crate::DaemonState;
use anyhow::Context;
use serde_json::{Value, json};

pub fn list_tools() -> Vec<Value> {
    vec![
        // Window control
        tool(
            "list_windows",
            "List all open windows.",
            json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        ),
        tool(
            "focus_window",
            "Focus a window by its ID.",
            json!({
                "type": "object",
                "properties": {
                    "window_id": {"type": "string", "description": "Window ID from list_windows"}
                },
                "required": ["window_id"]
            }),
        ),
        tool(
            "type_text",
            "Type a string via keyboard input.",
            json!({
                "type": "object",
                "properties": {
                    "text": {"type": "string", "description": "Text to type"}
                },
                "required": ["text"]
            }),
        ),
        tool(
            "press_keys",
            "Press a key combination.",
            json!({
                "type": "object",
                "properties": {
                    "keys": {"type": "array", "items": {"type": "string"}, "description": "Keys to press, e.g. Control_L+c"}
                },
                "required": ["keys"]
            }),
        ),
        tool(
            "mouse_move",
            "Move the mouse cursor to absolute coordinates.",
            json!({
                "type": "object",
                "properties": {
                    "x": {"type": "number", "description": "X coordinate"},
                    "y": {"type": "number", "description": "Y coordinate"}
                },
                "required": ["x", "y"]
            }),
        ),
        tool(
            "mouse_click",
            "Click a mouse button.",
            json!({
                "type": "object",
                "properties": {
                    "button": {"type": "string", "description": "Button: 'left', 'middle', or 'right'"}
                },
                "required": ["button"]
            }),
        ),
        tool(
            "screenshot",
            "Take a screenshot.",
            json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        ),
        tool(
            "clipboard_read",
            "Read clipboard contents.",
            json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        ),
        tool(
            "clipboard_write",
            "Write text to clipboard.",
            json!({
                "type": "object",
                "properties": {
                    "text": {"type": "string", "description": "Text to copy"}
                },
                "required": ["text"]
            }),
        ),
        // AT-SPI tools
        tool(
            "list_apps",
            "List AT-SPI application roots.",
            json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        ),
        tool(
            "get_accessibility_tree",
            "Get the AT-SPI accessibility tree for an app.",
            json!({
                "type": "object",
                "properties": {
                    "app_name": {"type": "string", "description": "Filter by app name"},
                    "pid": {"type": "integer", "description": "Filter by process ID"},
                    "max_nodes": {"type": "integer", "description": "Maximum nodes (default: 200)"},
                    "max_depth": {"type": "integer", "description": "Maximum depth (default: 10)"}
                },
                "required": []
            }),
        ),
        tool(
            "perform_action",
            "Perform an AT-SPI action on an element.",
            json!({
                "type": "object",
                "properties": {
                    "object_ref": {"type": "string", "description": "AT-SPI object reference path"},
                    "action_name": {"type": "string", "description": "Action name (e.g. 'click', 'activate')"}
                },
                "required": ["object_ref"]
            }),
        ),
        tool(
            "set_element_value",
            "Set the value of an AT-SPI element.",
            json!({
                "type": "object",
                "properties": {
                    "object_ref": {"type": "string", "description": "AT-SPI object reference path"},
                    "value": {"type": "string", "description": "Value to set"}
                },
                "required": ["object_ref", "value"]
            }),
        ),
        tool(
            "get_element_text",
            "Get text content from an AT-SPI element.",
            json!({
                "type": "object",
                "properties": {
                    "object_ref": {"type": "string", "description": "AT-SPI object reference path"},
                    "max_chars": {"type": "integer", "description": "Maximum characters to return"}
                },
                "required": ["object_ref"]
            }),
        ),
        tool(
            "click_element",
            "Click an AT-SPI element with coordinate fallback.",
            json!({
                "type": "object",
                "properties": {
                    "object_ref": {"type": "string", "description": "AT-SPI object reference path"}
                },
                "required": ["object_ref"]
            }),
        ),
        tool(
            "doctor",
            "Run AT-SPI accessibility diagnostics.",
            json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        ),
        tool(
            "setup_accessibility",
            "Enable AT-SPI accessibility via gsettings.",
            json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        ),
        tool(
            "capabilities",
            "List available Deskbrid capabilities.",
            json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        ),
    ]
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}

pub async fn call_tool(state: &DaemonState, name: &str, args: &Value) -> anyhow::Result<String> {
    let result = match name {
        "list_windows" => do_execute(state, "windows.list", json!({})).await?,
        "focus_window" => {
            let id = args["window_id"].as_str().unwrap_or("");
            do_focus_window(state, id).await?
        }
        "type_text" => {
            let text = args["text"].as_str().unwrap_or("");
            do_type_text(state, text).await?
        }
        "press_keys" => {
            let keys: Vec<String> = args["keys"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            do_press_keys(state, &keys).await?
        }
        "mouse_move" => {
            let x = args["x"].as_f64().unwrap_or(0.0);
            let y = args["y"].as_f64().unwrap_or(0.0);
            do_mouse_move(state, x, y).await?
        }
        "mouse_click" => {
            let button = args["button"].as_str().unwrap_or("left");
            do_mouse_click(state, button).await?
        }
        "screenshot" => do_execute(state, "screenshot", json!({})).await?,
        "clipboard_read" => do_execute(state, "clipboard.read", json!({})).await?,
        "clipboard_write" => {
            let text = args["text"].as_str().unwrap_or("");
            do_clipboard_write(state, text).await?
        }
        "list_apps" => do_list_apps(state).await?,
        "get_accessibility_tree" => {
            let app_name = args["app_name"].as_str();
            let pid = args["pid"].as_u64().map(|v| v as u32);
            let max_nodes = args["max_nodes"].as_u64().map(|v| v as usize);
            let max_depth = args["max_depth"].as_u64().map(|v| v as u32);
            do_get_accessibility_tree(state, app_name, pid, max_nodes, max_depth).await?
        }
        "perform_action" => {
            let object_ref = args["object_ref"].as_str().unwrap_or("");
            let action_name = args["action_name"].as_str();
            do_perform_action(state, object_ref, action_name).await?
        }
        "set_element_value" => {
            let object_ref = args["object_ref"].as_str().unwrap_or("");
            let value = args["value"].as_str().unwrap_or("");
            do_set_element_value(state, object_ref, value).await?
        }
        "get_element_text" => {
            let object_ref = args["object_ref"].as_str().unwrap_or("");
            let max_chars = args["max_chars"].as_i64().map(|v| v as i32);
            do_get_element_text(state, object_ref, max_chars).await?
        }
        "click_element" => {
            let object_ref = args["object_ref"].as_str().unwrap_or("");
            do_click_element(state, object_ref).await?
        }
        "doctor" => do_doctor(state).await?,
        "setup_accessibility" => do_setup_accessibility(state).await?,
        "capabilities" => do_capabilities(state).await?,
        "click_coordinate" => {
            let x = args["x"].as_f64().unwrap_or(0.0);
            let y = args["y"].as_f64().unwrap_or(0.0);
            let button = args["button"].as_str().unwrap_or("left");
            do_click_coordinate(x, y, button)?
        }
        "drag" => {
            let from_x = args["from_x"].as_f64().unwrap_or(0.0);
            let from_y = args["from_y"].as_f64().unwrap_or(0.0);
            let to_x = args["to_x"].as_f64().unwrap_or(0.0);
            let to_y = args["to_y"].as_f64().unwrap_or(0.0);
            let button = args["button"].as_str().unwrap_or("left");
            do_drag(from_x, from_y, to_x, to_y, button)?
        }
        _ => anyhow::bail!("unknown tool: {name}"),
    };
    Ok(serde_json::to_string(&result)?)
}

// --- Action helpers ---

async fn do_execute(state: &DaemonState, action_type: &str, _args: Value) -> anyhow::Result<Value> {
    let action =
        crate::protocol::Action::from_json(&format!(r#"{{"type":"{}","id":"mcp"}}"#, action_type))
            .map(|(_, a)| a)?;
    let backend = state.backend.read().await;
    let backend = backend
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no backend loaded"))?;
    crate::daemon::execute::execute_action(action, backend.as_ref(), state).await
}

async fn do_focus_window(state: &DaemonState, window_id: &str) -> anyhow::Result<Value> {
    let action = crate::protocol::Action::WindowsFocus(window_id.to_string());
    let backend = state.backend.read().await;
    let backend = backend
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no backend loaded"))?;
    crate::daemon::execute::execute_action(action, backend.as_ref(), state).await
}

async fn do_type_text(state: &DaemonState, text: &str) -> anyhow::Result<Value> {
    let action = crate::protocol::Action::InputKeyboardType {
        text: text.to_string(),
    };
    let backend = state.backend.read().await;
    let backend = backend
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no backend loaded"))?;
    crate::daemon::execute::execute_action(action, backend.as_ref(), state).await
}

async fn do_press_keys(state: &DaemonState, keys: &[String]) -> anyhow::Result<Value> {
    let action = crate::protocol::Action::InputKeyboardCombo {
        keys: keys.to_vec(),
    };
    let backend = state.backend.read().await;
    let backend = backend
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no backend loaded"))?;
    crate::daemon::execute::execute_action(action, backend.as_ref(), state).await
}

async fn do_mouse_move(state: &DaemonState, x: f64, y: f64) -> anyhow::Result<Value> {
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

async fn do_mouse_click(state: &DaemonState, button: &str) -> anyhow::Result<Value> {
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

async fn do_clipboard_write(state: &DaemonState, text: &str) -> anyhow::Result<Value> {
    let action = crate::protocol::Action::ClipboardWrite {
        text: text.to_string(),
    };
    let backend = state.backend.read().await;
    let backend = backend
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no backend loaded"))?;
    crate::daemon::execute::execute_action(action, backend.as_ref(), state).await
}

async fn do_list_apps(_state: &DaemonState) -> anyhow::Result<Value> {
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

async fn do_get_accessibility_tree(
    _state: &DaemonState,
    app_name: Option<&str>,
    pid: Option<u32>,
    max_nodes: Option<usize>,
    max_depth: Option<u32>,
) -> anyhow::Result<Value> {
    crate::a11y::tree::snapshot_tree(app_name, pid, max_nodes, max_depth).await
}

async fn do_perform_action(
    _state: &DaemonState,
    object_ref: &str,
    action_name: Option<&str>,
) -> anyhow::Result<Value> {
    crate::a11y::actions::perform_action(object_ref, action_name).await
}

async fn do_set_element_value(
    _state: &DaemonState,
    object_ref: &str,
    value: &str,
) -> anyhow::Result<Value> {
    crate::a11y::value::set_element_value(object_ref, value).await
}

async fn do_get_element_text(
    _state: &DaemonState,
    object_ref: &str,
    max_chars: Option<i32>,
) -> anyhow::Result<Value> {
    crate::a11y::value::get_element_text(object_ref, max_chars).await
}

async fn do_click_element(_state: &DaemonState, object_ref: &str) -> anyhow::Result<Value> {
    crate::a11y::actions::click_element(object_ref).await
}

async fn do_doctor(_state: &DaemonState) -> anyhow::Result<Value> {
    Ok(crate::a11y::setup::doctor_report().await)
}

async fn do_setup_accessibility(_state: &DaemonState) -> anyhow::Result<Value> {
    let enabled = crate::a11y::setup::enable_accessibility().await?;
    Ok(json!({"enabled": enabled}))
}

async fn do_capabilities(state: &DaemonState) -> anyhow::Result<Value> {
    let backend = state.backend.read().await;
    let has_backend = backend.is_some();
    Ok(json!({
        "backend_loaded": has_backend,
        "tools": crate::protocol::Action::public_action_types(),
        "mcp_enabled": true,
    }))
}

// --- Absolute Pointer tools ---

fn do_click_coordinate(x: f64, y: f64, button: &str) -> anyhow::Result<Value> {
    let mut pointer = crate::abs_pointer::create_for_screen()
        .context("uinput not available — is the uinput kernel module loaded?")?;
    let btn = crate::abs_pointer::button_code(button);
    pointer.click_at(x, y, btn)?;
    Ok(json!({"clicked": true, "x": x, "y": y, "button": button}))
}

fn do_drag(from_x: f64, from_y: f64, to_x: f64, to_y: f64, button: &str) -> anyhow::Result<Value> {
    let mut pointer = crate::abs_pointer::create_for_screen()
        .context("uinput not available — is the uinput kernel module loaded?")?;
    let btn = crate::abs_pointer::button_code(button);
    pointer.drag(from_x, from_y, to_x, to_y, btn)?;
    Ok(
        json!({"dragged": true, "from": {"x": from_x, "y": from_y}, "to": {"x": to_x, "y": to_y}, "button": button}),
    )
}
