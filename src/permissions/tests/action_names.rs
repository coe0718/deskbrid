use super::*;

#[test]
fn test_action_name_mapping() {
    assert_eq!(action_name(&Action::WindowsList), "windows.list");
    assert_eq!(
        action_name(&Action::Screenshot {
            monitor: None,
            region: None,
            window_id: None,
            output: None,
        }),
        "screenshot"
    );
    assert_eq!(
        action_name(&Action::InputKeyboardType { text: "".into() }),
        "input.keyboard"
    );
    assert_eq!(
        action_name(&Action::InputMouse {
            action: "click".into(),
            x: None,
            y: None,
            button: None,
            dx: None,
            dy: None
        }),
        "input.mouse"
    );
    assert_eq!(action_name(&Action::ClipboardRead), "clipboard.read");
    assert_eq!(
        action_name(&Action::ProcessStart {
            command: vec![],
            workdir: None,
            env: None
        }),
        "process.start"
    );
    assert_eq!(
        action_name(&Action::WindowsActivateOrLaunch {
            app_id: "code".into(),
            command: vec!["code".into()],
            workdir: None,
            env: None,
        }),
        "windows.activate_or_launch"
    );
    assert_eq!(
        action_name(&Action::LayoutProfileRestore {
            name: "coding".into()
        }),
        "layout_profiles.restore"
    );
    assert_eq!(
        action_name(&Action::MonitorSetScale {
            output: "DP-1".into(),
            scale: 1.25,
        }),
        "monitor.set_scale"
    );
    assert_eq!(
        action_name(&Action::MonitorDisable {
            output: "HDMI-A-1".into(),
        }),
        "monitor.disable"
    );
    assert_eq!(
        action_name(&Action::SystemInhibit {
            what: "sleep".into(),
            who: "deskbrid".into(),
            why: None,
            mode: None,
        }),
        "system.inhibit"
    );
    assert_eq!(
        action_name(&Action::ServiceRestart {
            name: "ssh.service".into(),
        }),
        "service.restart"
    );
    assert_eq!(
        action_name(&Action::JournalQuery {
            since: None,
            until: None,
            unit: None,
            priority: None,
            tail: None,
        }),
        "journal.query"
    );
    assert_eq!(
        action_name(&Action::SystemElevate {
            action_id: "org.deskbrid.system.service-control".into(),
            reason: None,
        }),
        "system.elevate"
    );
    assert_eq!(
        action_name(&Action::SystemUpdate {
            check: true,
            force: false,
        }),
        "system.update"
    );
    assert_eq!(
        action_name(&Action::DbusCall {
            bus: None,
            service: "org.freedesktop.DBus".into(),
            path: "/org/freedesktop/DBus".into(),
            interface: "org.freedesktop.DBus".into(),
            method: "ListNames".into(),
            args: None,
        }),
        "dbus.call"
    );
}
