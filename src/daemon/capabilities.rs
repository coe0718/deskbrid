
use super::MONITOR_CONTROL_ACTIONS;

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

if desktop.contains("kde") || desktop.contains("hyprland") {
    set_degraded(
        &mut actions,
        "input.keyboard",
        "depends_on_ydotoold_and_uinput_permissions",
    );
    set_degraded(
        &mut actions,
        "input.mouse",
        "depends_on_ydotoold_and_uinput_permissions",
    );
    set_requires(&mut actions, "input.keyboard", &["ydotoold", "/dev/uinput"]);
    set_requires(&mut actions, "input.mouse", &["ydotoold", "/dev/uinput"]);
    set_session(&mut actions, "input.keyboard", "wayland");
    set_session(&mut actions, "input.mouse", "wayland");
}

if desktop.contains("kde") {
    for action in MONITOR_CONTROL_ACTIONS {
        set_requires(&mut actions, action, &["kscreen-doctor"]);
    }
}

if desktop.contains("hyprland") {
    for action in MONITOR_CONTROL_ACTIONS {
        set_requires(&mut actions, action, &["hyprctl"]);
    }
    set_unsupported(
        &mut actions,
        "monitor.set_primary",
        "hyprland_has_no_primary_monitor_setting",
    );
}

if desktop.contains("x11") {
    for action in MONITOR_CONTROL_ACTIONS {
        set_requires(&mut actions, action, &["xrandr"]);
    }
    set_degraded(
        &mut actions,
        "windows.activate_or_launch",
        "x11_window_enumeration_unavailable_launch_only",
    );
    set_degraded(
        &mut actions,
        "layout_profiles.save",
        "x11_window_enumeration_unavailable",
    );
    set_degraded(
        &mut actions,
        "layout_profiles.restore",
        "x11_window_enumeration_unavailable",
    );
    set_requires(&mut actions, "windows.maximize", &["wmctrl"]);
    // X11 backend doesn't support notification actions via GNOME/KDE APIs
    set_unsupported(&mut actions, "notification.send", "x11_unsupported");
    set_unsupported(&mut actions, "notification.close", "x11_unsupported");
    set_unsupported(&mut actions, "screencast.start", "x11_unsupported");
    set_unsupported(&mut actions, "screencast.stop", "x11_unsupported");
}

for action in [
    "ui.tree.get",
    "ui.element.click",
    "ui.element.set_text",
    "bluetooth.pair",
    "bluetooth.forget",
] {
    set_unsupported(&mut actions, action, "not_implemented");
}

if desktop.contains("hyprland") {
    set_unsupported(
        &mut actions,
        "windows.minimize",
        "hyprland_has_no_native_minimize_dispatcher",
    );
}

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

