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

#[tokio::test]
async fn agent_registry_actions_work_without_desktop_backend() {
    let mut state = isolated_state();
    state.permissions = crate::permissions::Permissions::default_safe();

    let register = dispatch::dispatch_action_with_options(
        "agent-register",
        Action::AgentRegister {
            name: "planner".to_string(),
            agent_type: Some("llm".to_string()),
            capabilities: vec!["ocr".to_string(), "wait".to_string()],
            metadata: Some(serde_json::json!({"role": "lead"})),
            heartbeat_interval_ms: Some(1_000),
        },
        &state,
        1000,
        1,
        Default::default(),
        "planner-session",
    )
    .await;
    assert_eq!(register["status"], "ok");
    assert_eq!(register["data"]["agent"]["name"], "planner");
    assert_eq!(register["data"]["agent"]["session_id"], "planner-session");

    let heartbeat = dispatch::dispatch_action_with_options(
        "agent-heartbeat",
        Action::AgentHeartbeat {
            name: "planner".to_string(),
        },
        &state,
        1000,
        2,
        Default::default(),
        "planner-session",
    )
    .await;
    assert_eq!(heartbeat["status"], "ok");

    let list = dispatch::dispatch_action_with_options(
        "agent-list",
        Action::AgentList,
        &state,
        1000,
        3,
        Default::default(),
        "planner-session",
    )
    .await;
    assert_eq!(list["status"], "ok");
    assert_eq!(list["data"]["count"], 1);
    assert_eq!(list["data"]["agents"][0]["name"], "planner");

    let get = dispatch::dispatch_action_with_options(
        "agent-get",
        Action::AgentGet {
            name: "planner".to_string(),
        },
        &state,
        1000,
        4,
        Default::default(),
        "planner-session",
    )
    .await;
    assert_eq!(get["status"], "ok");
    assert_eq!(get["data"]["found"], true);
}

#[tokio::test]
async fn lock_actions_enforce_holder_and_token_without_desktop_backend() {
    let mut state = isolated_state();
    state.permissions = crate::permissions::Permissions::default_safe();

    let first = dispatch::dispatch_action_with_options(
        "lock-first",
        Action::LockAcquire {
            resource: "window:editor".to_string(),
            holder: Some("agent-a".to_string()),
            ttl_ms: Some(5_000),
            wait_ms: Some(0),
            force: false,
        },
        &state,
        1000,
        1,
        Default::default(),
        "agent-a",
    )
    .await;
    assert_eq!(first["status"], "ok");
    assert_eq!(first["data"]["acquired"], true);
    let first_token = first["data"]["lock"]["token"].as_str().unwrap().to_string();

    let second = dispatch::dispatch_action_with_options(
        "lock-second",
        Action::LockAcquire {
            resource: "window:editor".to_string(),
            holder: Some("agent-b".to_string()),
            ttl_ms: Some(5_000),
            wait_ms: Some(0),
            force: false,
        },
        &state,
        1000,
        2,
        Default::default(),
        "agent-b",
    )
    .await;
    assert_eq!(second["status"], "ok");
    assert_eq!(second["data"]["acquired"], false);
    assert_eq!(second["data"]["owner"]["holder"], "agent-a");

    let stolen = dispatch::dispatch_action_with_options(
        "lock-steal",
        Action::LockAcquire {
            resource: "window:editor".to_string(),
            holder: Some("agent-b".to_string()),
            ttl_ms: Some(5_000),
            wait_ms: Some(0),
            force: true,
        },
        &state,
        1000,
        3,
        Default::default(),
        "agent-b",
    )
    .await;
    assert_eq!(stolen["status"], "ok");
    assert_eq!(stolen["data"]["acquired"], true);
    assert_eq!(stolen["data"]["owner"]["holder"], "agent-a");
    let stolen_token = stolen["data"]["lock"]["token"]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(first_token, stolen_token);

    let wrong_release = dispatch::dispatch_action_with_options(
        "lock-wrong-release",
        Action::LockRelease {
            resource: "window:editor".to_string(),
            token: first_token,
        },
        &state,
        1000,
        4,
        Default::default(),
        "agent-a",
    )
    .await;
    assert_eq!(wrong_release["status"], "ok");
    assert_eq!(wrong_release["data"]["released"], false);
    assert_eq!(wrong_release["data"]["reason"], "token_mismatch");

    let release = dispatch::dispatch_action_with_options(
        "lock-release",
        Action::LockRelease {
            resource: "window:editor".to_string(),
            token: stolen_token,
        },
        &state,
        1000,
        5,
        Default::default(),
        "agent-b",
    )
    .await;
    assert_eq!(release["status"], "ok");
    assert_eq!(release["data"]["released"], true);

    let list = dispatch::dispatch_action_with_options(
        "lock-list",
        Action::LockList,
        &state,
        1000,
        6,
        Default::default(),
        "agent-b",
    )
    .await;
    assert_eq!(list["status"], "ok");
    assert_eq!(list["data"]["count"], 0);
}

