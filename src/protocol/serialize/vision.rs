use super::Action;
use serde_json::json;

pub(super) fn serialize_vision(action: &Action, id: &str) -> serde_json::Value {
    match action {
        Action::VisionFindElement {
            template_path,
            screenshot,
            min_confidence,
            max_results,
        } => {
            let mut obj = json!({
                "type": "vision.find_element",
                "id": id,
                "template_path": template_path,
            });
            if let Some(s) = screenshot {
                obj["screenshot"] = json!(s);
            }
            if let Some(c) = min_confidence {
                obj["min_confidence"] = json!(c);
            }
            if let Some(m) = max_results {
                obj["max_results"] = json!(m);
            }
            obj
        }
        Action::VisionFindByText { text, screenshot } => {
            let mut obj = json!({
                "type": "vision.find_by_text",
                "id": id,
                "text": text,
            });
            if let Some(s) = screenshot {
                obj["screenshot"] = json!(s);
            }
            obj
        }
        Action::VisionDetectState { screenshot, checks } => {
            let mut obj = json!({
                "type": "vision.detect_state",
                "id": id,
            });
            if let Some(s) = screenshot {
                obj["screenshot"] = json!(s);
            }
            obj["checks"] = json!(checks);
            obj
        }
        _ => json!({}),
    }
}
