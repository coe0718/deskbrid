use super::*;
use crate::protocol;

pub(super) async fn system_info(_backend: &CosmicBackend) -> anyhow::Result<protocol::SystemInfo> {
    Ok(protocol::SystemInfo {
        desktop: "COSMIC".to_string(),
        desktop_version: "1.0".to_string(),
        compositor: "cosmic-comp".to_string(),
        session_type: "wayland".to_string(),
        monitors: vec![],
        workspace_count: 1,
        current_workspace: 1,
        idle_seconds: 0,
    })
}

pub(super) async fn idle_seconds(_backend: &CosmicBackend) -> anyhow::Result<u64> {
    // Simple: check /dev/input/event* modification time
    // Fallback to 0
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

// ─── Network ────────────────────────────────────────
