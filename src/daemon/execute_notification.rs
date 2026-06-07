use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::{Action, DeskbridEvent};
use serde_json::Value;

/// Whether a notification watcher (D-Bus signal listener) has been started.
pub(crate) static NOTIFICATION_WATCH_ACTIVE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Start the D-Bus notification signal watcher if not already running.
/// Runs as a background task that polls org.freedesktop.Notifications.
pub(crate) fn ensure_notification_watcher(event_tx: tokio::sync::broadcast::Sender<DeskbridEvent>) {
    if NOTIFICATION_WATCH_ACTIVE.swap(true, std::sync::atomic::Ordering::Relaxed) {
        return; // already running
    }

    tokio::spawn(async move {
        tracing::debug!("notification watcher started (D-Bus interception)");
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

            // Attempt to poll D-Bus for notification events by monitoring
            // org.freedesktop.Notifications signals
            let output = tokio::process::Command::new("dbus-monitor")
                .arg("--session")
                .arg("--monitor")
                .arg("type='signal',interface='org.freedesktop.Notifications'")
                .output()
                .await;

            if let Ok(o) = output
                && o.status.success()
            {
                let stdout = String::from_utf8_lossy(&o.stdout);
                for line in stdout.lines() {
                    if line.contains("NotificationClosed") {
                        // Extract notification ID
                        if let Some(pos) = line.find("uint32 ") {
                            let id_str = &line[pos + 7..];
                            if let Some(end) = id_str.find(|c: char| !c.is_ascii_digit())
                                && let Ok(nid) = id_str[..end].parse::<u32>()
                            {
                                tracing::debug!("notification closed via D-Bus: {}", nid);
                                let _ = event_tx.send(DeskbridEvent::NotificationReceived {
                                    id: nid,
                                    app_name: "dbus".into(),
                                    title: "closed".into(),
                                    body: None,
                                    urgency: "normal".into(),
                                    actions: None,
                                    timestamp: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs(),
                                });
                            }
                        }
                    }
                    if line.contains("ActionInvoked") {
                        // Extract notification ID and action key
                        if let Some(pos) = line.find("uint32 ") {
                            let rest = &line[pos + 7..];
                            let parts: Vec<&str> = rest.split_whitespace().collect();
                            if parts.len() >= 2
                                && let Ok(nid) = parts[0].parse::<u32>()
                            {
                                let action_key = parts[1].trim_matches('"').to_string();
                                let timestamp = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs();
                                let _ = event_tx.send(DeskbridEvent::NotificationActed {
                                    id: nid,
                                    action_key,
                                    timestamp,
                                });
                            }
                        }
                    }
                }
            }
        }
    });
}

pub(crate) async fn execute_notification(
    action: Action,
    backend: &dyn DesktopBackend,
    state: &DaemonState,
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

            // Record to history DB
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let db = state.database.lock().unwrap();
            let db_id = db
                .insert_notification(
                    app_name,
                    title,
                    Some(body.as_str()),
                    Some(urgency.as_str()),
                    None::<&[String]>,
                    timestamp,
                )
                .unwrap_or(-1);
            drop(db);

            // Broadcast notification event to subscribers
            let _ = state.event_tx.send(DeskbridEvent::NotificationReceived {
                id,
                app_name: app_name.clone(),
                title: title.clone(),
                body: if body.is_empty() {
                    None
                } else {
                    Some(body.clone())
                },
                urgency: urgency.clone(),
                actions: None,
                timestamp,
            });

            serde_json::json!({"notification_id": id, "db_id": db_id})
        }
        NotificationClose { notification_id } => {
            backend.notification_close(notification_id).await?;
            serde_json::json!({"closed": notification_id})
        }
        NotificationHistory {
            ref limit,
            ref app_name,
            ref since,
        } => {
            let db = state.database.lock().unwrap();
            let limit = limit.unwrap_or(50) as usize;
            let notifications = db.get_notifications(limit, app_name.as_deref(), *since)?;
            serde_json::json!({ "notifications": notifications })
        }
        NotificationAction {
            notification_id,
            ref action_key,
        } => {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            // Broadcast the action event
            let _ = state.event_tx.send(DeskbridEvent::NotificationActed {
                id: notification_id,
                action_key: action_key.clone(),
                timestamp,
            });

            serde_json::json!({
                "notification_id": notification_id,
                "action_key": action_key,
                "timestamp": timestamp
            })
        }
        NotificationClearHistory => {
            let db = state.database.lock().unwrap();
            db.clear_notifications()?;
            serde_json::json!({"cleared": true})
        }
        NotificationWatch => {
            // Start the notification watcher using the event broadcast channel
            ensure_notification_watcher(state.event_tx.clone());
            serde_json::json!({"watching": true})
        }

        _ => unreachable!("not a notification action"),
    })
}
