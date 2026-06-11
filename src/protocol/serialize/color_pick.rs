use super::Action;
use serde_json::json;

pub(super) fn serialize_color_pick(action: &Action, id: &str) -> serde_json::Value {
    match action {
        // Color picker
        Action::ColorPick { x, y, path } => {
            let mut obj = json!({"type": "color.pick", "id": id, "x": x, "y": y});
            if let Some(path) = path {
                obj["path"] = json!(path);
            }
            obj
        }
        _ => serde_json::json!({"error": "not a color_pick action"}),
    }
}
