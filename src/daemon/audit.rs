use std::sync::atomic::Ordering;

use crate::DaemonState;
use crate::protocol::{Action, AuditEntry};

const DEFAULT_AUDIT_CAPACITY: usize = 2048;
const DEFAULT_AUDIT_LIMIT: usize = 100;
const MAX_AUDIT_LIMIT: usize = 1000;
const DEFAULT_ACTION_TIMEOUT_MS: u64 = 60_000;

#[derive(Debug, Clone)]
pub(crate) struct AuditRecord {
    pub seq: u64,
    pub peer_uid: u32,
    pub action_type: String,
    pub status: String,
    pub duration_ms: u64,
    pub error: Option<String>,
    pub dry_run: Option<bool>,
}

pub(crate) fn audit_capacity_from_env() -> usize {
    std::env::var("DESKBRID_AUDIT_MAX_ENTRIES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_AUDIT_CAPACITY)
}

pub(crate) fn action_timeout_from_env() -> Option<u64> {
    std::env::var("DESKBRID_ACTION_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(|value| if value == 0 { None } else { Some(value) })
        .unwrap_or(Some(DEFAULT_ACTION_TIMEOUT_MS))
}

/// Load recent audit entries from the DB into the in-memory buffer at startup.
pub(crate) async fn load_audit_from_db(state: &DaemonState) {
    let db_arc = state.database.clone();
    let cap = state.audit_capacity;
    let entries = tokio::task::spawn_blocking(move || {
        let handle = tokio::runtime::Handle::current();
        let db = handle.block_on(db_arc.lock());
        db.get_audit_log(cap, None, None).unwrap_or_else(|e| {
            tracing::warn!("Failed to load audit log from database: {e}");
            Vec::new()
        })
    })
    .await
    .unwrap_or_else(|e| {
        tracing::error!("audit DB load panicked: {e}");
        Vec::new()
    });
    let mut log = state.audit_log.lock().await;
    log.clear();
    for entry in entries.into_iter().rev() {
        log.push_back(entry);
    }
    tracing::info!("Loaded {} audit entries from database", log.len());
}

pub(crate) async fn record_audit_entry(state: &DaemonState, record: AuditRecord) {
    let entry = AuditEntry {
        id: state.next_audit_id(),
        timestamp: super::unix_timestamp(),
        seq: record.seq,
        peer_uid: record.peer_uid,
        action_type: record.action_type,
        status: record.status,
        duration_ms: record.duration_ms,
        error: record.error,
        dry_run: record.dry_run,
    };

    let mut entries = state.audit_log.lock().await;
    entries.push_back(entry.clone());
    while entries.len() > state.audit_capacity {
        entries.pop_front();
    }
    drop(entries);

    // Persist to SQLite synchronously — DB is the source of truth.
    let db = state.database.lock().await;
    let _ = db.insert_audit(&entry);
}

pub(crate) fn is_audit_action(action: &Action) -> bool {
    matches!(action, Action::AuditLog { .. } | Action::AuditClear)
}

pub(crate) async fn execute_audit_action(
    action: Action,
    state: &DaemonState,
) -> anyhow::Result<serde_json::Value> {
    match action {
        Action::AuditLog {
            limit,
            action_type,
            status,
        } => {
            let limit = limit.unwrap_or(DEFAULT_AUDIT_LIMIT).min(MAX_AUDIT_LIMIT);
            let db_arc = state.database.clone();
            let mut entries = tokio::task::spawn_blocking(move || {
                let handle = tokio::runtime::Handle::current();
                let db = handle.block_on(db_arc.lock());
                db.get_audit_log(limit, action_type.as_deref(), status.as_deref())
            })
            .await
            .unwrap()?;
            entries.reverse(); // DB returns newest-first; return chronological
            Ok(serde_json::json!({
                "entries": entries,
                "count": entries.len(),
                "capacity": state.audit_capacity
            }))
        }
        Action::AuditClear => {
            let mut entries = state.audit_log.lock().await;
            let cleared = entries.len();
            entries.clear();
            drop(entries);
            state.next_audit_id.store(1, Ordering::Relaxed);

            let db = state.database.lock().await;
            db.clear_audit()?;
            Ok(serde_json::json!({"cleared": cleared}))
        }
        _ => anyhow::bail!("not an audit action"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn isolated_state() -> DaemonState {
        DaemonState::with_test_database(crate::daemon::persistence::Database::memory().unwrap())
    }

    #[tokio::test]
    async fn audit_log_filters_newest_entries_then_returns_chronological_order() {
        let state = isolated_state();
        for seq in 1..=3 {
            record_audit_entry(
                &state,
                AuditRecord {
                    seq,
                    peer_uid: 1000,
                    action_type: if seq == 2 {
                        "windows.list".to_string()
                    } else {
                        "clipboard.read".to_string()
                    },
                    status: "ok".to_string(),
                    duration_ms: seq,
                    error: None,
                    dry_run: None,
                },
            )
            .await;
        }

        let response = execute_audit_action(
            Action::AuditLog {
                limit: Some(2),
                action_type: None,
                status: Some("ok".to_string()),
            },
            &state,
        )
        .await
        .unwrap();
        let entries = response["entries"].as_array().unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0]["seq"], 2);
        assert_eq!(entries[1]["seq"], 3);
    }
}
