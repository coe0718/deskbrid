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

