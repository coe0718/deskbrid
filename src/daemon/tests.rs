use crate::protocol::Action;
use crate::protocol::WindowInfo;
use std::path::{Path, PathBuf};

use super::*;

fn isolated_state() -> crate::DaemonState {
    crate::DaemonState::with_test_database(crate::daemon::persistence::Database::memory().unwrap())
}

fn temp_db_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "deskbrid-{name}-{}-{}.db",
        std::process::id(),
        uuid::Uuid::new_v4()
    ))
}

fn persistent_state(db_path: &Path) -> crate::DaemonState {
    crate::DaemonState::with_test_database(
        crate::daemon::persistence::Database::open_path(db_path).unwrap(),
    )
}

fn remove_sqlite_files(db_path: &Path) {
    let _ = std::fs::remove_file(db_path);
    let _ = std::fs::remove_file(db_path.with_extension("db-shm"));
    let _ = std::fs::remove_file(db_path.with_extension("db-wal"));
}

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
    let state = isolated_state();

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
    let state = isolated_state();

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
    let state = isolated_state();

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
    let state = isolated_state();

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
    let db_path = temp_db_path("clipboard-persist");
    let state = persistent_state(&db_path);

    crate::daemon::clipboard::record_clipboard_text(&state, "survivor", "test").await;

    // Drop state so WAL is checkpointed before state2 opens
    drop(state);

    // "Restart": create a new DaemonState — same on-disk DB should have the entry.
    let state2 = persistent_state(&db_path);
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

    drop(state2);
    remove_sqlite_files(&db_path);
}

