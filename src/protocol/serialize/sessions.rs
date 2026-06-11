use super::Action;
use serde_json::json;

pub(super) fn serialize_sessions(action: &Action, id: &str) -> serde_json::Value {
    match action {
        Action::SessionCreate { name, clone_from } => {
            let mut obj = json!({"type": "session.create", "id": id, "name": name});
            if let Some(cf) = clone_from {
                obj["clone_from"] = serde_json::Value::String(cf.clone());
            }
            obj
        }
        Action::SessionDestroy { name } => {
            json!({"type": "session.destroy", "id": id, "name": name})
        }
        Action::SessionList => json!({"type": "session.list", "id": id}),
        Action::SessionSwitch { name } => {
            json!({"type": "session.switch", "id": id, "name": name})
        }
        Action::SessionVarSet { name, value } => {
            json!({"type": "session.var.set", "id": id, "name": name, "value": value})
        }
        Action::SessionVarGet { name } => {
            json!({"type": "session.var.get", "id": id, "name": name})
        }
        Action::SessionVarList => json!({"type": "session.var.list", "id": id}),
        _ => serde_json::json!({"error": "not a session action"}),
    }
}
