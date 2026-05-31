use super::t;
use serde_json::{Value, json};

pub fn tools() -> Vec<Value> {
    vec![
        // ══════ Input ══════
        t(
            "type_text",
            "Type a string via keyboard input.",
            json!({"type":"object","properties":{"text":{"type":"string","description":"Text to type"}},"required":["text"]}),
        ),
        t(
            "press_key",
            "Press a single key (e.g. Return, Escape, Tab).",
            json!({"type":"object","properties":{"key":{"type":"string","description":"Single key name"}},"required":["key"]}),
        ),
        t(
            "press_keys",
            "Press a key combination.",
            json!({"type":"object","properties":{"keys":{"type":"array","items":{"type":"string"},"description":"Keys to press"}},"required":["keys"]}),
        ),
        t(
            "mouse_move",
            "Move the mouse cursor to absolute coordinates.",
            json!({"type":"object","properties":{"x":{"type":"number","description":"X coordinate"},"y":{"type":"number","description":"Y coordinate"}},"required":["x","y"]}),
        ),
        t(
            "mouse_click",
            "Click a mouse button at the current position.",
            json!({"type":"object","properties":{"button":{"type":"string","description":"Button: left, middle, or right"}},"required":[]}),
        ),
        t(
            "mouse_scroll",
            "Scroll the mouse wheel.",
            json!({"type":"object","properties":{"dx":{"type":"number","description":"Horizontal scroll"},"dy":{"type":"number","description":"Vertical scroll (negative = down)"}},"required":[]}),
        ),
        t(
            "click_coordinate",
            "Move to pixel coordinates and click.",
            json!({"type":"object","properties":{"x":{"type":"number","description":"X coordinate"},"y":{"type":"number","description":"Y coordinate"},"button":{"type":"string","description":"Button"}},"required":["x","y"]}),
        ),
        t(
            "drag",
            "Click-and-drag between two pixel coordinates.",
            json!({"type":"object","properties":{"from_x":{"type":"number","description":"Start X"},"from_y":{"type":"number","description":"Start Y"},"to_x":{"type":"number","description":"End X"},"to_y":{"type":"number","description":"End Y"},"button":{"type":"string","description":"Button"}},"required":["from_x","from_y","to_x","to_y"]}),
        ),
        // ══════ Clipboard ══════
        t(
            "clipboard_read",
            "Read the current clipboard contents.",
            json!({"type":"object","properties":{},"required":[]}),
        ),
        t(
            "clipboard_write",
            "Write text to the system clipboard.",
            json!({"type":"object","properties":{"text":{"type":"string","description":"Text to copy"}},"required":["text"]}),
        ),
    ]
}