#[tokio::test]
async fn audit_entries_persist_across_state_instances() {
    let db_path = temp_db_path("audit-persist");
    let state = persistent_state(&db_path);

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

    // Force WAL checkpoint so the next DaemonState sees committed data
    state
        .database
        .lock()
        .await
        .conn
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .ok();
    drop(state);

    // "Restart": new DaemonState, load from DB, query directly
    let state2 = persistent_state(&db_path);
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

    drop(state2);
    remove_sqlite_files(&db_path);
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

async fn mock_protocol_state() -> crate::DaemonState {
    let mut state = isolated_state();
    state.permissions = crate::permissions::Permissions::default_safe();
    let backend = crate::backend::create_mock_backend(state.event_tx.clone())
        .await
        .unwrap();
    *state.backend.write().await = Some(backend);
    state
}

async fn dispatch_json_to_mock(
    state: &crate::DaemonState,
    seq: u64,
    line: &str,
) -> serde_json::Value {
    let (id, action, options) = Action::from_json_with_options(line).unwrap();
    dispatch::dispatch_action_with_options(&id, action, state, 1000, seq, options, "mock-test")
        .await
}

#[tokio::test]
async fn mock_backend_dispatches_core_protocol_actions() {
    let state = mock_protocol_state().await;

    let windows =
        dispatch_json_to_mock(&state, 1, r#"{"type":"windows.list","id":"windows"}"#).await;
    assert_eq!(windows["status"], "ok");
    assert!(windows["data"].as_array().unwrap().len() >= 3);

    let focus = dispatch_json_to_mock(
        &state,
        2,
        r#"{"type":"windows.focus","id":"focus","window_id":"mock-browser"}"#,
    )
    .await;
    assert_eq!(focus["status"], "ok");
    assert_eq!(focus["data"]["focused"], "mock-browser");

    let focused = dispatch_json_to_mock(
        &state,
        3,
        r#"{"type":"windows.get","id":"get","window_id":"mock-browser"}"#,
    )
    .await;
    assert_eq!(focused["status"], "ok");
    assert_eq!(focused["data"]["is_focused"], true);

    let switch = dispatch_json_to_mock(
        &state,
        4,
        r#"{"type":"workspaces.switch","id":"workspace","workspace_id":2}"#,
    )
    .await;
    assert_eq!(switch["status"], "ok");

    let info = dispatch_json_to_mock(&state, 5, r#"{"type":"system.info","id":"info"}"#).await;
    assert_eq!(info["status"], "ok");
    assert_eq!(info["data"]["desktop"], "Mock");
    assert_eq!(info["data"]["current_workspace"], 2);

    let monitor = dispatch_json_to_mock(
        &state,
        6,
        r#"{"type":"monitor.set_resolution","id":"monitor","output":"mock-0","width":1280,"height":720}"#,
    )
    .await;
    assert_eq!(monitor["status"], "ok");

    let monitors =
        dispatch_json_to_mock(&state, 7, r#"{"type":"monitor.list","id":"monitors"}"#).await;
    assert_eq!(monitors["status"], "ok");
    assert_eq!(monitors["data"][0]["width"], 1280);
    assert_eq!(monitors["data"][0]["height"], 720);
}

#[tokio::test]
async fn mock_backend_dispatches_screenshot_and_input_without_real_desktop() {
    let state = mock_protocol_state().await;

    let input = dispatch_json_to_mock(
        &state,
        1,
        r#"{"type":"input.keyboard","id":"keyboard","action":"type","text":"hello"}"#,
    )
    .await;
    assert_eq!(input["status"], "ok");
    assert_eq!(input["data"]["typed"], 5);

    let screenshot = dispatch_json_to_mock(
        &state,
        2,
        r#"{"type":"screenshot","id":"shot","region":{"x":0,"y":0,"width":9,"height":7}}"#,
    )
    .await;
    assert_eq!(screenshot["status"], "ok");
    assert_eq!(screenshot["data"]["width"], 9);
    assert_eq!(screenshot["data"]["height"], 7);
    let path = screenshot["data"]["path"].as_str().unwrap();
    assert!(std::path::Path::new(path).exists());
    let _ = std::fs::remove_file(path);

    let clipboard =
        dispatch_json_to_mock(&state, 3, r#"{"type":"clipboard.read","id":"clipboard"}"#).await;
    assert_eq!(clipboard["status"], "ok");
    assert_eq!(clipboard["data"]["text"], "mock clipboard");
}

#[tokio::test]
async fn watch_actions_create_list_and_remove_with_mock_backend() {
    let state = mock_protocol_state().await;

    let region_create = dispatch_json_to_mock(
        &state,
        1,
        r#"{"type":"region_watch.create","id":"region-create","name":"mock-region","region":{"x":0,"y":0,"width":20,"height":20},"interval_ms":10000,"change_threshold_pct":1.0}"#,
    )
    .await;
    assert_eq!(region_create["status"], "ok");
    assert_eq!(region_create["data"]["created"], "mock-region");

    let region_list = dispatch_json_to_mock(
        &state,
        2,
        r#"{"type":"region_watch.list","id":"region-list"}"#,
    )
    .await;
    assert_eq!(region_list["status"], "ok");
    assert_eq!(region_list["data"]["count"], 1);

    let region_remove = dispatch_json_to_mock(
        &state,
        3,
        r#"{"type":"region_watch.remove","id":"region-remove","name":"mock-region"}"#,
    )
    .await;
    assert_eq!(region_remove["status"], "ok");

    let text_create = dispatch_json_to_mock(
        &state,
        4,
        r#"{"type":"text_watch.create","id":"text-create","name":"mock-text","region":{"x":0,"y":0,"width":40,"height":20},"interval_ms":10000,"notify_on_match":"Done","max_entries":3}"#,
    )
    .await;
    assert_eq!(text_create["status"], "ok");
    assert_eq!(text_create["data"]["created"], "mock-text");

    let text_list =
        dispatch_json_to_mock(&state, 5, r#"{"type":"text_watch.list","id":"text-list"}"#).await;
    assert_eq!(text_list["status"], "ok");
    assert_eq!(text_list["data"]["count"], 1);

    let text_remove = dispatch_json_to_mock(
        &state,
        6,
        r#"{"type":"text_watch.remove","id":"text-remove","name":"mock-text"}"#,
    )
    .await;
    assert_eq!(text_remove["status"], "ok");
}
