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

/// Whether clipboard history persistence is enabled. Defaults to true.
/// Set DESKBRID_CLIPBOARD_HISTORY=false to disable — clipboard reads/writes
/// still work, but nothing is persisted to disk or in-memory history.
///
/// Set DESKBRID_CLIPBOARD_HISTORY_REDACT_SECRETS=false to allow secrets to be
/// persisted (default: true, secrets are detected and skipped). Detection
/// is heuristic — high-entropy random strings, AWS keys, JWT tokens, and
/// PEM-style key headers all trigger the filter.
pub(crate) fn clipboard_history_enabled() -> bool {
    std::env::var("DESKBRID_CLIPBOARD_HISTORY")
        .map(|v| v.to_lowercase() != "false" && v != "0")
        .unwrap_or(true)
}

/// Whether to filter suspected secrets out of clipboard history.
/// Default: true. Set DESKBRID_CLIPBOARD_HISTORY_REDACT_SECRETS=false to disable.
pub(crate) fn clipboard_history_redact_secrets() -> bool {
    std::env::var("DESKBRID_CLIPBOARD_HISTORY_REDACT_SECRETS")
        .map(|v| v.to_lowercase() != "false" && v != "0")
        .unwrap_or(true)
}

/// Heuristic secret detection for clipboard history redaction.
/// Returns true if the text looks like a secret that should not be persisted.
/// This is intentionally conservative — false positives are acceptable (just
/// don't record that entry); false negatives are the failure mode.
///
/// W8 (docs/CODE_REVIEW_VEX.md): prevents passwords/API keys/tokens that
/// users copy from secret-tool or password managers from being persisted
/// to disk and exposed via `clipboard.history` reads.
pub(crate) fn looks_like_secret(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.len() < 16 {
        return false;
    }
    // PEM key headers / private keys
    if trimmed.starts_with("-----BEGIN") && trimmed.contains("PRIVATE KEY") {
        return true;
    }
    // AWS access keys (AKIA / ASIA prefix + 16 base32 chars)
    if (trimmed.starts_with("AKIA") || trimmed.starts_with("ASIA"))
        && trimmed.len() == 20
        && trimmed
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    {
        return true;
    }
    // JWT (header.payload.signature — three base64url segments)
    if trimmed.split('.').count() == 3 {
        let parts: Vec<&str> = trimmed.split('.').collect();
        if parts.iter().all(|p| p.len() > 8 && !p.is_empty()) && parts[0].starts_with("eyJ") {
            return true;
        }
    }
    // GitHub tokens (ghp_, gho_, ghu_, ghs_, ghr_ prefixes)
    if trimmed.starts_with("ghp_")
        || trimmed.starts_with("gho_")
        || trimmed.starts_with("ghu_")
        || trimmed.starts_with("ghs_")
        || trimmed.starts_with("ghr_")
    {
        return true;
    }
    // Slack tokens (xox[bpars]-...)
    if trimmed.starts_with("xoxb-")
        || trimmed.starts_with("xoxp-")
        || trimmed.starts_with("xoxa-")
        || trimmed.starts_with("xoxr-")
        || trimmed.starts_with("xoxs-")
    {
        return true;
    }
    // High-entropy detection (Shannon entropy ≥ 4.5 bits/char over ≥ 32 chars).
    // Random passwords / API keys typically score 4.0–5.5; English text ≤ 3.5.
    if trimmed.len() >= 32 && shannon_entropy(trimmed) >= 4.5 {
        return true;
    }
    false
}

/// Shannon entropy in bits per character.
fn shannon_entropy(s: &str) -> f64 {
    use std::collections::HashMap;
    let mut counts: HashMap<char, usize> = HashMap::new();
    for c in s.chars() {
        *counts.entry(c).or_insert(0) += 1;
    }
    let len = s.chars().count() as f64;
    if len == 0.0 {
        return 0.0;
    }
    let mut entropy = 0.0;
    for &count in counts.values() {
        let p = count as f64 / len;
        if p > 0.0 {
            entropy -= p * p.log2();
        }
    }
    entropy
}

pub(crate) fn is_clipboard_history_action(action: &Action) -> bool {
    matches!(
        action,
        Action::ClipboardHistoryList { .. } | Action::ClipboardHistoryClear
    )
}

