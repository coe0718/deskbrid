use super::Action;
use serde_json::json;

pub(super) fn serialize_audit(action: &Action, id: &str) -> serde_json::Value {
    match action {
        // Audit
        Action::AuditLog {
            limit,
            action_type,
            status,
        } => {
            let mut obj = json!({"type": "audit.log", "id": id});
            if let Some(limit) = limit {
                obj["limit"] = json!(limit);
            }
            if let Some(action_type) = action_type {
                obj["action_type"] = json!(action_type);
            }
            if let Some(status) = status {
                obj["status"] = json!(status);
            }
            obj
        }
        Action::AuditClear => json!({"type": "audit.clear", "id": id}),

        // Notifications
        Action::NotificationSend {
            app_name,
            title,
            body,
            urgency,
        } => {
            json!({"type": "notification.send", "id": id, "app_name": app_name, "title": title, "body": body, "urgency": urgency})
        }
        Action::NotificationClose { notification_id } => {
            json!({"type": "notification.close", "id": id, "notification_id": notification_id})
        }
        Action::NotificationHistory {
            limit,
            app_name,
            since,
        } => {
            let mut obj = json!({"type": "notification.history", "id": id});
            if let Some(limit) = limit {
                obj["limit"] = json!(limit);
            }
            if let Some(app_name) = app_name {
                obj["app_name"] = json!(app_name);
            }
            if let Some(since) = since {
                obj["since"] = json!(since);
            }
            obj
        }
        Action::NotificationAction {
            notification_id,
            action_key,
        } => {
            json!({"type": "notification.action", "id": id, "notification_id": notification_id, "action_key": action_key})
        }
        Action::NotificationClearHistory => json!({"type": "notification.clear_history", "id": id}),
        Action::NotificationWatch => json!({"type": "notification.watch", "id": id}),
        _ => unreachable!("not a audit action"),
    }
}