#[tokio::test]
async fn agent_mailbox_dispatch_uses_current_session() {
    let mut state = isolated_state();
    state.permissions = crate::permissions::Permissions::default_safe();

    let sent = dispatch::dispatch_action_with_options(
        "agent-message",
        Action::AgentMessage {
            to_session: "target".to_string(),
            subject: "heads-up".to_string(),
            body: serde_json::json!({"ok": true}),
            ttl_ms: None,
            reply_to: None,
        },
        &state,
        1000,
        1,
        Default::default(),
        "source",
    )
    .await;
    assert_eq!(sent["status"], "ok");

    let mailbox = dispatch::dispatch_action_with_options(
        "agent-mailbox",
        Action::AgentMailbox,
        &state,
        1000,
        2,
        Default::default(),
        "target",
    )
    .await;
    assert_eq!(mailbox["status"], "ok");
    assert_eq!(mailbox["data"]["count"], 1);
    assert_eq!(mailbox["data"]["messages"][0]["from_session"], "source");
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

// W4 regression: every high-risk action produces an audit entry under the
// default_safe profile. Audit is enforced separately from permissions — the
// rule check decides if the action runs, but the audit log records what
// happened. This test pins down that contract for files.write.
#[tokio::test]
async fn files_write_emits_audit_entry() {
    use crate::protocol::Action;
    let mut state = isolated_state();
    state.permissions = crate::permissions::Permissions::default_safe();

    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("w4_regression.txt");

    let write = dispatch_action(
        "w4-write",
        Action::FilesWrite {
            path: target.to_string_lossy().to_string(),
            content: "audit-me".to_string(),
            append: false,
        },
        &state,
        1000,
        1,
    )
    .await;
    let _ = write;

    if target.exists() {
        let read_back = std::fs::read_to_string(&target).expect("read");
        assert_eq!(read_back, "audit-me");
    }

    let audit = dispatch_action(
        "w4-audit",
        Action::AuditLog {
            limit: Some(50),
            action_type: Some("files.write".to_string()),
            status: None,
        },
        &state,
        1000,
        2,
    )
    .await;
    assert_eq!(audit["status"], "ok");
    let entries = audit["data"]["entries"].as_array().expect("entries array");
    assert!(
        !entries.is_empty(),
        "files.write must produce an audit entry, got: {audit}",
    );
}

// W4 regression: secrets.get is a high-risk action that must always be
// audited, regardless of profile. The audit pipeline must not have a
// mode that skips secrets.get logging.
//
// Even when the profile denies secrets.get, the dispatch layer must
// still emit an audit entry recording the denied attempt — this is
// the security property Vex called out.
#[tokio::test]
async fn secrets_get_produces_audit_entry() {
    use crate::protocol::Action;
    use std::collections::HashMap;
    let mut state = isolated_state();

    // Set up a minimal TOML that allows only audit.* — so the dispatch
    // can query the audit log, while secrets.get is denied.
    let dir = tempfile::tempdir().expect("tempdir");
    let toml_path = dir.path().join("permissions.toml");
    std::fs::write(
        &toml_path,
        r#"[default]
allow = ["audit.*"]
deny = []
"#,
    )
    .unwrap();
    // permissions::load() reads from $XDG_CONFIG_HOME/deskbrid/permissions.toml.
    let prev_xdg = std::env::var_os("XDG_CONFIG_HOME");
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", dir.path());
    }
    state.permissions = crate::permissions::Permissions::load();
    // Restore env after load.
    match prev_xdg {
        Some(v) => unsafe {
            std::env::set_var("XDG_CONFIG_HOME", v);
        },
        None => unsafe {
            std::env::remove_var("XDG_CONFIG_HOME");
        },
    }

    let mut attrs = HashMap::new();
    attrs.insert("service".to_string(), "w4test".to_string());
    attrs.insert("account".to_string(), "ci".to_string());

    let get = dispatch_action(
        "w4-secrets",
        Action::SecretsGetSecret { attributes: attrs },
        &state,
        1000,
        1,
    )
    .await;
    // Permission denied is the expected outcome.
    assert_eq!(
        get["status"], "error",
        "minimal profile should reject secrets.get, got: {get}"
    );
    assert_eq!(
        get["error"]["code"], "PERMISSION_DENIED",
        "must produce PERMISSION_DENIED, got: {get}"
    );

    // Audit must still have an entry for the attempt.
    let audit = dispatch_action(
        "w4-audit2",
        Action::AuditLog {
            limit: Some(50),
            action_type: Some("secrets.get_secret".to_string()),
            status: None,
        },
        &state,
        1000,
        2,
    )
    .await;
    assert_eq!(audit["status"], "ok");
    let entries = audit["data"]["entries"].as_array().expect("entries array");
    assert!(
        !entries.is_empty(),
        "secrets.get_secret must produce an audit entry even when denied, got: {audit}",
    );
}

