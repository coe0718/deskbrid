//! MCP tool implementations — no external MCP crate dependencies.

use crate::DaemonState;
use crate::mcp::helpers::*;
use serde_json::{Value, json};

pub use super::tool_list::list_tools;

pub async fn call_tool(state: &DaemonState, name: &str, args: &Value) -> anyhow::Result<String> {
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        anyhow::bail!("invalid tool name: '{name}'");
    }
    let result = match name {
        "list_windows" => do_execute(state, "windows.list", json!({})).await?,
        "focus_window" => {
            let id = args["window_id"].as_str().unwrap_or("");
            do_focus_window(state, id).await?
        }
        "close_window" => {
            let id = args["window_id"].as_str().unwrap_or("");
            do_execute(state, "windows.close", json!({"window_id": id})).await?
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
            do_click_coordinate(x, y, button).await?
        }
        "drag" => {
            let from_x = args["from_x"].as_f64().unwrap_or(0.0);
            let from_y = args["from_y"].as_f64().unwrap_or(0.0);
            let to_x = args["to_x"].as_f64().unwrap_or(0.0);
            let to_y = args["to_y"].as_f64().unwrap_or(0.0);
            let button = args["button"].as_str().unwrap_or("left");
            do_drag(from_x, from_y, to_x, to_y, button).await?
        }
        _ => {
            // Generic fallback: pass the tool name as action type and args directly.
            // Covers all tools added in Phase 3 without per-tool boilerplate.
            do_execute_with(state, name, args.clone()).await?
        }
    };
    Ok(serde_json::to_string(&result)?)
}
