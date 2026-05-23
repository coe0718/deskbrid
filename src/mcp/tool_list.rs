//! MCP tool definitions — the full list of tools exposed via MCP.
//!
//! Pure serde_json + tokio bridging to Deskbrid's backend and a11y modules.

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
            "close_window",
            "Close a window by its ID.",
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
