use super::*;
use crate::protocol;

pub(super) async fn system_info(backend: &HyprBackend) -> anyhow::Result<protocol::SystemInfo> {
    let version = backend
        .hyprctl_json(&["version"])
        .await
        .map(|v| {
            v.get("version")
                .and_then(|s| s.as_str())
                .unwrap_or("unknown")
                .to_string()
        })
        .unwrap_or_else(|_| "unknown".into());
    let session_type = if backend.wl_socket.is_some() {
        "wayland"
    } else if std::env::var("DISPLAY").is_ok() {
        "x11"
    } else {
        "unknown"
    };
    let monitors = {
        let m = backend.monitors.lock().unwrap();
        m.clone()
    };
    let workspaces = workspace::workspaces_list(backend)
        .await
        .unwrap_or_default();
    let current_workspace = workspaces
        .iter()
        .find(|w| w.is_active)
        .map(|w| w.id)
        .unwrap_or(1);
    Ok(protocol::SystemInfo {
        desktop: "Hyprland".into(),
        desktop_version: version,
        compositor: "hyprland".into(),
        session_type: session_type.into(),
        monitors,
        workspace_count: workspaces.len() as u32,
        current_workspace,
        idle_seconds: backend.idle_seconds_inner().await.unwrap_or(0),
    })
}

pub(super) async fn idle_seconds(backend: &HyprBackend) -> anyhow::Result<u64> {
    backend.idle_seconds_inner().await
}

pub(super) async fn power_action(backend: &HyprBackend, action: &str) -> anyhow::Result<()> {
    match action {
        "suspend" => {
            backend.sh("systemctl", &["suspend"]).await?;
        }
        "hibernate" => {
            backend.sh("systemctl", &["hibernate"]).await?;
        }
        "shutdown" | "poweroff" => {
            backend.sh("systemctl", &["poweroff"]).await?;
        }
        "reboot" | "restart" => {
            backend.sh("systemctl", &["reboot"]).await?;
        }
        "lock" => {
            if !backend.sh_ok("loginctl", &["lock-session"]).await {
                backend
                    .sh("hyprctl", &["dispatch", "exec", "loginctl lock-session"])
                    .await?;
            }
        }
        "logout" => {
            backend.sh("hyprctl", &["dispatch", "exit"]).await?;
        }
        _ => anyhow::bail!("unsupported power action: {}", action),
    }
    Ok(())
}

pub(super) async fn battery_status(
    _backend: &HyprBackend,
) -> anyhow::Result<Vec<protocol::BatteryInfo>> {
    let mut batteries = Vec::new();
    let mut dirs = if let Ok(entries) = tokio::fs::read_dir("/sys/class/power_supply").await {
        entries
    } else {
        return Ok(batteries);
    };
    while let Some(entry) = dirs.next_entry().await? {
        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if !name.starts_with("BAT") {
            continue;
        }
        let capacity = tokio::fs::read_to_string(path.join("capacity"))
            .await
            .ok()
            .and_then(|s| s.trim().parse::<f64>().ok())
            .unwrap_or(0.0);
        let status = tokio::fs::read_to_string(path.join("status"))
            .await
            .ok()
            .map(|s| s.trim().to_lowercase())
            .unwrap_or_else(|| "unknown".into());
        let energy_now = tokio::fs::read_to_string(path.join("energy_now"))
            .await
            .ok()
            .and_then(|s| s.trim().parse::<f64>().ok())
            .unwrap_or(0.0);
        let power_now = tokio::fs::read_to_string(path.join("power_now"))
            .await
            .ok()
            .and_then(|s| s.trim().parse::<f64>().ok())
            .unwrap_or(0.0);
        let time_remaining = if power_now > 0.0 {
            Some(((energy_now / power_now) * 60.0) as u32)
        } else {
            None
        };
        batteries.push(protocol::BatteryInfo {
            source: name.to_string(),
            percentage: capacity,
            state: status,
            time_remaining_minutes: time_remaining,
        });
    }
    Ok(batteries)
}
