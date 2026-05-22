use super::*;
use crate::protocol;

pub(super) async fn system_info(backend: &WayfireBackend) -> anyhow::Result<protocol::SystemInfo> {
    let ver = backend
        .sh("wayfire", &["--version"])
        .await
        .unwrap_or_default();
    let monitors = backend
        .wf_ipc_json(&["list-outputs", "-j"])
        .await
        .map(|v| parse_wayfire_outputs(&v))
        .unwrap_or_default();
    let idle = backend.idle_seconds().await.unwrap_or(0);
    Ok(protocol::SystemInfo {
        desktop: "Wayfire".into(),
        desktop_version: ver.trim().to_string(),
        compositor: format!("wayfire {}", ver.trim()),
        session_type: "wayland".into(),
        monitors,
        workspace_count: 1,
        current_workspace: 1,
        idle_seconds: idle,
    })
}

pub(super) async fn idle_seconds(_backend: &WayfireBackend) -> anyhow::Result<u64> {
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

pub(super) async fn power_action(backend: &WayfireBackend, action: &str) -> anyhow::Result<()> {
    match action {
        "suspend" => backend.sh("systemctl", &["suspend"]).await.map(|_| ()),
        "shutdown" => backend.sh("systemctl", &["poweroff"]).await.map(|_| ()),
        "reboot" => backend.sh("systemctl", &["reboot"]).await.map(|_| ()),
        "lock" => backend.sh("loginctl", &["lock-session"]).await.map(|_| ()),
        _ => anyhow::bail!("unsupported power action: {}", action),
    }
}

pub(super) async fn battery_status(
    _backend: &WayfireBackend,
) -> anyhow::Result<Vec<protocol::BatteryInfo>> {
    let mut bats = Vec::new();
    for i in 0..5 {
        let b = format!("/sys/class/power_supply/BAT{}", i);
        if let Ok(cap) = tokio::fs::read_to_string(&format!("{}/capacity", b)).await {
            let pct: f64 = cap.trim().parse().unwrap_or(0.0);
            let st = tokio::fs::read_to_string(&format!("{}/status", b))
                .await
                .unwrap_or_default()
                .trim()
                .to_string();
            bats.push(protocol::BatteryInfo {
                source: format!("BAT{}", i),
                percentage: pct,
                state: st,
                time_remaining_minutes: None,
            });
        }
    }
    Ok(bats)
}
