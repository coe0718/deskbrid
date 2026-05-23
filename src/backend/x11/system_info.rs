use super::*;
use crate::protocol;

pub(super) async fn system_info(backend: &X11Backend) -> anyhow::Result<protocol::SystemInfo> {
    let workspace_count = workspace_count(backend).await.unwrap_or(1);
    let current_workspace = current_workspace(backend).await.unwrap_or(0);
    let idle_seconds = idle_seconds(backend).await.unwrap_or(0);

    Ok(protocol::SystemInfo {
        desktop: backend.detected_de.clone(),
        desktop_version: "unknown".into(),
        compositor: "x11".into(),
        session_type: "x11".into(),
        monitors: backend.xrandr_monitors().await.unwrap_or_else(|_| {
            vec![protocol::MonitorInfo {
                id: 0,
                name: "X11".into(),
                width: 1920,
                height: 1080,
                scale: 1.0,
                primary: true,
                enabled: true,
                x: 0,
                y: 0,
                refresh_rate: None,
                rotation: "normal".into(),
            }]
        }),
        workspace_count,
        current_workspace,
        idle_seconds,
    })
}

pub(super) async fn idle_seconds(backend: &X11Backend) -> anyhow::Result<u64> {
    let output = backend.sh("xprintidle", &[]).await?;
    let millis = output.trim().parse::<u64>()?;
    Ok(millis / 1000)
}

pub(super) async fn power_action(backend: &X11Backend, action: &str) -> anyhow::Result<()> {
    match action {
        "suspend" => backend.sh("systemctl", &["suspend"]).await.map(|_| ()),
        "shutdown" => backend.sh("systemctl", &["poweroff"]).await.map(|_| ()),
        "reboot" => backend.sh("systemctl", &["reboot"]).await.map(|_| ()),
        "lock" => backend.sh("loginctl", &["lock-session"]).await.map(|_| ()),
        _ => anyhow::bail!("unsupported power action: {}", action),
    }
}

pub(super) async fn battery_status(
    _backend: &X11Backend,
) -> anyhow::Result<Vec<protocol::BatteryInfo>> {
    let mut batteries = Vec::new();
    let mut entries = match tokio::fs::read_dir("/sys/class/power_supply").await {
        Ok(entries) => entries,
        Err(_) => return Ok(batteries),
    };

    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("BAT") {
            continue;
        }
        let base = entry.path();
        let percentage = tokio::fs::read_to_string(base.join("capacity"))
            .await
            .unwrap_or_default()
            .trim()
            .parse::<f64>()
            .unwrap_or(0.0);
        let state = tokio::fs::read_to_string(base.join("status"))
            .await
            .unwrap_or_default()
            .trim()
            .to_string();
        batteries.push(protocol::BatteryInfo {
            source: name,
            percentage,
            state,
            time_remaining_minutes: None,
        });
    }

    Ok(batteries)
}

async fn workspace_count(backend: &X11Backend) -> anyhow::Result<u32> {
    Ok(backend
        .sh("xdotool", &["get_num_desktops"])
        .await?
        .trim()
        .parse()
        .unwrap_or(1))
}

async fn current_workspace(backend: &X11Backend) -> anyhow::Result<u32> {
    Ok(backend
        .sh("xdotool", &["get_desktop"])
        .await?
        .trim()
        .parse()
        .unwrap_or(0))
}
