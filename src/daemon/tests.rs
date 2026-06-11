use crate::protocol::Action;
use crate::protocol::WindowInfo;

use super::*;

fn window(id: &str, app_id: &str, title: &str) -> WindowInfo {
    WindowInfo {
        id: id.to_string(),
        title: title.to_string(),
        app_id: app_id.to_string(),
        workspace_id: 1,
        is_focused: false,
        is_minimized: false,
        geometry: None,
        pid: None,
    }
}

#[test]
fn layout_profile_matching_prefers_saved_id() {
    let saved = window("saved-id", "app.one", "Editor");
    let current = vec![
        window("other-id", "app.one", "Editor"),
        window("saved-id", "app.two", "Terminal"),
    ];

    assert_eq!(match_profile_window_index(&saved, &current), Some(1));
}

#[test]
fn layout_profile_matching_consumes_fallback_matches() {
    let saved = [
        window("old-a", "app.editor", "Notes"),
        window("old-b", "app.editor", "Notes"),
    ];
    let mut current = vec![
        window("live-a", "app.editor", "Notes"),
        window("live-b", "app.editor", "Notes"),
    ];

    let first = current.remove(match_profile_window_index(&saved[0], &current).unwrap());
    let second = current.remove(match_profile_window_index(&saved[1], &current).unwrap());

    assert_eq!(first.id, "live-a");
    assert_eq!(second.id, "live-b");
}

#[test]
fn layout_profile_matching_missing_after_only_live_match_is_consumed() {
    let saved = [
        window("old-a", "app.editor", "Notes"),
        window("old-b", "app.editor", "Notes"),
    ];
    let mut current = vec![window("live-a", "app.editor", "Notes")];

    let _ = current.remove(match_profile_window_index(&saved[0], &current).unwrap());

    assert_eq!(match_profile_window_index(&saved[1], &current), None);
}

fn default_capability_actions() -> serde_json::Map<String, serde_json::Value> {
    let mut actions = serde_json::Map::new();
    for action in crate::protocol::Action::public_action_types() {
        actions.insert(
            (*action).to_string(),
            serde_json::json!({
                "supported": true,
                "degraded": false,
                "reason": serde_json::Value::Null,
                "requires": [],
                "session": "any",
                "degraded_modes": []
            }),
        );
    }
    actions
}

#[test]
fn gnome_wayland_marks_primary_monitor_capability_unsupported() {
    let mut actions = default_capability_actions();

    apply_gnome_capability_overrides(&mut actions, "wayland");

    assert_eq!(actions["monitor.set_primary"]["supported"], false);
    assert_eq!(
        actions["monitor.set_primary"]["reason"],
        "gnome_wayland_has_no_primary_monitor_helper"
    );
}

#[test]
fn gnome_x11_keeps_primary_monitor_capability_supported() {
    let mut actions = default_capability_actions();

    apply_gnome_capability_overrides(&mut actions, "x11");

    assert_eq!(actions["monitor.set_primary"]["supported"], true);
    assert_eq!(
        actions["monitor.set_primary"]["requires"],
        serde_json::json!(["xrandr-or-wlr-randr"])
    );
}

#[tokio::test]
async fn audit_actions_work_without_desktop_backend() {
    let state = crate::DaemonState::new();
    // Clear stale on-disk entries from previous test runs.
    state.database.lock().await.clear_audit().unwrap();

    let first = dispatch_action(
        "test",
        crate::protocol::Action::AuditLog {
            limit: None,
            action_type: None,
            status: None,
        },
        &state,
        1000,
        1,
    )
    .await;
    assert_eq!(first["status"], "ok");
    assert_eq!(first["data"]["entries"].as_array().unwrap().len(), 0);

    let second = dispatch_action(
        "test",
        crate::protocol::Action::AuditLog {
            limit: None,
            action_type: None,
            status: Some("ok".to_string()),
        },
        &state,
        1000,
        2,
    )
    .await;
    assert_eq!(second["status"], "ok");
    assert_eq!(second["data"]["entries"][0]["action_type"], "audit.log");
    assert_eq!(second["data"]["entries"][0]["peer_uid"], 1000);
}

