use crate::protocol::Action;
use serde_json::{Value, json};

/// Default TTL for pending confirmations: 5 minutes.
const CONFIRMATION_TTL_MS: u64 = 300_000;

/// Execute confirmation actions.
pub async fn execute_confirmation(
    action: Action,
    state: &crate::DaemonState,
) -> anyhow::Result<Value> {
    // Sweep expired entries before any operation
    sweep_expired(state).await;

    match action {
        Action::ConfirmAction { id } => {
            let mut pending = state.pending_confirmations.lock().await;
            if let Some(entry) = pending.remove(&id) {
                let backend = state.backend.read().await;
                let backend_ref = backend.as_ref().map(|b| b.as_ref());
                let result = match backend_ref {
                    Some(b) => crate::daemon::execute::execute_action(entry.action, b, state).await,
                    None => Ok(serde_json::json!({
                        "error": "no desktop backend available",
                        "headless": true,
                    })),
                };
                match result {
                    Ok(value) => Ok(json!({"status": "confirmed", "id": id, "result": value})),
                    Err(e) => Ok(json!({"status": "confirmed", "id": id, "error": e.to_string()})),
                }
            } else {
                Ok(
                    json!({"status": "not_found", "id": id, "error": "no pending confirmation with that id"}),
                )
            }
        }
        Action::DenyAction { id } => {
            let mut pending = state.pending_confirmations.lock().await;
            if pending.remove(&id).is_some() {
                Ok(json!({"status": "denied", "id": id}))
            } else {
                Ok(
                    json!({"status": "not_found", "id": id, "error": "no pending confirmation with that id"}),
                )
            }
        }
        Action::ConfirmationList => {
            let pending = state.pending_confirmations.lock().await;
            let items: Vec<_> = pending
                .iter()
                .map(|(id, entry)| {
                    json!({
                        "id": id,
                        "action_type": entry.action.action_type(),
                        "created_at": entry.created_at,
                        "session_id": entry.session_id,
                    })
                })
                .collect();
            Ok(json!({"pending": items, "count": items.len()}))
        }
        _ => unreachable!("not a confirmation action"),
    }
}

/// Purge confirmations older than CONFIRMATION_TTL_MS.
async fn sweep_expired(state: &crate::DaemonState) {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let mut pending = state.pending_confirmations.lock().await;
    pending.retain(|_, entry| now_ms.saturating_sub(entry.created_at) < CONFIRMATION_TTL_MS);
}

pub struct PendingConfirmation {
    pub request_id: String,
    pub action: Action,
    pub options: crate::protocol::RequestOptions,
    pub peer_uid: u32,
    pub seq: u64,
    pub session_id: String,
    pub created_at: u64,
}
