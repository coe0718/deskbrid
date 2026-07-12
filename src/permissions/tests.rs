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
    assert!(p.check(1000, &Action::WindowsList));
    assert!(p.check(1000, &Action::SystemInfo));
    // High-risk actions require explicit naming — wildcard "*" doesn't authorize them.
    // Screenshot and ClipboardRead are high-risk now (expanded list),
    // so they're denied under allow-all's "*" wildcard.
    assert!(!p.check(
        1000,
        &Action::Screenshot {
            monitor: None,
            region: None,
            window_id: None,
            output: None,
        }
    ));
    assert!(!p.check(1000, &Action::ClipboardRead));
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
            audit_level: None,
        },
        permissions: HashMap::new(),
        rate_limits: HashMap::new(),
        profile: HashMap::new(),
        auto_suspend: AutoSuspendConfig::default(),
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
    assert!(p.check(1000, &Action::SystemInfo));
    assert!(p.check(1000, &Action::WindowsList));
}

#[test]
fn test_permissions_per_uid() {
    let mut per_uid = HashMap::new();
    per_uid.insert(
        "uid:1000".into(),
        PermissionEntry {
            allow: vec!["*".into(), "screenshot".into()],
            deny: vec![],
            audit_level: None,
        },
    );
    per_uid.insert(
        "uid:1001".into(),
        PermissionEntry {
            allow: vec!["windows.*".into(), "clipboard.read".into()],
            deny: vec!["screenshot".into()],
            audit_level: None,
        },
    );

    let inner = PermissionsInner {
        default: PermissionEntry {
            allow: vec![],
            deny: vec!["*".into()],
            audit_level: None,
        },
        permissions: per_uid,
        rate_limits: HashMap::new(),
        profile: HashMap::new(),
        auto_suspend: AutoSuspendConfig::default(),
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
            audit_level: None,
        },
        permissions: HashMap::new(),
        rate_limits: HashMap::new(),
        profile: HashMap::new(),
        auto_suspend: AutoSuspendConfig::default(),
    };
    let p = Permissions {
        inner: Arc::new(inner),
    };
    assert!(!p.check(9999, &Action::Ping));
}

#[test]
fn test_default_safe_allows_canonical_input_layout_actions() {
    let p = Permissions::default_safe();

    assert!(p.check(1000, &Action::InputListLayouts));
    assert!(p.check(1000, &Action::InputGetLayout));
    assert!(p.check(
        1000,
        &Action::InputSetLayout {
            index: None,
            name: Some("us".into()),
            variant: None,
        }
    ));
    assert!(p.check(
        1000,
        &Action::InputAddLayout {
            name: "de".into(),
            variant: None,
        }
    ));
    assert!(p.check(1000, &Action::InputRemoveLayout { index: 1 }));
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
            audit_level: None,
        },
        permissions: HashMap::new(),
        rate_limits: HashMap::new(),
        profile: HashMap::new(),
        auto_suspend: AutoSuspendConfig::default(),
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
            audit_level: None,
        },
        permissions: HashMap::new(),
        rate_limits: HashMap::new(),
        profile: HashMap::new(),
        auto_suspend: AutoSuspendConfig::default(),
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
            audit_level: None,
        },
        permissions: HashMap::new(),
        rate_limits: HashMap::new(),
        profile: HashMap::new(),
        auto_suspend: AutoSuspendConfig::default(),
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

