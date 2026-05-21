mod checks;
mod coords;
mod overrides;
mod remediation;

use super::MONITOR_CONTROL_ACTIONS;
use checks::{check_clipboard_tools, check_cmd, check_in_path, check_process, check_uinput};
use overrides::{
    apply_systemd_capability_overrides, set_degraded, set_requires, set_session, set_unsupported,
};
use remediation::health_remediation;

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
    insert_system_deps(&mut deps).await;

    if desktop.contains("gnome") {
        insert_gnome_deps(&mut deps).await;
    } else if desktop.contains("kde") {
        insert_kde_deps(&mut deps).await;
    } else if desktop.contains("hyprland") {
        insert_hyprland_deps(&mut deps).await;
    } else if desktop.contains("x11") {
        deps.insert("xrandr".to_string(), check_in_path("xrandr").await);
    }

    Ok(serde_json::json!({
        "schema_version": 1,
        "backend": desktop,
        "deps": deps,
        "remediation": health_remediation()
    }))
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

async fn insert_system_deps(deps: &mut serde_json::Map<String, serde_json::Value>) {
    deps.insert("systemctl".to_string(), check_in_path("systemctl").await);
    deps.insert("loginctl".to_string(), check_in_path("loginctl").await);
    deps.insert("journalctl".to_string(), check_in_path("journalctl").await);
    deps.insert(
        "systemd-inhibit".to_string(),
        check_in_path("systemd-inhibit").await,
    );
    deps.insert("pkcheck".to_string(), check_in_path("pkcheck").await);
    deps.insert("dm-tool".to_string(), check_in_path("dm-tool").await);
    deps.insert("tesseract".to_string(), check_in_path("tesseract").await);
}

async fn insert_gnome_deps(deps: &mut serde_json::Map<String, serde_json::Value>) {
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
        )
        .await,
    );
    deps.insert("grim".to_string(), check_in_path("grim").await);
    deps.insert("wl_clipboard".to_string(), check_clipboard_tools().await);
    deps.insert("xrandr".to_string(), check_in_path("xrandr").await);
    deps.insert("wlr-randr".to_string(), check_in_path("wlr-randr").await);
}

async fn insert_kde_deps(deps: &mut serde_json::Map<String, serde_json::Value>) {
    deps.insert("qdbus6".to_string(), check_in_path("qdbus6").await);
    deps.insert(
        "kscreen-doctor".to_string(),
        check_in_path("kscreen-doctor").await,
    );
    deps.insert("spectacle".to_string(), check_in_path("spectacle").await);
    deps.insert(
        "imagemagick_convert".to_string(),
        check_in_path("convert").await,
    );
    deps.insert("ydotoold".to_string(), check_process("ydotoold").await);
    deps.insert("ydotool".to_string(), check_in_path("ydotool").await);
    deps.insert("uinput".to_string(), check_uinput().await);
}

async fn insert_hyprland_deps(deps: &mut serde_json::Map<String, serde_json::Value>) {
    deps.insert("hyprctl".to_string(), check_in_path("hyprctl").await);
    deps.insert("ydotoold".to_string(), check_process("ydotoold").await);
    deps.insert("ydotool".to_string(), check_in_path("ydotool").await);
    deps.insert("uinput".to_string(), check_uinput().await);
    deps.insert("grim".to_string(), check_in_path("grim").await);
}
