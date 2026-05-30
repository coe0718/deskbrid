use super::*;

#[test]
fn test_glob_match_exact() {
    assert!(glob_match("screenshot", "screenshot"));
    assert!(glob_match("windows.list", "windows.list"));
    assert!(!glob_match("windows.list", "windows.focus"));
}

#[test]
fn test_glob_match_wildcard() {
    assert!(glob_match("*", "screenshot"));
    assert!(glob_match("*", "windows.list"));
    assert!(glob_match("*", "anything.at.all"));
}

#[test]
fn test_glob_match_category() {
    assert!(glob_match("windows.*", "windows.list"));
    assert!(glob_match("windows.*", "windows.focus"));
    assert!(glob_match("windows.*", "windows.get"));
    assert!(glob_match("input.*", "input.keyboard"));
    assert!(glob_match("input.*", "input.mouse"));
}

#[test]
fn test_glob_match_no_false_positives() {
    assert!(!glob_match("windows.*", "screenshot"));
    assert!(!glob_match("windows.*", "clipboard.read"));
    assert!(!glob_match("screenshot", "clipboard.read"));
}

#[test]
fn test_glob_match_prefix_not_segment() {
    assert!(!glob_match("window.*", "windows.list"));
    assert!(!glob_match("clip.*", "clipboard.read"));
}

#[test]
fn test_permissions_allow_all() {
    let p = Permissions::allow_all();
    // Normal actions work under allow-all
    assert!(p.check(
        1000,
        &Action::Screenshot {
            monitor: None,
            region: None,
            window_id: None,
            output: None,
        }
    ));
    assert!(p.check(1000, &Action::ClipboardRead));
    // High-risk actions require explicit naming — wildcard "*" doesn't authorize them
    assert!(!p.check(
        2000,
        &Action::ProcessStart {
            command: vec!["rm".into(), "-rf".into(), "/".into()],
            workdir: None,
            env: None,
        }
    ));
    assert!(!p.check(
        2000,
        &Action::SystemUpdate {
            check: false,
            force: false,
        }
    ));
    assert!(!p.check(
        2000,
        &Action::DbusCall {
            bus: None,
            service: "org.freedesktop.DBus".into(),
            path: "/".into(),
            interface: "org.freedesktop.DBus".into(),
            method: "ListNames".into(),
            args: None,
        }
    ));
}

#[test]
fn test_permissions_deny_screenshot() {
    let inner = PermissionsInner {
        default: PermissionEntry {
            allow: vec!["*".into()],
            deny: vec!["screenshot".into()],
        },
        permissions: HashMap::new(),
    };
    let p = Permissions {
        inner: Arc::new(inner),
    };

    assert!(!p.check(
        1000,
        &Action::Screenshot {
            monitor: None,
            region: None,
            window_id: None,
            output: None,
        }
    ));
    assert!(p.check(1000, &Action::ClipboardRead));
    assert!(p.check(1000, &Action::WindowsList));
}

#[test]
fn test_permissions_per_uid() {
    let mut per_uid = HashMap::new();
    per_uid.insert(
        "uid:1000".into(),
        PermissionEntry {
            allow: vec!["*".into()],
            deny: vec![],
        },
    );
    per_uid.insert(
        "uid:1001".into(),
        PermissionEntry {
            allow: vec!["windows.*".into(), "clipboard.read".into()],
            deny: vec!["screenshot".into()],
        },
    );

    let inner = PermissionsInner {
        default: PermissionEntry {
            allow: vec![],
            deny: vec!["*".into()],
        },
        permissions: per_uid,
    };
    let p = Permissions {
        inner: Arc::new(inner),
    };

    assert!(p.check(
        1000,
        &Action::Screenshot {
            monitor: None,
            region: None,
            window_id: None,
            output: None,
        }
    ));

    assert!(p.check(1001, &Action::WindowsList));
    assert!(p.check(1001, &Action::ClipboardRead));
    assert!(!p.check(
        1001,
        &Action::Screenshot {
            monitor: None,
            region: None,
            window_id: None,
            output: None,
        }
    ));
    assert!(!p.check(
        1001,
        &Action::InputKeyboardType {
            text: "hello".into()
        }
    ));

    assert!(!p.check(9999, &Action::WindowsList));
    assert!(!p.check(9999, &Action::Ping));
}

#[test]
fn test_permissions_ping_always_allowed_in_default_deny() {
    let inner = PermissionsInner {
        default: PermissionEntry {
            allow: vec![],
            deny: vec!["*".into()],
        },
        permissions: HashMap::new(),
    };
    let p = Permissions {
        inner: Arc::new(inner),
    };
    assert!(!p.check(9999, &Action::Ping));
}

#[test]
fn test_high_risk_denied_by_wildcard() {
    // allow_all uses "*" — high-risk actions should still be denied
    let p = Permissions::allow_all();
    assert!(!p.check(
        1000,
        &Action::BrowserEvaluate {
            tab_index: None,
            expression: "alert(1)".into(),
            await_promise: false,
        }
    ));
}

#[test]
fn test_high_risk_denied_by_category_wildcard() {
    let inner = PermissionsInner {
        default: PermissionEntry {
            allow: vec!["browser.*".into()],
            deny: vec![],
        },
        permissions: HashMap::new(),
    };
    let p = Permissions {
        inner: Arc::new(inner),
    };
    // browser.navigate should work via category wildcard
    assert!(p.check(
        1000,
        &Action::BrowserNavigate {
            tab_index: None,
            url: "https://example.com".into(),
        }
    ));
    // browser.evaluate should NOT work via category wildcard
    assert!(!p.check(
        1000,
        &Action::BrowserEvaluate {
            tab_index: None,
            expression: "alert(1)".into(),
            await_promise: false,
        }
    ));
}

#[test]
fn test_high_risk_explicitly_allowed() {
    let inner = PermissionsInner {
        default: PermissionEntry {
            allow: vec!["browser.evaluate".into(), "browser.*".into()],
            deny: vec![],
        },
        permissions: HashMap::new(),
    };
    let p = Permissions {
        inner: Arc::new(inner),
    };
    // Explicit naming should allow it
    assert!(p.check(
        1000,
        &Action::BrowserEvaluate {
            tab_index: None,
            expression: "alert(1)".into(),
            await_promise: false,
        }
    ));
}

#[test]
fn test_high_risk_deny_still_wins() {
    let inner = PermissionsInner {
        default: PermissionEntry {
            allow: vec!["browser.evaluate".into()],
            deny: vec!["browser.evaluate".into()],
        },
        permissions: HashMap::new(),
    };
    let p = Permissions {
        inner: Arc::new(inner),
    };
    // Explicit deny should still override
    assert!(!p.check(
        1000,
        &Action::BrowserEvaluate {
            tab_index: None,
            expression: "alert(1)".into(),
            await_promise: false,
        }
    ));
}

mod action_names;
