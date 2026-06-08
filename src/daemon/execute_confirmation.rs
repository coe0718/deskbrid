use crate::protocol::Action;
use serde_json::{Value, json};

/// Default TTL for pending confirmations: 5 minutes.
const CONFIRMATION_TTL_MS: u64 = 300_000;

/// Sweep interval for the background task: 30 seconds.
const SWEEP_INTERVAL_SECS: u64 = 30;

/// Returns true if the action is a confirmation management action.
/// These are backend-free: they operate on the in-memory confirmation queue.
pub fn is_confirmation_action(action: &Action) -> bool {
    matches!(
        action,
        Action::ConfirmAction { .. } | Action::DenyAction { .. } | Action::ConfirmationList
    )
}

/// Execute confirmation actions.
pub async fn execute_confirmation(
    action: Action,
    state: &crate::DaemonState,
    caller_uid: u32,
) -> anyhow::Result<Value> {
    match action {
        Action::ConfirmAction { id } => {
            let mut pending = state.pending_confirmations.lock().await;
            // Ownership check BEFORE removal — wrong peer must not consume the entry.
            if let Some(entry) = pending.get(&id)
                && entry.peer_uid != caller_uid
            {
                return Ok(
                    json!({"status": "denied", "id": id, "error": "confirmation ownership mismatch"}),
                );
            }
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
            // Ownership check BEFORE removal — wrong peer must not consume the entry.
            if let Some(entry) = pending.get(&id)
                && entry.peer_uid != caller_uid
            {
                return Ok(
                    json!({"status": "denied", "id": id, "error": "confirmation ownership mismatch"}),
                );
            }
            if let Some(entry) = pending.remove(&id) {
                let _ = entry;
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

/// Spawn a background task that sweeps expired pending confirmations.
/// Runs every SWEEP_INTERVAL_SECS, purging entries older than CONFIRMATION_TTL_MS.
pub fn spawn_confirmation_sweeper(state: std::sync::Arc<crate::DaemonState>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(SWEEP_INTERVAL_SECS)).await;
            let mut pending = state.pending_confirmations.lock().await;
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            let before = pending.len();
            pending
                .retain(|_, entry| now_ms.saturating_sub(entry.created_at) < CONFIRMATION_TTL_MS);
            if pending.len() != before {
                tracing::debug!(
                    "Confirmation sweep: {} → {} (removed {} expired)",
                    before,
                    pending.len(),
                    before - pending.len(),
                );
            }
        }
    });
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
