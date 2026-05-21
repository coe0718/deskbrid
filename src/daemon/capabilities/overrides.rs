use super::MONITOR_CONTROL_ACTIONS;

pub fn set_degraded(
    actions: &mut serde_json::Map<String, serde_json::Value>,
    action: &str,
    reason: &str,
) {
    actions.insert(
        action.to_string(),
        serde_json::json!({
            "supported": true,
            "degraded": true,
            "reason": reason,
            "requires": [],
            "session": "any",
            "degraded_modes": [reason]
        }),
    );
}

pub fn apply_gnome_capability_overrides(
    actions: &mut serde_json::Map<String, serde_json::Value>,
    session_type: &str,
) {
    set_degraded(
        actions,
        "input.mouse",
        "absolute_move_may_be_unavailable_without_screencast",
    );
    set_requires(actions, "windows.list", &["gnome-extension"]);
    set_requires(actions, "windows.focus", &["gnome-extension"]);
    set_requires(actions, "windows.close", &["gnome-extension"]);
    set_requires(actions, "windows.minimize", &["gnome-extension"]);
    set_requires(actions, "windows.maximize", &["gnome-extension"]);
    set_requires(actions, "windows.move_resize", &["gnome-extension"]);
    set_requires(actions, "windows.activate_or_launch", &["gnome-extension"]);
    set_requires(actions, "workspaces.list", &["gnome-extension"]);
    set_requires(actions, "workspaces.switch", &["gnome-extension"]);
    set_session(actions, "input.mouse", "wayland");
    for action in MONITOR_CONTROL_ACTIONS {
        set_requires(actions, action, &["xrandr-or-wlr-randr"]);
    }
    if session_type != "x11" {
        set_unsupported(
            actions,
            "monitor.set_primary",
            "gnome_wayland_has_no_primary_monitor_helper",
        );
    }
}

pub fn apply_systemd_capability_overrides(
    actions: &mut serde_json::Map<String, serde_json::Value>,
) {
    for action in [
        "service.status",
        "service.start",
        "service.stop",
        "service.restart",
    ] {
        set_requires(actions, action, &["systemctl"]);
    }
    for action in [
        "service.enable",
        "service.disable",
        "timer.list",
        "timer.start",
        "timer.stop",
    ] {
        set_requires(actions, action, &["systemctl"]);
    }
    set_requires(actions, "journal.query", &["journalctl"]);
    set_requires(actions, "system.sessions", &["loginctl"]);
    set_requires(actions, "system.lock_session", &["loginctl"]);
    set_requires(actions, "system.switch_user", &["dm-tool"]);
    set_requires(actions, "system.inhibit", &["systemd-inhibit"]);
    set_requires(actions, "system.release_inhibit", &["systemd-inhibit"]);
    set_requires(actions, "system.check_auth", &["pkcheck"]);
    set_requires(actions, "system.elevate", &["pkcheck", "polkit-agent"]);
}

pub fn set_unsupported(
    actions: &mut serde_json::Map<String, serde_json::Value>,
    action: &str,
    reason: &str,
) {
    actions.insert(
        action.to_string(),
        serde_json::json!({
            "supported": false,
            "degraded": false,
            "reason": reason,
            "requires": [],
            "session": "any",
            "degraded_modes": []
        }),
    );
}

pub fn set_requires(
    actions: &mut serde_json::Map<String, serde_json::Value>,
    action: &str,
    requires: &[&str],
) {
    if let Some(v) = actions.get_mut(action) {
        v["requires"] = serde_json::json!(requires);
    }
}

pub fn set_session(
    actions: &mut serde_json::Map<String, serde_json::Value>,
    action: &str,
    session: &str,
) {
    if let Some(v) = actions.get_mut(action) {
        v["session"] = serde_json::json!(session);
    }
}
