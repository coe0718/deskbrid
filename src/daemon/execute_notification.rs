use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::Action;
use serde_json::Value;

pub(crate) async fn execute_notification(
    action: Action,
    backend: &dyn DesktopBackend,
    _state: &DaemonState,
) -> anyhow::Result<Value> {
    use Action::*;
    Ok(match action {
        NotificationSend {
            ref app_name,
            ref title,
            ref body,
            ref urgency,
        } => {
            let id = backend
                .notification_send(app_name, title, body, urgency)
                .await?;
            serde_json::json!({"notification_id": id})
        }
        NotificationClose { notification_id } => {
            backend.notification_close(notification_id).await?;
            serde_json::json!({"closed": notification_id})
        }

        _ => unreachable!("not a notification action"),
    })
}