/// Load recent clipboard entries from the DB into the in-memory buffer at startup.
pub(crate) async fn load_clipboard_from_db(state: &DaemonState) {
    let db_arc = state.database.clone();
    let cap = state.clipboard_history_capacity;
    let entries = tokio::task::spawn_blocking(move || {
        let handle = tokio::runtime::Handle::current();
        let db = handle.block_on(db_arc.lock());
        db.get_clipboard_history(cap, None).unwrap_or_else(|e| {
            tracing::warn!("Failed to load clipboard history from database: {e}");
            Vec::new()
        })
    })
    .await
    .unwrap_or_else(|e| {
        tracing::error!("clipboard DB load panicked: {e}");
        Vec::new()
    });
    let mut history = state.clipboard_history.lock().await;
    history.clear();
    for entry in entries.into_iter().rev() {
        history.push_back(entry);
    }
    tracing::info!("Loaded {} clipboard entries from database", history.len());
}

pub(crate) async fn record_clipboard_text(state: &DaemonState, text: &str, source: &str) {
    if !clipboard_history_enabled() {
        return;
    }
    // W8 (docs/CODE_REVIEW_VEX.md): filter suspected secrets before persistence
    // so passwords copied from secret-tool / password managers don't end up
    // in the clipboard_history table exposed via `clipboard.history` reads.
    if clipboard_history_redact_secrets() && looks_like_secret(text) {
        tracing::debug!(
            "clipboard text from {} filtered as suspected secret (not persisted)",
            source
        );
        return;
    }
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
            let db_arc = state.database.clone();
            let mut entries = tokio::task::spawn_blocking(move || {
                let handle = tokio::runtime::Handle::current();
                let db = handle.block_on(db_arc.lock());
                db.get_clipboard_history(limit, query.as_deref())
            })
            .await
            .unwrap()?;
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

    fn isolated_state() -> DaemonState {
        DaemonState::with_test_database(crate::daemon::persistence::Database::memory().unwrap())
    }

    #[tokio::test]
    async fn clipboard_history_dedupes_consecutive_entries() {
        let state = isolated_state();
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

    // W8 (docs/CODE_REVIEW_VEX.md) tests — secret detection before persistence
    #[test]
    fn looks_like_secret_detects_pem_key() {
        let pem =
            "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA...\n-----END RSA PRIVATE KEY-----";
        assert!(looks_like_secret(pem));
    }

    #[test]
    fn looks_like_secret_detects_aws_key() {
        assert!(looks_like_secret("AKIAIOSFODNN7EXAMPLE"));
        assert!(looks_like_secret("ASIARZ4M2K7XE5BNV6PA"));
    }

    #[test]
    fn looks_like_secret_detects_github_token() {
        assert!(looks_like_secret(
            "ghp_aBcDeFgHiJkLmNoPqRsTuVwXyZ0123456789ABCD"
        ));
        assert!(looks_like_secret(
            "ghs_aBcDeFgHiJkLmNoPqRsTuVwXyZ0123456789ABCD"
        ));
    }

    #[test]
    fn looks_like_secret_detects_jwt() {
        // Real JWT format: header.payload.signature (base64url)
        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        assert!(looks_like_secret(jwt));
    }

    #[test]
    fn looks_like_secret_detects_high_entropy_random() {
        // 32 chars of high entropy (random base64-style)
        let random = "aX7q9vB2kP3nM8wL5jR6yT4hF1dC0sZ9eG";
        assert!(looks_like_secret(random));
    }

    #[test]
    fn looks_like_secret_allows_normal_text() {
        assert!(!looks_like_secret("hello world"));
        assert!(!looks_like_secret(
            "the quick brown fox jumps over the lazy dog"
        ));
        assert!(!looks_like_secret("https://example.com/path"));
    }

    #[test]
    fn looks_like_secret_allows_short_text() {
        // Anything under 16 chars shouldn't trigger
        assert!(!looks_like_secret("short"));
        assert!(!looks_like_secret(""));
    }

    #[tokio::test]
    async fn clipboard_history_skips_detected_secrets() {
        let state = isolated_state();
        // Normal text — should persist
        record_clipboard_text(&state, "meeting at 3pm tomorrow", "write").await;
        // Suspected secret — should NOT persist
        let fake_aws = "AKIAIOSFODNN7EXAMPLE";
        record_clipboard_text(&state, fake_aws, "write").await;
        // PEM key — should NOT persist
        let pem =
            "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA\n-----END RSA PRIVATE KEY-----";
        record_clipboard_text(&state, pem, "write").await;

        let response = execute_clipboard_history_action(
            Action::ClipboardHistoryList {
                limit: None,
                query: None,
            },
            &state,
        )
        .await
        .unwrap();

        let entries = response["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["text"], "meeting at 3pm tomorrow");
    }
}