if desktop.contains("gnome") {
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
} else if desktop.contains("kde") {
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
} else if desktop.contains("hyprland") {
    deps.insert("hyprctl".to_string(), check_in_path("hyprctl"));
    deps.insert("ydotoold".to_string(), check_process("ydotoold").await);
    deps.insert("ydotool".to_string(), check_in_path("ydotool"));

    deps.insert("uinput".to_string(), check_uinput());
    deps.insert("grim".to_string(), check_in_path("grim"));
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

pub fn set_degraded(
actions: &mut serde_json::Map<String, serde_json::Value>,
action: &str,
reason: &str,
) {
actions.insert(
    action.to_string(),
    serde_json::json!({"supported": true, "degraded": true, "reason": reason, "requires": [], "session": "any", "degraded_modes": [reason]}),
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

pub fn set_unsupported(
actions: &mut serde_json::Map<String, serde_json::Value>,
action: &str,
reason: &str,
) {
actions.insert(
    action.to_string(),
    serde_json::json!({"supported": false, "degraded": false, "reason": reason, "requires": [], "session": "any", "degraded_modes": []}),
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
pub fn check_in_path(cmd: &str) -> serde_json::Value {
match std::process::Command::new("sh")
    .arg("-c")
    .arg(format!("command -v {} >/dev/null 2>&1", cmd))
    .status()
{
    Ok(status) if status.success() => serde_json::json!({"ok": true, "details": "present"}),
    Ok(_) => serde_json::json!({"ok": false, "details": "missing"}),
    Err(e) => serde_json::json!({"ok": false, "details": format!("check failed: {}", e)}),
}
}

pub fn health_remediation() -> serde_json::Value {
serde_json::json!({
    "ydotoold": "Start ydotoold in your user session (e.g. autostart entry).",
    "uinput": "Configure udev: KERNEL==\"uinput\", GROUP=\"input\", MODE=\"0660\" and add your user to input group.",
    "gnome-extension": "Install/enable deskbrid GNOME extension, then restart shell/session.",
    "grim": "Install grim package for screenshots.",
    "spectacle": "Install spectacle package for KDE screenshots."
})
}

pub async fn run_system_remediation(check: &str, apply: bool) -> anyhow::Result<serde_json::Value> {
match check {
    "ydotoold" => {
        if !apply {
            return Ok(serde_json::json!({
                "check":"ydotoold",
                "applied": false,
                "command":"ydotoold &",
                "note":"Set apply=true to start ydotoold in current user session"
            }));
        }
        tokio::process::Command::new("sh")
            .arg("-c")
            .arg("pgrep -x ydotoold >/dev/null 2>&1 || (nohup ydotoold >/tmp/deskbrid-ydotoold.log 2>&1 &)")
            .output()
            .await?;
        // Don't trust nohup's exit code — it exits 0 even if ydotoold crashes immediately.
        // Verify the process actually started.
        let running = tokio::process::Command::new("pgrep")
            .args(["-x", "ydotoold"])
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false);
        Ok(serde_json::json!({
            "check":"ydotoold",
            "applied": running,
            "details": if running { "started_or_already_running" } else { "failed_to_start" }
        }))
    }
    "kde_ydotoold_autostart" => {
        let home = std::env::var("HOME").unwrap_or_default();
        let path = format!("{}/.config/autostart/ydotoold.desktop", home);
        if !apply {
            return Ok(
                serde_json::json!({"check":"kde_ydotoold_autostart","applied":false,"path":path}),
            );
        }
        tokio::fs::create_dir_all(format!("{}/.config/autostart", home)).await?;
        let desktop = "[Desktop Entry]\nType=Application\nExec=ydotoold\nHidden=false\nNoDisplay=false\nX-GNOME-Autostart-enabled=true\nName=Deskbrid ydotool Daemon\nComment=Auto-start ydotoold for input injection\n";
        tokio::fs::write(&path, desktop).await?;
        Ok(serde_json::json!({"check":"kde_ydotoold_autostart","applied":true,"path":path}))
    }
    _ => Ok(serde_json::json!({"check": check,"applied": false,"error": "unknown check"})),
}
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

pub async fn check_process(proc_name: &str) -> serde_json::Value {
match tokio::process::Command::new("pgrep")
    .args(["-x", proc_name])
    .output()
    .await
{
    Ok(out) if out.status.success() => serde_json::json!({"ok": true, "details": "running"}),
    Ok(_) => serde_json::json!({"ok": false, "details": "not running"}),
    Err(e) => serde_json::json!({"ok": false, "details": format!("check failed: {}", e)}),
}
}

pub fn check_cmd(cmd: &str, args: &[&str]) -> serde_json::Value {
match std::process::Command::new(cmd).args(args).output() {
    Ok(out) if out.status.success() => serde_json::json!({"ok": true, "details": "reachable"}),
    Ok(out) => {
        serde_json::json!({"ok": false, "details": format!("failed (code {:?})", out.status.code())})
    }
    Err(e) => serde_json::json!({"ok": false, "details": format!("check failed: {}", e)}),
}
}

pub fn check_uinput() -> serde_json::Value {
let path = std::path::Path::new("/dev/uinput");
if !path.exists() {
    return serde_json::json!({"ok": false, "details": "missing /dev/uinput"});
}
match std::fs::OpenOptions::new().write(true).open(path) {
    Ok(_) => serde_json::json!({"ok": true, "details": "write access"}),
    Err(e) => serde_json::json!({"ok": false, "details": format!("no write access: {}", e)}),
}
}
pub fn check_clipboard_tools() -> serde_json::Value {
let copy = std::process::Command::new("sh")
    .arg("-c")
    .arg("command -v wl-copy >/dev/null 2>&1")
    .status();
let paste = std::process::Command::new("sh")
    .arg("-c")
    .arg("command -v wl-paste >/dev/null 2>&1")
    .status();

let copy_ok = copy.map(|s| s.success()).unwrap_or(false);
let paste_ok = paste.map(|s| s.success()).unwrap_or(false);

if copy_ok && paste_ok {
    serde_json::json!({"ok": true, "details": "wl-copy and wl-paste present"})
} else {
    let mut missing = Vec::new();
    if !copy_ok {
        missing.push("wl-copy");
    }
    if !paste_ok {
        missing.push("wl-paste");
    }
    serde_json::json!({"ok": false, "details": format!("missing: {}", missing.join(", "))})
}
}
