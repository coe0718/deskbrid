use super::Action;
use serde_json::json;

pub(super) fn serialize_blackboard(action: &Action, id: &str) -> serde_json::Value {
    match action {
        Action::BlackboardSet {
            key,
            value,
            namespace,
        } => {
            let mut obj = json!({"type": "blackboard.set", "id": id, "key": key, "value": value});
            if let Some(ns) = namespace {
                obj["namespace"] = serde_json::Value::String(ns.clone());
            }
            obj
        }
        Action::BlackboardGet { key, namespace } => {
            let mut obj = json!({"type": "blackboard.get", "id": id, "key": key});
            if let Some(ns) = namespace {
                obj["namespace"] = serde_json::Value::String(ns.clone());
            }
            obj
        }
        Action::BlackboardDelete { key, namespace } => {
            let mut obj = json!({"type": "blackboard.delete", "id": id, "key": key});
            if let Some(ns) = namespace {
                obj["namespace"] = serde_json::Value::String(ns.clone());
            }
            obj
        }
        Action::BlackboardList { namespace } => {
            let mut obj = json!({"type": "blackboard.list", "id": id});
            if let Some(ns) = namespace {
                obj["namespace"] = serde_json::Value::String(ns.clone());
            }
            obj
        }
        _ => unreachable!("not a blackboard action"),
    }
}