#[tokio::test]
async fn dry_run_validates_permissions_without_backend() {
    let state = crate::DaemonState::new();
    // Clear stale on-disk entries from previous test runs.
    state.database.lock().await.clear_audit().unwrap();

    let response = dispatch::dispatch_action_with_options(
        "test",
        crate::protocol::Action::WindowsClose("0x1".to_string()),
        &state,
        1000,
        1,
        crate::protocol::RequestOptions {
            dry_run: true,
            timeout_ms: Some(250),
            require_confirmation: None,
        },
        "default",
    )
    .await;

    assert_eq!(response["status"], "ok");
    assert_eq!(response["data"]["dry_run"], true);
    assert_eq!(response["data"]["action_type"], "windows.close");
    assert_eq!(response["data"]["timeout_ms"], 250);

    let audit = dispatch_action(
        "test",
        crate::protocol::Action::AuditLog {
            limit: None,
            action_type: Some("windows.close".to_string()),
            status: None,
        },
        &state,
        1000,
        2,
    )
    .await;
    assert_eq!(audit["data"]["entries"][0]["dry_run"], true);
}

// ── 1.2 Cache/DB Consistency ──────────────────────────────

#[tokio::test]
async fn clipboard_cache_db_consistent_after_write() {
    let state = crate::DaemonState::new();
    state.database.lock().await.clear_clipboard().unwrap();

    // Write through the daemon API
    crate::daemon::clipboard::record_clipboard_text(&state, "test-one", "api").await;
    crate::daemon::clipboard::record_clipboard_text(&state, "test-two", "api").await;

    // Read back via the query API (goes to DB)
    let response = crate::daemon::clipboard::execute_clipboard_history_action(
        Action::ClipboardHistoryList {
            limit: None,
            query: None,
        },
        &state,
    )
    .await
    .unwrap();

    let entries = response["entries"].as_array().unwrap();
    assert_eq!(
        entries.len(),
        2,
        "two entries should be visible via DB read"
    );
    // Chronological order: oldest first
    assert_eq!(entries[0]["text"], "test-one");
    assert_eq!(entries[1]["text"], "test-two");
}

#[tokio::test]
async fn audit_cache_db_consistent_after_write() {
    let state = crate::DaemonState::new();
    state.database.lock().await.clear_audit().unwrap();

    // Write an audit entry through the daemon path
    dispatch_action(
        "test",
        Action::AuditLog {
            limit: None,
            action_type: None,
            status: None,
        },
        &state,
        1000,
        1,
    )
    .await;

    // Read back — the first query itself should be recorded
    let response = dispatch_action(
        "test",
        Action::AuditLog {
            limit: None,
            action_type: Some("audit.log".to_string()),
            status: Some("ok".to_string()),
        },
        &state,
        1000,
        2,
    )
    .await;

    assert_eq!(response["status"], "ok");
    let entries = response["data"]["entries"].as_array().unwrap();
    assert!(
        !entries.is_empty(),
        "audit.log action should be recorded in DB"
    );
    assert_eq!(entries[0]["action_type"], "audit.log");
}

// ── 1.3 Restart Survival ──────────────────────────────────

#[tokio::test]
async fn clipboard_entries_persist_across_state_instances() {
    let state = crate::DaemonState::new();
    state.database.lock().await.clear_clipboard().unwrap();

    crate::daemon::clipboard::record_clipboard_text(&state, "survivor", "test").await;

    // Drop state so WAL is checkpointed before state2 opens
    drop(state);

    // "Restart": create a new DaemonState — same on-disk DB should have the entry.
    let state2 = crate::DaemonState::new();
    crate::daemon::clipboard::load_clipboard_from_db(&state2).await;

    let response = crate::daemon::clipboard::execute_clipboard_history_action(
        Action::ClipboardHistoryList {
            limit: None,
            query: None,
        },
        &state2,
    )
    .await
    .unwrap();

    let entries = response["entries"].as_array().unwrap();
    assert!(!entries.is_empty(), "entries should survive restart");
    assert!(
        entries.iter().any(|e| e["text"] == "survivor"),
        "survivor entry missing after restart"
    );

    // Cleanup
    state2.database.lock().await.clear_clipboard().unwrap();
}

