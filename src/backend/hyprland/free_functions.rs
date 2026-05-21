/// Free functions and helper types for the Hyprland backend.

pub(super) struct HyprMonitorConfig {
    pub(super) name: String,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) refresh_rate: f64,
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) scale: f64,
    pub(super) transform: i32,
}

pub(super) fn json_truthy(value: Option<&serde_json::Value>) -> bool {
    match value {
        None => false,
        Some(v) => !v.is_null() && v != &serde_json::Value::Bool(false) && v != &serde_json::Value::Number(0.into()),
    }
}

/// Auto-detect the running Hyprland instance and Wayland display.
pub(super) fn detect_hypr_instance() -> (Option<String>, Option<String>) {
    let xdg_runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/run/user/1000".to_string());
    let hypr_dir = std::path::Path::new(&xdg_runtime).join("hypr");

    let entries = match std::fs::read_dir(&hypr_dir) {
        Ok(e) => e,
        Err(_) => return (None, None),
    };

    let mut instances: Vec<(std::path::PathBuf, std::time::SystemTime)> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| { e.metadata().ok().and_then(|m| m.modified().ok()).map(|t| (e.path(), t)) })
        .collect();

    instances.sort_by_key(|item| std::cmp::Reverse(item.1));

    if let Some((path, _)) = instances.first() {
        let sig = path.file_name().and_then(|n| n.to_str()).map(|s| s.to_string());
        let wl_sock = std::fs::read_link(path.join(".wayland_socket"))
            .ok()
            .and_then(|p| p.file_name().and_then(|n| n.to_str().map(|s| s.to_string())))
            .or_else(|| Some("wayland-1".to_string()));
        (sig, wl_sock)
    } else {
        (None, None)
    }
}

pub(super) fn rotation_to_hypr_transform(rotation: &str) -> anyhow::Result<i32> {
    match rotation {
        "normal" => Ok(0), "left" => Ok(1), "inverted" => Ok(2), "right" => Ok(3),
        _ => anyhow::bail!("unsupported monitor rotation: {}", rotation),
    }
}

pub(super) fn hypr_transform_to_rotation(transform: i32) -> &'static str {
    match transform { 1 => "left", 2 => "inverted", 3 => "right", _ => "normal" }
}

pub(super) fn format_monitor_float(value: f64) -> String {
    let mut out = format!("{:.3}", value);
    while out.contains('.') && out.ends_with('0') { out.pop(); }
    if out.ends_with('.') { out.pop(); }
    out
}

/// Map a human-readable key name to ydotool keycode name.
pub(super) fn ydotool_key_name(key: &str) -> String {
    match key.to_lowercase().as_str() {
        "return" | "enter" => "ENTER".into(),
        "tab" => "TAB".into(),
        "escape" | "esc" => "ESC".into(),
        "backspace" => "BACKSPACE".into(),
        "delete" | "del" => "DELETE".into(),
        "up" => "UP".into(), "down" => "DOWN".into(),
        "left" => "LEFT".into(), "right" => "RIGHT".into(),
        "home" => "HOME".into(), "end" => "END".into(),
        "page_up" | "pgup" => "PAGEUP".into(),
        "page_down" | "pgdn" => "PAGEDOWN".into(),
        "space" => "SPACE".into(),
        "shift" | "shift_l" | "shift_r" => "LEFTSHIFT".into(),
        "ctrl" | "control" | "control_l" | "ctrl_l" => "LEFTCTRL".into(),
        "alt" | "alt_l" => "LEFTALT".into(),
        "super" | "super_l" | "meta" | "win" | "windows" => "LEFTMETA".into(),
        other => other.to_string(),
    }
}

/// Simple PNG header parser for dimensions.
pub(super) fn get_png_dimensions(path: &str) -> anyhow::Result<(u32, u32)> {
    let data = std::fs::read(path)?;
    if data.len() < 24 || &data[1..4] != b"PNG" { anyhow::bail!("not a PNG file"); }
    let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    Ok((width, height))
}
