use crate::protocol::Action;
use serde_json::Value;

pub(super) fn parse_notifications(
    raw: &Value,
    _id: &str,
    type_str: &str,
) -> anyhow::Result<Action> {
    Ok(match type_str {
        // Notifications
        "notification.send" => Action::NotificationSend {
            app_name: raw["app_name"].as_str().unwrap_or("deskbrid").into(),
            title: raw["title"].as_str().unwrap_or("").into(),
            body: raw["body"].as_str().unwrap_or("").into(),
            urgency: raw["urgency"].as_str().unwrap_or("normal").into(),
        },
        "notification.close" => Action::NotificationClose {
            notification_id: raw["notification_id"].as_u64().unwrap_or(0) as u32,
        },
        "notification.history" => Action::NotificationHistory {
            limit: raw["limit"].as_u64().map(|v| v as u32),
            app_name: raw["app_name"].as_str().map(String::from),
            since: raw["since"].as_u64(),
        },
        "notification.action" => Action::NotificationAction {
            notification_id: raw["notification_id"].as_u64().unwrap_or(0) as u32,
            action_key: raw["action_key"].as_str().unwrap_or("").into(),
        },
        "notification.clear_history" => Action::NotificationClearHistory,
        "notification.watch" => Action::NotificationWatch,
        _ => anyhow::bail!("unknown notifications type: {type_str}"),
    })
}
