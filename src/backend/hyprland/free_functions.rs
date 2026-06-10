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
        Some(v) => {
            !v.is_null()
                && v != &serde_json::Value::Bool(false)
                && v != &serde_json::Value::Number(0.into())
        }
    }
}

/// Auto-detect the running Hyprland instance and Wayland display.
pub(super) async fn detect_hypr_instance() -> (Option<String>, Option<String>) {
    let xdg_runtime = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
    let hypr_dir = std::path::Path::new(&xdg_runtime).join("hypr");

    let mut entries = match tokio::fs::read_dir(&hypr_dir).await {
        Ok(e) => e,
        Err(_) => return (None, None),
    };

    let mut instances = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let Ok(file_type) = entry.file_type().await else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }
        if let Ok(metadata) = entry.metadata().await
            && let Ok(modified) = metadata.modified()
        {
            instances.push((entry.path(), modified));
        }
    }

    instances.sort_by_key(|item| std::cmp::Reverse(item.1));

    if let Some((path, _)) = instances.first() {
        let sig = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string());
        let wl_sock = tokio::fs::read_link(path.join(".wayland_socket"))
            .await
            .ok()
            .and_then(|p| {
                p.file_name()
                    .and_then(|n| n.to_str().map(|s| s.to_string()))
            })
            .or_else(|| Some("wayland-1".to_string()));
        (sig, wl_sock)
    } else {
        (None, None)
    }
}

pub(super) fn rotation_to_hypr_transform(rotation: &str) -> anyhow::Result<i32> {
    match rotation {
        "normal" => Ok(0),
        "left" => Ok(1),
        "inverted" => Ok(2),
        "right" => Ok(3),
        _ => anyhow::bail!("unsupported monitor rotation: {}", rotation),
    }
}

pub(super) fn hypr_transform_to_rotation(transform: i32) -> &'static str {
    match transform {
        1 => "left",
        2 => "inverted",
        3 => "right",
        _ => "normal",
    }
}

pub(super) fn format_monitor_float(value: f64) -> String {
    let mut out = format!("{:.3}", value);
    while out.contains('.') && out.ends_with('0') {
        out.pop();
    }
    if out.ends_with('.') {
        out.pop();
    }
    out
}

/// Map a human-readable key name to ydotool keycode name.
pub(super) fn ydotool_key_name(key: &str) -> String {
    // ydotool key subcommand ONLY accepts numeric key codes, not string names.
    // Using string names like "ENTER" silently fails — keys never arrive.
    // Key codes from linux/input-event-codes.h
    match key.to_lowercase().as_str() {
        "return" | "enter" => "28".into(),              // KEY_ENTER
        "tab" => "15".into(),                           // KEY_TAB
        "escape" | "esc" => "1".into(),                 // KEY_ESC
        "backspace" => "14".into(),                     // KEY_BACKSPACE
        "delete" | "del" => "111".into(),               // KEY_DELETE
        "up" => "103".into(),                           // KEY_UP
        "down" => "108".into(),                         // KEY_DOWN
        "left" => "105".into(),                         // KEY_LEFT
        "right" => "106".into(),                        // KEY_RIGHT
        "home" => "102".into(),                         // KEY_HOME
        "end" => "107".into(),                          // KEY_END
        "page_up" | "pgup" => "104".into(),             // KEY_PAGEUP
        "page_down" | "pgdn" => "109".into(),           // KEY_PAGEDOWN
        "space" => "57".into(),                         // KEY_SPACE
        "shift" | "shift_l" | "shift_r" => "42".into(), // KEY_LEFTSHIFT
        "ctrl" | "control" | "control_l" | "ctrl_l" => "29".into(), // KEY_LEFTCTRL
        "alt" | "alt_l" => "56".into(),                 // KEY_LEFTALT
        "super" | "super_l" | "meta" | "win" | "windows" => "125".into(), // KEY_LEFTMETA
        other => other.to_string(),
    }
}

/// Simple PNG header parser for dimensions.
pub(super) async fn get_png_dimensions(path: &str) -> anyhow::Result<(u32, u32)> {
    let data = tokio::fs::read(path).await?;
    if data.len() < 24 || &data[1..4] != b"PNG" {
        anyhow::bail!("not a PNG file");
    }
    let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    Ok((width, height))
}
