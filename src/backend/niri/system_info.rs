use super::*;
use crate::protocol;

pub(super) async fn system_info(backend: &NiriBackend) -> anyhow::Result<protocol::SystemInfo> {
    let version = backend.sh("niri", &["--version"]).await.unwrap_or_default();
    let workspaces = backend
        .niri_json(&["workspaces"])
        .await
        .map(|v| parse_niri_workspaces(&v))
        .unwrap_or_default();
    let current_ws = workspaces
        .iter()
        .find(|w| w.is_active)
        .map(|w| w.id)
        .unwrap_or(0);
    let idle = backend.idle_seconds().await.unwrap_or(0);
    let monitors = backend
        .niri_json(&["outputs"])
        .await
        .map(|v| parse_niri_outputs(&v))
        .unwrap_or_default();

    Ok(protocol::SystemInfo {
        desktop: "Niri".into(),
        desktop_version: version.trim().to_string(),
        compositor: format!("niri {}", version.trim()),
        session_type: "wayland".into(),
        monitors,
        workspace_count: workspaces.len() as u32,
        current_workspace: current_ws,
        idle_seconds: idle,
    })
}

pub(super) async fn idle_seconds(_backend: &NiriBackend) -> anyhow::Result<u64> {
    let out = Command::new("sh")
        .arg("-c")
        .arg("find /dev/input -name 'event*' -printf '%T@\n' 2>/dev/null | sort -rn | head -1")
        .output()
        .await?;
    let latest: f64 = String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse()
        .unwrap_or(0.0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();
    Ok((now - latest) as u64)
}

pub(super) async fn power_action(backend: &NiriBackend, action: &str) -> anyhow::Result<()> {
    match action {
        "suspend" => backend.sh("systemctl", &["suspend"]).await.map(|_| ()),
        "shutdown" => backend.sh("systemctl", &["poweroff"]).await.map(|_| ()),
        "reboot" => backend.sh("systemctl", &["reboot"]).await.map(|_| ()),
        "lock" => backend.sh("loginctl", &["lock-session"]).await.map(|_| ()),
        _ => anyhow::bail!("unsupported power action: {}", action),
    }
}

pub(super) async fn battery_status(
    _backend: &NiriBackend,
) -> anyhow::Result<Vec<protocol::BatteryInfo>> {
    let mut batteries = Vec::new();
    for i in 0..5 {
        let base = format!("/sys/class/power_supply/BAT{}", i);
        let cap_path = format!("{}/capacity", base);
        let stat_path = format!("{}/status", base);
        if let Ok(cap) = tokio::fs::read_to_string(&cap_path).await {
            let percentage: f64 = cap.trim().parse().unwrap_or(0.0);
            let state = tokio::fs::read_to_string(&stat_path)
                .await
                .unwrap_or_default()
                .trim()
                .to_string();
            batteries.push(protocol::BatteryInfo {
                source: format!("BAT{}", i),
                percentage,
                state,
                time_remaining_minutes: None,
            });
        }
    }
    Ok(batteries)
}
