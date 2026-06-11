use super::Action;
use serde_json::json;

pub(super) fn serialize_connection(action: &Action, id: &str) -> serde_json::Value {
    match action {
        Action::Ping => json!({"type": "ping", "id": id}),
        // Connection
        Action::Subscribe { events } => {
            json!({"type": "subscribe", "id": id, "events": events})
        }
        Action::Unsubscribe { events } => {
            json!({"type": "unsubscribe", "id": id, "events": events})
        }
        Action::Disconnect => json!({"type": "disconnect", "id": id}),
        _ => serde_json::json!({"error": "not a connection action"}),
    }
}
