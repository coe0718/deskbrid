use super::Action;
use serde_json::json;

pub(super) fn serialize_input(action: &Action, id: &str) -> serde_json::Value {
    match action {
        // Input
        Action::InputKeyboardType { text } => {
            json!({"type": "input.keyboard", "id": id, "action": "type", "text": text})
        }
        Action::InputKeyboardKey { key } => {
            json!({"type": "input.keyboard", "id": id, "action": "key", "key": key})
        }
        Action::InputKeyboardCombo { keys } => {
            json!({"type": "input.keyboard", "id": id, "action": "combo", "keys": keys})
        }
        Action::InputMouse {
            action,
            x,
            y,
            button,
            dx,
            dy,
        } => {
            let mut obj = json!({"type": "input.mouse", "id": id, "action": action});
            if let Some(x) = x {
                obj["x"] = json!(x);
            }
            if let Some(y) = y {
                obj["y"] = json!(y);
            }
            if let Some(button) = button {
                obj["button"] = json!(button);
            }
            if let Some(dx) = dx {
                obj["dx"] = json!(dx);
            }
            if let Some(dy) = dy {
                obj["dy"] = json!(dy);
            }
            obj
        }
        Action::InputMouseDrag {
            from_x,
            from_y,
            to_x,
            to_y,
            button,
            duration_ms,
        } => {
            let mut obj = json!({
                "type": "input.mouse.drag",
                "id": id,
                "from_x": from_x,
                "from_y": from_y,
                "to_x": to_x,
                "to_y": to_y
            });
            if let Some(button) = button {
                obj["button"] = json!(button);
            }
            if let Some(duration_ms) = duration_ms {
                obj["duration_ms"] = json!(duration_ms);
            }
            obj
        }

        // Clipboard
        Action::ClipboardRead => json!({"type": "clipboard.read", "id": id}),
        Action::ClipboardWrite { text } => {
            json!({"type": "clipboard.write", "id": id, "text": text})
        }
        Action::ClipboardHistoryList { limit, query } => {
            let mut obj = json!({"type": "clipboard.history", "id": id});
            if let Some(limit) = limit {
                obj["limit"] = json!(limit);
            }
            if let Some(query) = query {
                obj["query"] = json!(query);
            }
            obj
        }
        Action::ClipboardHistoryClear => json!({"type": "clipboard.history.clear", "id": id}),
        _ => serde_json::json!({"error": "not a input action"}),
    }
}