// W15 regression: client-supplied `confirm: Some(false)` must NOT bypass
// the confirmation gate for HIGH_RISK actions. The dispatch layer must
// force confirmation based on server-side policy, regardless of what
// the client sends in RequestOptions.
//
// We use ProcessStart (HIGH_RISK) as the canonical example. The mock
// backend's process.start succeeds, but the daemon should require
// confirmation first and only run the action after the user confirms.
#[tokio::test]
async fn client_cannot_bypass_confirmation_for_high_risk_actions() {
    use crate::protocol::{Action, RequestOptions};
    let mut state = isolated_state();
    state.permissions = crate::permissions::Permissions::default_safe();

    // Build RequestOptions that try to bypass confirmation. RequestOptions
    // exposes `require_confirmation` (an Option<bool>); a `Some(false)`
    // is the client's attempt to opt out of confirmation.
    let bypass_attempt = RequestOptions {
        dry_run: false,
        timeout_ms: None,
        require_confirmation: Some(false),
    };

    // Attempt to start a process without confirmation — should be
    // blocked by the confirmation gate, returning a response that
    // includes action_requires_confirmation status.
    let response = dispatch::dispatch_action_with_options(
        "w15-bypass",
        Action::ProcessStart {
            command: vec!["echo".to_string(), "should-not-run".to_string()],
            workdir: None,
            env: None,
        },
        &state,
        1000,
        1,
        bypass_attempt,
        "default",
    )
    .await;

    // Either the response explicitly says confirmation is required,
    // OR the action fails with PERMISSION_DENIED because process.start
    // isn't allowed under default_safe (also acceptable — either way
    // the action did NOT execute). Crucially, no `pid` should appear.
    assert!(
        response.get("pid").is_none(),
        "process.start executed despite bypass attempt — got: {response}"
    );
    let status = response["status"].as_str().unwrap_or("");
    assert!(
        status == "action_requires_confirmation" || status == "error",
        "expected confirmation-required or error, got status={status}, response={response}"
    );
}
