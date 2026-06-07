use crate::DaemonState;
use crate::protocol::{Action, ClipboardHistoryEntry};

const DEFAULT_CLIPBOARD_HISTORY_CAPACITY: usize = 200;
const DEFAULT_CLIPBOARD_HISTORY_LIMIT: usize = 50;
const MAX_CLIPBOARD_HISTORY_LIMIT: usize = 500;

pub(crate) fn clipboard_history_capacity_from_env() -> usize {
    std::env::var("DESKBRID_CLIPBOARD_HISTORY_MAX_ENTRIES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_CLIPBOARD_HISTORY_CAPACITY)
}

pub(crate) fn is_clipboard_history_action(action: &Action) -> bool {
    matches!(
        action,
        Action::ClipboardHistoryList { .. } | Action::ClipboardHistoryClear
    )
}

/// Load recent clipboard entries from the DB into the in-memory buffer at startup.
pub(crate) async fn load_clipboard_from_db(state: &DaemonState) {
    let db = state.database.lock().await;
    match db.get_clipboard_history(state.clipboard_history_capacity, None) {
        Ok(entries) => {
            let mut history = state.clipboard_history.lock().await;
            history.clear();
            for entry in entries.into_iter().rev() {
                history.push_back(entry);
            }
            tracing::info!("Loaded {} clipboard entries from database", history.len());
        }
        Err(e) => {
            tracing::warn!("Failed to load clipboard history from database: {e}");
        }
    }
}

pub(crate) async fn record_clipboard_text(state: &DaemonState, text: &str, source: &str) {
    let mut history = state.clipboard_history.lock().await;
    if history.back().is_some_and(|entry| entry.text == text) {
        return;
    }

    history.push_back(ClipboardHistoryEntry {
        id: state.next_clipboard_history_id(),
        timestamp: super::unix_timestamp(),
        text: text.to_string(),
        size: text.len(),
        source: source.to_string(),
    });
    while history.len() > state.clipboard_history_capacity {
        history.pop_front();
    }
    drop(history);

    // Persist to SQLite synchronously — DB is the source of truth.
    let db = state.database.lock().await;
    let _ = db.insert_clipboard(text, Some(source));
}

pub(crate) async fn execute_clipboard_history_action(
    action: Action,
    state: &DaemonState,
) -> anyhow::Result<serde_json::Value> {
    match action {
        Action::ClipboardHistoryList { limit, query } => {
            let limit = limit
                .unwrap_or(DEFAULT_CLIPBOARD_HISTORY_LIMIT)
                .min(MAX_CLIPBOARD_HISTORY_LIMIT);
            let query_str = query.as_deref();
            let db = state.database.lock().await;
            let mut entries = db.get_clipboard_history(limit, query_str)?;
            entries.reverse(); // DB returns newest-first; return chronological
            Ok(serde_json::json!({
                "entries": entries,
                "count": entries.len(),
                "capacity": state.clipboard_history_capacity
            }))
        }
        Action::ClipboardHistoryClear => {
            let mut history = state.clipboard_history.lock().await;
            let cleared = history.len();
            history.clear();
            drop(history);

            let db = state.database.lock().await;
            db.clear_clipboard()?;
            Ok(serde_json::json!({"cleared": cleared}))
        }
        _ => anyhow::bail!("not a clipboard history action"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn clipboard_history_dedupes_consecutive_entries() {
        let state = DaemonState::new();
        // Clear stale on-disk entries from previous test runs.
        state.database.lock().await.clear_clipboard().unwrap();
        record_clipboard_text(&state, "hello", "write").await;
        record_clipboard_text(&state, "hello", "read").await;

        let response = execute_clipboard_history_action(
            Action::ClipboardHistoryList {
                limit: None,
                query: None,
            },
            &state,
        )
        .await
        .unwrap();

        assert_eq!(response["entries"].as_array().unwrap().len(), 1);
        assert_eq!(response["entries"][0]["text"], "hello");
    }
}