#[test]
fn test_high_risk_all_blocked_by_wildcard() {
    // Every HIGH_RISK_ACTIONS entry must be blocked under allow-all ("*").
    let p = Permissions::allow_all();

    // browser.evaluate
    assert!(!p.check(
        1000,
        &Action::BrowserEvaluate {
            tab_index: None,
            expression: "1+1".into(),
            await_promise: false,
        }
    ));
    // process.start
    assert!(!p.check(
        1000,
        &Action::ProcessStart {
            command: vec!["echo".into(), "hi".into()],
            workdir: None,
            env: None,
        }
    ));
    // process.stop
    assert!(!p.check(
        1000,
        &Action::ProcessStop {
            pid: 1,
            signal: None,
        }
    ));
    // process.signal
    assert!(!p.check(
        1000,
        &Action::ProcessSignal {
            pid: 1,
            signal: "SIGTERM".into(),
        }
    ));
    // terminal.create
    assert!(!p.check(
        1000,
        &Action::TerminalCreate {
            shell: None,
            rows: Some(24),
            cols: Some(80),
            cwd: None,
            env: None,
        }
    ));
    // system.update
    assert!(!p.check(
        1000,
        &Action::SystemUpdate {
            check: false,
            force: false,
        }
    ));
    // system.power
    assert!(!p.check(
        1000,
        &Action::SystemPower {
            action: "suspend".into(),
        }
    ));
    // dbus.call
    assert!(!p.check(
        1000,
        &Action::DbusCall {
            bus: None,
            service: "org.freedesktop.DBus".into(),
            path: "/".into(),
            interface: "org.freedesktop.DBus".into(),
            method: "ListNames".into(),
            args: None,
        }
    ));
    // files.write
    assert!(!p.check(
        1000,
        &Action::FilesWrite {
            path: "/tmp/test".into(),
            content: "data".into(),
            append: false,
        }
    ));
    // files.delete
    assert!(!p.check(
        1000,
        &Action::FilesDelete {
            path: "/tmp/test".into(),
            recursive: false,
        }
    ));
    // files.move
    assert!(!p.check(
        1000,
        &Action::FilesMove {
            source: "/tmp/a".into(),
            destination: "/tmp/b".into(),
        }
    ));
    // clipboard.read
    assert!(!p.check(1000, &Action::ClipboardRead));
    // clipboard.history
    assert!(!p.check(
        1000,
        &Action::ClipboardHistoryList {
            limit: None,
            query: None,
        }
    ));
    // screenshot
    assert!(!p.check(
        1000,
        &Action::Screenshot {
            monitor: None,
            region: None,
            window_id: None,
            output: None,
        }
    ));
    // screenshot.ocr
    assert!(!p.check(
        1000,
        &Action::ScreenshotOcr {
            path: None,
            language: None,
            psm: None,
            bounding_boxes: false,
            monitor: None,
            region: None,
            window_id: None,
        }
    ));
    // screenshot.diff
    assert!(!p.check(
        1000,
        &Action::ScreenshotDiff {
            before_path: "a.png".into(),
            after_path: None,
            tolerance: None,
            diff_path: None,
            save_diff: false,
            monitor: None,
            region: None,
            window_id: None,
        }
    ));
    // input.keyboard
    assert!(!p.check(
        1000,
        &Action::InputKeyboardType {
            text: "hello".into(),
        }
    ));
    // input.mouse
    assert!(!p.check(
        1000,
        &Action::InputMouse {
            action: "move".into(),
            x: Some(100.0),
            y: Some(200.0),
            button: None,
            dx: None,
            dy: None,
        }
    ));
    // input.mouse.drag
    assert!(!p.check(
        1000,
        &Action::InputMouseDrag {
            from_x: 0.0,
            from_y: 0.0,
            to_x: 100.0,
            to_y: 100.0,
            button: None,
            duration_ms: None,
        }
    ));
    // secrets.get_secret
    assert!(!p.check(
        1000,
        &Action::SecretsGetSecret {
            attributes: std::collections::HashMap::new(),
        }
    ));
    // secrets.store_secret
    assert!(!p.check(
        1000,
        &Action::SecretsStoreSecret {
            attributes: std::collections::HashMap::new(),
            secret: "s3cret".into(),
            label: None,
            collection: None,
        }
    ));
}

#[test]
fn test_permissions_allow_all_function() {
    // Permissions::allow_all() uses "*" — safe actions pass, high-risk actions don't.
    // We test the function directly (not through load() which now returns default_safe).
    let p = Permissions::allow_all();
    assert!(p.check(1000, &Action::WindowsList));
    assert!(p.check(1000, &Action::SystemInfo));
    // High-risk actions are denied under "*" wildcard
    assert!(!p.check(1000, &Action::ClipboardRead));
}

#[test]
fn test_permissions_deny_all_blocks_everything_except_ping() {
    let p = Permissions::deny_all();
    // Everything should be denied
    assert!(!p.check(1000, &Action::WindowsList));
    assert!(!p.check(1000, &Action::ClipboardRead));
    assert!(!p.check(
        1000,
        &Action::Screenshot {
            monitor: None,
            region: None,
            window_id: None,
            output: None,
        }
    ));
}

#[test]
fn test_profile_narrows_uid_permissions_and_requires_explicit_high_risk() {
    let mut profiles = HashMap::new();
    profiles.insert(
        "code-agent".into(),
        ProfileEntry {
            allow: vec!["windows.*".into(), "clipboard.read".into()],
            deny: vec!["windows.close".into()],
            confirm: vec!["clipboard.read".into()],
            audit_level: Some("all".into()),
            rate_limits: HashMap::new(),
            invalid_rate_limits: Vec::new(),
        },
    );
    let p = Permissions {
        inner: Arc::new(PermissionsInner {
            default: PermissionEntry {
                allow: vec!["*".into(), "clipboard.read".into()],
                deny: vec![],
                audit_level: None,
            },
            permissions: HashMap::new(),
            rate_limits: HashMap::new(),
            profile: profiles,
            auto_suspend: AutoSuspendConfig::default(),
        }),
    };

    assert!(matches!(
        p.check_profile(Some("code-agent"), &Action::WindowsList),
        ProfileCheck::Allowed
    ));
    assert!(matches!(
        p.check_profile(Some("code-agent"), &Action::WindowsClose("0x1".into())),
        ProfileCheck::Denied { .. }
    ));
    assert!(matches!(
        p.check_profile(Some("code-agent"), &Action::ClipboardRead),
        ProfileCheck::Allowed
    ));
    assert!(p.profile_requires_confirmation(Some("code-agent"), &Action::ClipboardRead));
}

mod action_names;
