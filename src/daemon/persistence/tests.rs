#[cfg(test)]
use super::*;
use crate::protocol::{AuditEntry, EventTrigger};

#[test]
fn clipboard_insert_and_retrieve() {
    let db = Database::memory().unwrap();
    db.insert_clipboard("hello world", Some("write")).unwrap();
    db.insert_clipboard("second entry", Some("read")).unwrap();

    let history = db.get_clipboard_history(10, None).unwrap();
    assert_eq!(history.len(), 2);
    // ORDER BY id DESC — newest first
    assert_eq!(history[0].text, "second entry");
    assert_eq!(history[1].text, "hello world");
}

#[test]
fn clipboard_history_respects_limit() {
    let db = Database::memory().unwrap();
    for i in 0..5 {
        db.insert_clipboard(&format!("entry {}", i), Some("test"))
            .unwrap();
    }
    let history = db.get_clipboard_history(2, None).unwrap();
    assert_eq!(history.len(), 2);
}

#[test]
fn clipboard_clear() {
    let db = Database::memory().unwrap();
    db.insert_clipboard("test", Some("write")).unwrap();
    db.clear_clipboard().unwrap();
    let history = db.get_clipboard_history(10, None).unwrap();
    assert!(history.is_empty());
}

#[test]
fn audit_insert_and_retrieve() {
    let db = Database::memory().unwrap();
    let entry = AuditEntry {
        id: 1,
        timestamp: 1000,
        seq: 1,
        peer_uid: 42,
        action_type: "windows.list".into(),
        status: "ok".into(),
        duration_ms: 15,
        error: None,
        dry_run: Some(false),
    };
    db.insert_audit(&entry).unwrap();

    let log = db.get_audit_log(10, None, None).unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].action_type, "windows.list");
    assert_eq!(log[0].status, "ok");
}

#[test]
fn audit_filter_by_status() {
    let db = Database::memory().unwrap();
    db.insert_audit(&AuditEntry {
        id: 1,
        timestamp: 1000,
        seq: 1,
        peer_uid: 1,
        action_type: "test".into(),
        status: "ok".into(),
        duration_ms: 1,
        error: None,
        dry_run: None,
    })
    .unwrap();
    db.insert_audit(&AuditEntry {
        id: 2,
        timestamp: 1001,
        seq: 2,
        peer_uid: 2,
        action_type: "test".into(),
        status: "error".into(),
        duration_ms: 1,
        error: Some("fail".into()),
        dry_run: None,
    })
    .unwrap();

    let ok_only = db.get_audit_log(10, None, Some("ok")).unwrap();
    assert_eq!(ok_only.len(), 1);
    assert_eq!(ok_only[0].status, "ok");

    let err_only = db.get_audit_log(10, None, Some("error")).unwrap();
    assert_eq!(err_only.len(), 1);
    assert_eq!(err_only[0].error.as_deref(), Some("fail"));
}

#[test]
fn blackboard_upsert_get_delete() {
    let db = Database::memory().unwrap();

    db.upsert_blackboard("greeting", "default", "hello", None)
        .unwrap();
    let val = db.get_blackboard("greeting", "default").unwrap();
    assert_eq!(val.unwrap(), "hello");

    db.upsert_blackboard("greeting", "default", "bonjour", None)
        .unwrap();
    let val = db.get_blackboard("greeting", "default").unwrap();
    assert_eq!(val.unwrap(), "bonjour");

    assert!(db.delete_blackboard("greeting", "default").unwrap());
    assert!(db.get_blackboard("greeting", "default").unwrap().is_none());
}

#[test]
fn blackboard_namespace_isolation() {
    let db = Database::memory().unwrap();
    db.upsert_blackboard("key", "ns1", "alpha", None).unwrap();
    db.upsert_blackboard("key", "ns2", "beta", None).unwrap();

    assert_eq!(db.get_blackboard("key", "ns1").unwrap().unwrap(), "alpha");
    assert_eq!(db.get_blackboard("key", "ns2").unwrap().unwrap(), "beta");
}

#[test]
fn blackboard_list_keys() {
    let db = Database::memory().unwrap();
    db.upsert_blackboard("a", "default", "1", None).unwrap();
    db.upsert_blackboard("b", "default", "2", None).unwrap();
    db.upsert_blackboard("c", "other", "3", None).unwrap();

    let keys = db.blackboard_keys("default").unwrap();
    assert_eq!(keys.len(), 2);
    assert!(keys.contains(&"a".into()));
    assert!(keys.contains(&"b".into()));

    let other = db.blackboard_keys("other").unwrap();
    assert_eq!(other.len(), 1);
    assert_eq!(other[0], "c");
}

#[test]
fn session_upsert_delete_load() {
    let db = Database::memory().unwrap();
    let mut session = crate::SessionData::new("agent-1".into());
    session.vars.insert("greeting".into(), "hello".into());

    db.upsert_session(&session).unwrap();
    let loaded = db.load_sessions().unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].name, "agent-1");
    assert_eq!(loaded[0].vars.get("greeting").unwrap(), "hello");

    db.delete_session("agent-1").unwrap();
    assert!(db.load_sessions().unwrap().is_empty());
}

#[test]
fn rule_upsert_delete_load() {
    let db = Database::memory().unwrap();
    let rule = crate::protocol::Rule {
        id: "r1".into(),
        name: "Test Rule".into(),
        trigger: EventTrigger::ClipboardChanged,
        condition: None,
        action_type: "notification.send".into(),
        action_params: serde_json::json!({"title": "Fired!"}),
        enabled: true,
        cooldown_ms: Some(5000),
        max_fires: Some(10),
    };

    db.upsert_rule(&rule).unwrap();
    let loaded = db.load_rules().unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].name, "Test Rule");
    assert!(loaded[0].enabled);

    db.delete_rule("r1").unwrap();
    assert!(db.load_rules().unwrap().is_empty());
}

#[test]
fn notification_insert_and_query() {
    let db = Database::memory().unwrap();
    db.insert_notification(
        "TestApp",
        "Hello",
        Some("World"),
        Some("normal"),
        None,
        1000,
    )
    .unwrap();
    db.insert_notification("OtherApp", "Alert", None, Some("critical"), None, 1001)
        .unwrap();

    let all = db.get_notifications(10, None, None).unwrap();
    assert_eq!(all.len(), 2);

    let filtered = db.get_notifications(10, Some("TestApp"), None).unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0]["app_name"], "TestApp");
}