#[tokio::test]
async fn audit_entries_persist_across_state_instances() {
    let state = crate::DaemonState::new();
    state.database.lock().await.clear_audit().unwrap();

    dispatch_action(
        "test",
        Action::AuditLog {
            limit: None,
            action_type: None,
            status: None,
        },
        &state,
        1000,
        1,
    )
    .await;

    // Drop state so WAL is checkpointed before state2 opens
    drop(state);

    // "Restart": new DaemonState, load from DB, query directly
    let state2 = crate::DaemonState::new();
    crate::daemon::audit::load_audit_from_db(&state2).await;

    // Query via execute_audit_action directly — no new writes, no ID collision
    let response = crate::daemon::audit::execute_audit_action(
        Action::AuditLog {
            limit: None,
            action_type: Some("audit.log".to_string()),
            status: Some("ok".to_string()),
        },
        &state2,
    )
    .await
    .unwrap();

    let entries = response["entries"].as_array().unwrap();
    assert!(!entries.is_empty(), "audit entries should survive restart");
    assert_eq!(entries[0]["action_type"], "audit.log");

    // Cleanup
    state2.database.lock().await.clear_audit().unwrap();
}

// ── 3. Confirmation System ────────────────────────────────

#[tokio::test]
async fn confirmation_queue_stores_pending_requests() {
    let state = crate::DaemonState::new();

    let response = dispatch::dispatch_action_with_options(
        "test",
        Action::WindowsClose("0x1".to_string()),
        &state,
        1000,
        1,
        crate::protocol::RequestOptions {
            dry_run: false,
            timeout_ms: Some(250),
            require_confirmation: Some(true),
        },
        "test-session",
    )
    .await;

    assert_eq!(response["status"], "action_requires_confirmation");
    let confirm_id = response["confirmation_id"].as_str().unwrap().to_string();
    assert!(!confirm_id.is_empty());

    // Verify it's in the pending queue
    assert!(
        state.pending_confirmations.contains_key(&confirm_id),
        "confirmation should be in queue"
    );
    let entry = state.pending_confirmations.get(&confirm_id).unwrap();
    assert_eq!(entry.value().action.action_type(), "windows.close");
    assert_eq!(entry.value().peer_uid, 1000);
    assert_eq!(entry.value().session_id, "test-session");
}

#[tokio::test]
async fn confirmation_deny_removes_from_queue() {
    let state = crate::DaemonState::new();

    // Create a pending confirmation
    let response = dispatch::dispatch_action_with_options(
        "test",
        Action::WindowsClose("0x2".to_string()),
        &state,
        1000,
        1,
        crate::protocol::RequestOptions {
            dry_run: false,
            timeout_ms: Some(250),
            require_confirmation: Some(true),
        },
        "session-deny",
    )
    .await;

    let confirm_id = response["confirmation_id"].as_str().unwrap().to_string();

    // Deny it
    let deny = crate::daemon::execute_confirmation::execute_confirmation(
        Action::DenyAction {
            id: confirm_id.clone(),
        },
        &state,
        1000,
    )
    .await
    .unwrap();

    assert_eq!(deny["status"], "denied");
    assert_eq!(deny["id"], confirm_id);

    // Verify it's removed from queue
    assert!(
        !state.pending_confirmations.contains_key(&confirm_id),
        "denied confirmation should be removed"
    );
}

