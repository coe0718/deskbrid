use super::Action;
use serde_json::json;

pub(super) fn serialize_a11y_location(action: &Action, id: &str) -> serde_json::Value {
    match action {
        // Location
        Action::LocationGet => json!({"type": "location.get", "id": id}),
        Action::UiTreeGet => json!({"type":"ui.tree.get","id":id}),
        Action::UiElementClick {
            selector,
            tab_index,
        } => {
            let mut v = json!({"type":"ui.element.click","id":id,"selector":selector});
            if let Some(ti) = tab_index {
                v["tab_index"] = json!(ti);
            }
            v
        }
        Action::UiElementSetText {
            selector,
            text,
            tab_index,
        } => {
            let mut v =
                json!({"type":"ui.element.set_text","id":id,"selector":selector,"text":text});
            if let Some(ti) = tab_index {
                v["tab_index"] = json!(ti);
            }
            v
        }
        _ => serde_json::json!({"error": "not a a11y_location action"}),
    }
}
