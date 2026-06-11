use super::Action;
use serde_json::json;

pub(super) fn serialize_desktop(action: &Action, id: &str) -> serde_json::Value {
    match action {
        Action::DesktopGetSetting { schema, key } => {
            json!({"type": "desktop.get_setting", "id": id, "schema": schema, "key": key})
        }
        Action::DesktopSetSetting { schema, key, value } => {
            json!({"type": "desktop.set_setting", "id": id, "schema": schema, "key": key, "value": value})
        }
        Action::DesktopListSchemas => {
            json!({"type": "desktop.list_schemas", "id": id})
        }
        _ => serde_json::json!({"error": "not a desktop settings action"}),
    }
}
