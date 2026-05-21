mod checks;
mod overrides;
mod remediation;

use super::MONITOR_CONTROL_ACTIONS;
use checks::{check_clipboard_tools, check_cmd, check_in_path, check_process, check_uinput};
use overrides::{
    apply_systemd_capability_overrides, set_degraded, set_requires, set_session, set_unsupported,
};
use remediation::health_remediation;

pub use overrides::apply_gnome_capability_overrides;
pub use remediation::run_system_remediation;

pub async fn build_system_capabilities(
    backend: &dyn crate::backend::DesktopBackend,
) -> anyhow::Result<serde_json::Value> {
    let info = backend.system_info().await?;
    let desktop = info.desktop.to_lowercase();
    let session_type = info.session_type.to_lowercase();
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

    if desktop.contains("gnome") {
        apply_gnome_capability_overrides(&mut actions, &session_type);
    }
    apply_systemd_capability_overrides(&mut actions);
    apply_input_capabilities(&mut actions, &desktop);
    apply_monitor_capabilities(&mut actions, &desktop);
    apply_stub_capabilities(&mut actions, &desktop);

    Ok(serde_json::json!({
        "schema_version": 1,
        "backend": desktop,
        "actions": actions,
        "backend_notes": {
            "gnome": "window control via Shell extension + Mutter DBus",
            "kde": "window control via KWin scripting/DBus",
            "hyprland": "window control via hyprctl dispatch"
        }
    }))
}

pub async fn build_system_health(
    backend: &dyn crate::backend::DesktopBackend,
) -> anyhow::Result<serde_json::Value> {
    let desktop = backend.system_info().await?.desktop.to_lowercase();
    let mut deps = serde_json::Map::new();
    insert_system_deps(&mut deps);

    if desktop.contains("gnome") {
        insert_gnome_deps(&mut deps);
    } else if desktop.contains("kde") {
        insert_kde_deps(&mut deps).await;
    } else if desktop.contains("hyprland") {
        insert_hyprland_deps(&mut deps).await;
    } else if desktop.contains("x11") {
        deps.insert("xrandr".to_string(), check_in_path("xrandr"));
    }

    Ok(serde_json::json!({
        "schema_version": 1,
        "backend": desktop,
        "deps": deps,
        "remediation": health_remediation()
    }))
}

pub fn normalize_coords(
    info: &crate::protocol::SystemInfo,
    x: f64,
    y: f64,
    monitor: Option<u32>,
) -> serde_json::Value {
    let target = monitor
        .and_then(|m| info.monitors.iter().find(|mon| mon.id == m))
        .or_else(|| info.monitors.iter().find(|m| m.primary))
        .or_else(|| info.monitors.first());
    if let Some(mon) = target {
        let px = (x * mon.scale).round();
        let py = (y * mon.scale).round();
        serde_json::json!({
            "input": {"x": x, "y": y, "monitor": monitor},
            "monitor": {"id": mon.id, "name": mon.name, "scale": mon.scale, "width": mon.width, "height": mon.height},
            "backend_coords": {"x": px, "y": py}
        })
    } else {
        serde_json::json!({
            "input": {"x": x, "y": y, "monitor": monitor},
            "backend_coords": {"x": x, "y": y},
            "note": "no monitor metadata available"
        })
    }
}

fn apply_input_capabilities(
    actions: &mut serde_json::Map<String, serde_json::Value>,
    desktop: &str,
) {
    if desktop.contains("kde") || desktop.contains("hyprland") {
        set_degraded(
            actions,
            "input.keyboard",
            "depends_on_ydotoold_and_uinput_permissions",
        );
        set_degraded(
            actions,
            "input.mouse",
            "depends_on_ydotoold_and_uinput_permissions",
        );
        set_requires(actions, "input.keyboard", &["ydotoold", "/dev/uinput"]);
        set_requires(actions, "input.mouse", &["ydotoold", "/dev/uinput"]);
        set_session(actions, "input.keyboard", "wayland");
        set_session(actions, "input.mouse", "wayland");
    }
}