#[tokio::test]
async fn confirmation_deny_nonexistent_returns_not_found() {
    let state = crate::DaemonState::new();

    let result = crate::daemon::execute_confirmation::execute_confirmation(
        Action::DenyAction {
            id: "nonexistent-id".to_string(),
        },
        &state,
        0,
    )
    .await
    .unwrap();

    assert_eq!(result["status"], "not_found");
}

#[tokio::test]
async fn confirmation_list_shows_pending_items() {
    let state = crate::DaemonState::new();

    // Add two confirmations
    for i in 0..2 {
        dispatch::dispatch_action_with_options(
            "test",
            Action::WindowsClose(format!("0x{}", i + 10)),
            &state,
            1000,
            i + 1,
            crate::protocol::RequestOptions {
                dry_run: false,
                timeout_ms: Some(250),
                require_confirmation: Some(true),
            },
            "list-session",
        )
        .await;
    }

    let list = crate::daemon::execute_confirmation::execute_confirmation(
        Action::ConfirmationList,
        &state,
        0,
    )
    .await
    .unwrap();

    assert_eq!(list["count"], 2);
    let items = list["pending"].as_array().unwrap();
    assert_eq!(items.len(), 2);
}

#[tokio::test]
async fn confirmation_sweeper_removes_expired_entries() {
    let state = crate::DaemonState::new();

    // Insert a confirmation with an old timestamp (simulating expiry)
    let old_created_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
        - 400_000; // 400 seconds ago (TTL is 300s)

    let confirm_id = "expired-test-id".to_string();
    state.pending_confirmations.insert(
        confirm_id.clone(),
        crate::daemon::execute_confirmation::PendingConfirmation {
            request_id: "test".to_string(),
            action: Action::WindowsList,
            options: Default::default(),
            peer_uid: 1000,
            seq: 1,
            session_id: "sweep-session".to_string(),
            created_at: old_created_at,
        },
    );

    // Run the sweeper logic directly (single sweep iteration)
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let before = state.pending_confirmations.len();
    state
        .pending_confirmations
        .retain(|_, entry| now_ms.saturating_sub(entry.created_at) < 300_000); // TTL
    let removed = before - state.pending_confirmations.len();

    assert!(removed > 0, "expired confirmation should be swept");
    assert!(
        !state.pending_confirmations.contains_key(&confirm_id),
        "expired entry should be gone"
    );
}

#[tokio::test]
async fn confirmation_rejects_wrong_peer_uid() {
    let state = crate::DaemonState::new();

    // Queue a pending confirmation owned by peer_uid 100
    let confirm_id = "ownership-test".to_string();
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let entry = crate::daemon::execute_confirmation::PendingConfirmation {
        request_id: "req-1".into(),
        action: Action::SystemInfo,
        options: Default::default(),
        peer_uid: 100,
        seq: 1,
        session_id: "default".into(),
        created_at: now_ms,
    };
    state
        .pending_confirmations
        .insert(confirm_id.clone(), entry);

    // Peer 200 tries to deny — should be rejected
    let deny = crate::daemon::execute_confirmation::execute_confirmation(
        Action::DenyAction {
            id: confirm_id.clone(),
        },
        &state,
        200, // wrong peer_uid
    )
    .await
    .unwrap();

    assert_eq!(deny["status"], "denied");
    assert!(
        deny["error"]
            .as_str()
            .unwrap()
            .contains("ownership mismatch")
    );

    // Original entry should still be in queue
    assert!(
        state.pending_confirmations.contains_key(&confirm_id),
        "entry owned by peer 100 should not be removable by peer 200"
    );

    // Peer 100 can deny it
    let deny = crate::daemon::execute_confirmation::execute_confirmation(
        Action::DenyAction {
            id: confirm_id.clone(),
        },
        &state,
        100, // correct peer_uid
    )
    .await
    .unwrap();
    assert_eq!(deny["status"], "denied");
    assert!(!deny.as_object().unwrap().contains_key("error"));

    // Now it's gone
    assert!(!state.pending_confirmations.contains_key(&confirm_id));
}
