use super::*;
use crate::protocol;

/// Parse `cosmic-randr list` output.
fn parse_cosmic_randr(raw: &str) -> Vec<protocol::MonitorInfo> {
    let raw = crate::util::strip_ansi(raw);
    let mut monitors = Vec::new();
    let mut current: Option<protocol::MonitorInfo> = None;

    for line in raw.lines() {
        let trimmed = line.trim();
        // New output header: "eDP-1 (enabled)" or "eDP-1 (disabled)"
        if trimmed.ends_with("(enabled)") || trimmed.ends_with("(disabled)") {
            if let Some(m) = current.take() {
                monitors.push(m);
            }
            let enabled = trimmed.ends_with("(enabled)");
            let name = trimmed
                .trim_end_matches(" (enabled)")
                .trim_end_matches(" (disabled)")
                .to_string();
            current = Some(protocol::MonitorInfo {
                id: monitors.len() as u32,
                name,
                width: 0,
                height: 0,
                scale: 1.0,
                primary: false,
                enabled,
                x: 0,
                y: 0,
                refresh_rate: None,
                rotation: "normal".into(),
            });
            continue;
        }

        let Some(ref mut mon) = current else {
            continue;
        };

        if let Some(val) = trimmed.strip_prefix("Position: ") {
            let mut parts = val.split(',');
            mon.x = parts.next().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
            mon.y = parts.next().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
        } else if let Some(val) = trimmed.strip_prefix("Scale: ") {
            mon.scale = val
                .trim_end_matches('%')
                .parse::<f64>()
                .map(|p| p / 100.0)
                .unwrap_or(1.0);
        } else if let Some(val) = trimmed.strip_prefix("Transform: ") {
            mon.rotation = match val.trim() {
                "normal" | "Normal" => "normal",
                "rotate90" | "90" => "left",
                "rotate180" | "180" => "inverted",
                "rotate270" | "270" => "right",
                "flipped" | "flipped-90" | "flipped-180" | "flipped-270" => "normal",
                _ => "normal",
            }
            .to_string();
        } else if trimmed.starts_with("Xwayland primary: true") {
            mon.primary = true;
        } else if trimmed.contains("(current)") && trimmed.contains('@') {
            // Mode line: "  1366x768 @  60.026 Hz (current) (preferred)"
            let clean = trimmed
                .trim_end_matches(" (preferred)")
                .trim_end_matches(" (current)");
            if let Some(size_part) = clean.split('@').next() {
                let size = size_part.trim();
                if let Some(x_pos) = size.find('x') {
                    mon.width = size[..x_pos].parse().unwrap_or(0);
                    mon.height = size[x_pos + 1..].parse().unwrap_or(0);
                }
            }
            if let Some(refresh_part) = clean.split('@').nth(1) {
                mon.refresh_rate = refresh_part
                    .trim()
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse().ok());
            }
        }
    }

    if let Some(m) = current.take() {
        monitors.push(m);
    }

    // Set primary on first monitor if none marked
    if !monitors.is_empty() && !monitors.iter().any(|m| m.primary) {
        monitors[0].primary = true;
    }

    monitors
}

pub(super) async fn system_info(backend: &CosmicBackend) -> anyhow::Result<protocol::SystemInfo> {
    let monitors = backend
        .sh("cosmic-randr", &["list"])
        .await
        .map(|raw| parse_cosmic_randr(&raw))
        .unwrap_or_default();

    // Count workspaces from cosmic-helper
    let ws_json = backend
        .helper_json(&["workspace-list"])
        .await
        .unwrap_or_default();
    let ws_count = ws_json
        .as_array()
        .map(|a| a.len() as u32)
        .unwrap_or(1);

    Ok(protocol::SystemInfo {
        desktop: "COSMIC".to_string(),
        desktop_version: "1.0".to_string(),
        compositor: "cosmic-comp".to_string(),
        session_type: "wayland".to_string(),
        monitors,
        workspace_count: ws_count,
        current_workspace: 1,
        idle_seconds: 0,
    })
}

pub(super) async fn idle_seconds(_backend: &CosmicBackend) -> anyhow::Result<u64> {
    Ok(0)
}

pub(super) async fn power_action(backend: &CosmicBackend, action: &str) -> anyhow::Result<()> {
    match action {
        "suspend" | "sleep" => backend.sh("systemctl", &["suspend"]).await.map(|_| ()),
        "hibernate" => backend.sh("systemctl", &["hibernate"]).await.map(|_| ()),
        "poweroff" | "shutdown" => backend.sh("systemctl", &["poweroff"]).await.map(|_| ()),
        "reboot" => backend.sh("systemctl", &["reboot"]).await.map(|_| ()),
        "lock" => backend.sh("loginctl", &["lock-session"]).await.map(|_| ()),
        _ => anyhow::bail!("unknown power action: {}", action),
    }
}

pub(super) async fn battery_status(
    _backend: &CosmicBackend,
) -> anyhow::Result<Vec<protocol::BatteryInfo>> {
    let mut batteries = Vec::new();
    let mut entries = match tokio::fs::read_dir("/sys/class/power_supply/").await {
        Ok(entries) => entries,
        Err(_) => return Ok(batteries),
    };

    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("BAT") {
            continue;
        }
        let base = entry.path();
        let capacity = tokio::fs::read_to_string(base.join("capacity"))
            .await
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);
        let status = tokio::fs::read_to_string(base.join("status"))
            .await
            .unwrap_or_default()
            .trim()
            .to_string();
        batteries.push(protocol::BatteryInfo {
            source: name,
            percentage: capacity as f64 / 100.0,
            state: status,
            time_remaining_minutes: None,
        });
    }
    Ok(batteries)
}