fn apply_monitor_capabilities(
    actions: &mut serde_json::Map<String, serde_json::Value>,
    desktop: &str,
) {
    if desktop.contains("kde") {
        for action in MONITOR_CONTROL_ACTIONS {
            set_requires(actions, action, &["kscreen-doctor"]);
        }
    }
    if desktop.contains("hyprland") {
        for action in MONITOR_CONTROL_ACTIONS {
            set_requires(actions, action, &["hyprctl"]);
        }
        set_unsupported(
            actions,
            "monitor.set_primary",
            "hyprland_has_no_primary_monitor_setting",
        );
    }
    if desktop.contains("x11") {
        for action in MONITOR_CONTROL_ACTIONS {
            set_requires(actions, action, &["xrandr"]);
        }
        set_degraded(
            actions,
            "windows.activate_or_launch",
            "x11_window_enumeration_unavailable_launch_only",
        );
        set_degraded(
            actions,
            "layout_profiles.save",
            "x11_window_enumeration_unavailable",
        );
        set_degraded(
            actions,
            "layout_profiles.restore",
            "x11_window_enumeration_unavailable",
        );
        set_requires(actions, "windows.maximize", &["wmctrl"]);
        set_unsupported(actions, "notification.send", "x11_unsupported");
        set_unsupported(actions, "notification.close", "x11_unsupported");
        set_unsupported(actions, "screencast.start", "x11_unsupported");
        set_unsupported(actions, "screencast.stop", "x11_unsupported");
    }
}

fn apply_stub_capabilities(
    actions: &mut serde_json::Map<String, serde_json::Value>,
    desktop: &str,
) {
    for action in [
        "ui.tree.get",
        "ui.element.click",
        "ui.element.set_text",
        "bluetooth.pair",
        "bluetooth.forget",
    ] {
        set_unsupported(actions, action, "not_implemented");
    }
    if desktop.contains("hyprland") {
        set_unsupported(
            actions,
            "windows.minimize",
            "hyprland_has_no_native_minimize_dispatcher",
        );
    }
}

fn insert_system_deps(deps: &mut serde_json::Map<String, serde_json::Value>) {
    deps.insert("systemctl".to_string(), check_in_path("systemctl"));
    deps.insert("loginctl".to_string(), check_in_path("loginctl"));
    deps.insert("journalctl".to_string(), check_in_path("journalctl"));
    deps.insert(
        "systemd-inhibit".to_string(),
        check_in_path("systemd-inhibit"),
    );
    deps.insert("pkcheck".to_string(), check_in_path("pkcheck"));
    deps.insert("dm-tool".to_string(), check_in_path("dm-tool"));
}

fn insert_gnome_deps(deps: &mut serde_json::Map<String, serde_json::Value>) {
    deps.insert(
        "gnome-extension".to_string(),
        check_cmd(
            "gdbus",
            &[
                "introspect",
                "--session",
                "--dest",
                "org.deskbrid.WindowManager",
                "--object-path",
                "/org/deskbrid/WindowManager",
            ],
        ),
    );
    deps.insert("grim".to_string(), check_in_path("grim"));
    deps.insert("wl_clipboard".to_string(), check_clipboard_tools());
    deps.insert("xrandr".to_string(), check_in_path("xrandr"));
    deps.insert("wlr-randr".to_string(), check_in_path("wlr-randr"));
}

async fn insert_kde_deps(deps: &mut serde_json::Map<String, serde_json::Value>) {
    deps.insert("qdbus6".to_string(), check_in_path("qdbus6"));
    deps.insert(
        "kscreen-doctor".to_string(),
        check_in_path("kscreen-doctor"),
    );
    deps.insert("spectacle".to_string(), check_in_path("spectacle"));
    deps.insert("imagemagick_convert".to_string(), check_in_path("convert"));
    deps.insert("ydotoold".to_string(), check_process("ydotoold").await);
    deps.insert("ydotool".to_string(), check_in_path("ydotool"));
    deps.insert("uinput".to_string(), check_uinput());
}

async fn insert_hyprland_deps(deps: &mut serde_json::Map<String, serde_json::Value>) {
    deps.insert("hyprctl".to_string(), check_in_path("hyprctl"));
    deps.insert("ydotoold".to_string(), check_process("ydotoold").await);
    deps.insert("ydotool".to_string(), check_in_path("ydotool"));
    deps.insert("uinput".to_string(), check_uinput());
    deps.insert("grim".to_string(), check_in_path("grim"));
}
