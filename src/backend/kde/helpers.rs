use super::*;

impl KdeBackend {
    pub(super) async fn get_monitors(&self) -> anyhow::Result<Vec<protocol::MonitorInfo>> {
        let out = self.sh("kscreen-doctor", &["--outputs"]).await?;
        Ok(parse_kscreen_outputs(&out))
    }

    pub(super) async fn kscreen_mode_for(
        &self,
        output: &str,
        width: u32,
        height: u32,
    ) -> anyhow::Result<String> {
        let out = self.sh("kscreen-doctor", &["--outputs"]).await?;
        find_kscreen_mode(&out, output, width, height)
            .ok_or_else(|| anyhow::anyhow!("mode not found for {}: {}x{}", output, width, height))
    }
}

pub(super) fn parse_kscreen_outputs(raw: &str) -> Vec<protocol::MonitorInfo> {
    let mut monitors = Vec::new();
    let mut current: Option<protocol::MonitorInfo> = None;

    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Output:") {
            if let Some(monitor) = current.take() {
                monitors.push(monitor);
            }
            let parts: Vec<&str> = rest.split_whitespace().collect();
            let id = parts.first().and_then(|v| v.parse().ok()).unwrap_or(0);
            let name = parts.get(1).copied().unwrap_or("").to_string();
            current = Some(protocol::MonitorInfo {
                id,
                name,
                width: 0,
                height: 0,
                scale: 1.0,
                primary: trimmed.contains("primary") || has_kscreen_primary_priority(trimmed),
                enabled: trimmed.contains(" enabled "),
                x: 0,
                y: 0,
                refresh_rate: None,
                rotation: "normal".into(),
            });
            if let Some(ref mut monitor) = current {
                if let Some(idx) = trimmed.find("Geometry:") {
                    parse_geometry(&trimmed[idx + "Geometry:".len()..], monitor);
                }
                if let Some(idx) = trimmed.find("Modes:") {
                    parse_current_mode(&trimmed[idx..], monitor);
                }
                if let Some(idx) = trimmed.find("Scale:") {
                    let rest = &trimmed[idx + "Scale:".len()..];
                    monitor.scale = rest
                        .split_whitespace()
                        .next()
                        .and_then(|value| value.parse().ok())
                        .unwrap_or(1.0);
                }
            }
            continue;
        }

        let Some(ref mut monitor) = current else {
            continue;
        };

        if trimmed == "enabled" || trimmed.contains(" enabled ") {
            monitor.enabled = true;
        } else if trimmed == "disabled" || trimmed.contains(" disabled ") {
            monitor.enabled = false;
        }
        if trimmed.contains("primary") {
            monitor.primary = true;
        }
        if has_kscreen_primary_priority(trimmed) {
            monitor.primary = true;
        }
        if let Some(geometry) = trimmed.strip_prefix("Geometry:") {
            parse_geometry(geometry.trim(), monitor);
        }
        if let Some(scale) = trimmed.strip_prefix("Scale:") {
            monitor.scale = scale.trim().parse().unwrap_or(1.0);
        }
        if let Some(rotation) = trimmed.strip_prefix("Rotation:") {
            monitor.rotation = kde_rotation_name(rotation.trim()).to_string();
        }
        if trimmed.starts_with("Modes:") {
            parse_current_mode(trimmed, monitor);
        }
    }

    if let Some(monitor) = current.take() {
        monitors.push(monitor);
    }
    monitors
}

pub(super) fn parse_geometry(value: &str, monitor: &mut protocol::MonitorInfo) {
    let mut parts = value.split_whitespace();
    if let Some(pos) = parts.next() {
        let mut xy = pos.split(',');
        monitor.x = xy.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        monitor.y = xy.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    }
    if let Some(size) = parts.next() {
        let mut wh = size.split('x');
        monitor.width = wh.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        monitor.height = wh.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    }
}

pub(super) fn parse_current_mode(value: &str, monitor: &mut protocol::MonitorInfo) {
    let Some(current) = value.split_whitespace().find(|part| part.contains('*')) else {
        return;
    };
    let mode = clean_kscreen_mode_token(current);
    let mut mode_parts = mode.split('@');
    if let Some(size) = mode_parts.next() {
        let mut wh = size.split('x');
        monitor.width = wh
            .next()
            .and_then(|v| v.parse().ok())
            .unwrap_or(monitor.width);
        monitor.height = wh
            .next()
            .and_then(|v| v.parse().ok())
            .unwrap_or(monitor.height);
    }
    if let Some(refresh) = mode_parts.next() {
        monitor.refresh_rate = refresh.parse().ok();
    }
}

pub(super) fn find_kscreen_mode(
    raw: &str,
    output: &str,
    width: u32,
    height: u32,
) -> Option<String> {
    let target = format!("{}x{}@", width, height);
    let mut in_output = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Output:") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            in_output = parts.get(1).copied() == Some(output);
        }
        if !in_output || !trimmed.contains("Modes:") {
            continue;
        }
        for token in trimmed.split_whitespace() {
            if !token.contains(&target) {
                continue;
            }
            let mode = clean_kscreen_mode_token(token);
            if !mode.is_empty() {
                return Some(mode.to_string());
            }
        }
    }
    None
}

pub(super) fn clean_kscreen_mode_token(token: &str) -> &str {
    token
        .split(':')
        .next_back()
        .unwrap_or(token)
        .trim_end_matches(['*', '+', '!'])
}

pub(super) fn has_kscreen_primary_priority(value: &str) -> bool {
    let mut saw_priority = false;
    for token in value.split_whitespace() {
        let normalized = token.trim_matches([':', ',', ';']).to_ascii_lowercase();
        if saw_priority && normalized == "1" {
            return true;
        }
        saw_priority = normalized == "priority";
    }
    false
}

pub(super) fn kde_rotation(rotation: &str) -> anyhow::Result<&'static str> {
    match rotation {
        "normal" => Ok("none"),
        "left" => Ok("left"),
        "right" => Ok("right"),
        "inverted" => Ok("inverted"),
        _ => anyhow::bail!("unsupported monitor rotation: {}", rotation),
    }
}

pub(super) fn kde_rotation_name(rotation: &str) -> &'static str {
    match rotation {
        "2" | "left" => "left",
        "4" | "right" => "right",
        "8" | "inverted" => "inverted",
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
