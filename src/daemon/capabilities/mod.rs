mod checks;
mod confinement;
mod coords;
mod health;
mod overrides;
mod remediation;
mod shared_tools;

use super::MONITOR_CONTROL_ACTIONS;
use overrides::{
    apply_systemd_capability_overrides, set_degraded, set_requires, set_session, set_unsupported,
};
use remediation::health_remediation;
use shared_tools::apply_shared_linux_tool_capabilities;

pub use confinement::build_confinement_report;
pub use coords::normalize_coords;
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
    apply_sysfs_capabilities(&mut actions);
    apply_shared_linux_tool_capabilities(&mut actions, &desktop);
    apply_input_capabilities(&mut actions, &desktop);
    apply_monitor_capabilities(&mut actions, &desktop);
    apply_stub_capabilities(&mut actions, &desktop);
    let confinement = build_confinement_report().await?;

    Ok(serde_json::json!({
        "schema_version": 1,
        "backend": desktop,
        "session_type": session_type,
        "confinement": confinement,
        "actions": actions,
        "backend_notes": {
            "gnome": "window control via Shell extension + Mutter DBus",
            "kde": "window control via KWin scripting/DBus",
            "hyprland": "window control via hyprctl dispatch",
            "cosmic": "window control via cosmic-helper where supported; monitor control via cosmic-randr/wlr-randr",
            "sway": "window and monitor control via swaymsg",
            "niri": "window control via niri msg; monitor control via wlr-randr where supported",
            "wayfire": "window control via wf-ipc; monitor control via wlr-randr where supported",
            "labwc": "window control via wlrctl; monitor control via wlr-randr where supported"
        }
    }))
}

pub async fn build_system_health(
    backend: &dyn crate::backend::DesktopBackend,
) -> anyhow::Result<serde_json::Value> {
    let desktop = backend.system_info().await?.desktop.to_lowercase();
    let mut deps = serde_json::Map::new();
    health::insert_deps(&desktop, &mut deps).await;
    let confinement = build_confinement_report().await?;

    Ok(serde_json::json!({
        "schema_version": 1,
        "backend": desktop,
        "confinement": confinement,
        "deps": deps,
        "remediation": health_remediation()
    }))
}

fn apply_input_capabilities(
    actions: &mut serde_json::Map<String, serde_json::Value>,
    desktop: &str,
) {
    if [
        "kde", "hyprland", "cosmic", "sway", "niri", "wayfire", "labwc",
    ]
    .iter()
    .any(|name| desktop.contains(name))
    {
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
        set_degraded(
            actions,
            "input.mouse.drag",
            "depends_on_ydotoold_and_uinput_permissions",
        );
        set_requires(actions, "input.keyboard", &["ydotoold", "/dev/uinput"]);
        set_requires(actions, "input.mouse", &["ydotoold", "/dev/uinput"]);
        set_requires(actions, "input.mouse.drag", &["ydotoold", "/dev/uinput"]);
        set_session(actions, "input.keyboard", "wayland");
        set_session(actions, "input.mouse", "wayland");
        set_session(actions, "input.mouse.drag", "wayland");
    }
}

fn apply_sysfs_capabilities(actions: &mut serde_json::Map<String, serde_json::Value>) {
    set_requires(actions, "system.backlight.get", &["/sys/class/backlight"]);
    set_requires(
        actions,
        "system.backlight.set",
        &["/sys/class/backlight", "backlight-write-permission"],
    );
}

fn apply_monitor_capabilities(
    actions: &mut serde_json::Map<String, serde_json::Value>,
    desktop: &str,
) {
    if desktop.contains("kde") {
        for action in MONITOR_CONTROL_ACTIONS {
            set_requires(actions, action, &["kscreen-doctor"]);
        }
        set_unsupported(
            actions,
            "notification.close",
            "kde_notify_send_close_unsupported",
        );
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
    if desktop.contains("cosmic") {
        for action in MONITOR_CONTROL_ACTIONS {
            set_requires(actions, action, &["cosmic-randr"]);
        }
        for action in ["windows.move_resize", "windows.tile"] {
            set_unsupported(actions, action, "cosmic_move_resize_not_available");
        }
        set_unsupported(
            actions,
            "notification.close",
            "cosmic_notify_send_close_unsupported",
        );
    }
    if desktop.contains("sway") {
        for action in MONITOR_CONTROL_ACTIONS {
            set_requires(actions, action, &["swaymsg"]);
        }
    }
    if desktop.contains("niri") {
        for action in MONITOR_CONTROL_ACTIONS {
            set_requires(actions, action, &["wlr-randr"]);
        }
        set_unsupported(
            actions,
            "monitor.set_primary",
            "niri_has_no_primary_monitor_setting",
        );
        set_unsupported(actions, "windows.minimize", "niri_has_no_minimize_concept");
        set_degraded(
            actions,
            "windows.move_resize",
            "niri_only_sets_column_width",
        );
        set_degraded(actions, "windows.tile", "niri_only_sets_column_width");
    }
    if desktop.contains("wayfire") {
        for action in MONITOR_CONTROL_ACTIONS {
            set_requires(actions, action, &["wlr-randr"]);
        }
        set_unsupported(
            actions,
            "monitor.set_primary",
            "wayfire_has_no_primary_monitor_setting",
        );
        for action in ["windows.move_resize", "windows.tile"] {
            set_unsupported(actions, action, "wf_ipc_move_resize_not_available");
        }
    }
    if desktop.contains("labwc") {
        for action in MONITOR_CONTROL_ACTIONS {
            set_requires(actions, action, &["wlr-randr"]);
        }
        set_unsupported(
            actions,
            "monitor.set_primary",
            "labwc_has_no_primary_monitor_setting",
        );
        set_unsupported(actions, "windows.minimize", "wlrctl_minimize_not_available");
        for action in ["windows.move_resize", "windows.tile"] {
            set_unsupported(actions, action, "wlrctl_move_resize_not_available");
        }
    }
    if desktop.contains("x11") {
        for action in MONITOR_CONTROL_ACTIONS {
            set_requires(actions, action, &["xrandr"]);
        }
        for action in ["windows.list", "windows.get", "windows.activate_or_launch"] {
            set_requires(actions, action, &["wmctrl"]);
        }
        for action in [
            "windows.focus",
            "windows.close",
            "windows.minimize",
            "windows.move_resize",
            "windows.tile",
            "workspaces.list",
            "workspaces.switch",
            "workspaces.move_window",
            "input.mouse.drag",
        ] {
            set_requires(actions, action, &["xdotool"]);
        }
        set_requires(actions, "windows.maximize", &["wmctrl"]);
        set_requires(actions, "layout_profiles.save", &["wmctrl"]);
        set_requires(actions, "layout_profiles.restore", &["wmctrl", "xdotool"]);
        set_requires(actions, "notification.send", &["notify-send"]);
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
