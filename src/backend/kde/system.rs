use super::*;
use crate::protocol;

pub(super) async fn system_info(backend: &KdeBackend) -> anyhow::Result<protocol::SystemInfo> {
    let _hostname = backend.sh("hostname", &[]).await.unwrap_or_default();
    let _kernel = backend.sh("uname", &["-r"]).await.unwrap_or_default();

    let version = backend
        .qdbus(
            "org.kde.KWin",
            "/KWin",
            "org.kde.KWin.supportInformation",
            &[],
        )
        .await
        .unwrap_or_default();
    let first_line = version.lines().next().unwrap_or("KDE Plasma 6").to_string();

    Ok(protocol::SystemInfo {
        desktop: "KDE Plasma".into(),
        desktop_version: first_line,
        compositor: "KWin".into(),
        session_type: "wayland".into(),
        monitors: backend.get_monitors().await.unwrap_or_default(),
        workspace_count: 1,
        current_workspace: 0,
        idle_seconds: 0,
    })
}

pub(super) async fn idle_seconds(backend: &KdeBackend) -> anyhow::Result<u64> {
    let out = backend
        .sh("loginctl", &["show-session", "auto", "-p", "IdleSinceHint"])
        .await?;
    if let Some(val) = out.strip_prefix("IdleSinceHint=") {
        let micros: u64 = val.trim().parse().unwrap_or(0);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;
        if micros > 0 && now > micros {
            return Ok((now - micros) / 1_000_000);
        }
    }
    Ok(0)
}

pub(super) async fn power_action(backend: &KdeBackend, action: &str) -> anyhow::Result<()> {
    match action {
        "suspend" => backend.sh("systemctl", &["suspend"]).await?,
        "hibernate" => backend.sh("systemctl", &["hibernate"]).await?,
        "shutdown" => backend.sh("systemctl", &["poweroff"]).await?,
        "reboot" => backend.sh("systemctl", &["reboot"]).await?,
        "lock" => backend.sh("loginctl", &["lock-session"]).await?,
        "logout" => {
            backend
                .qdbus(
                    "org.kde.ksmserver",
                    "/KSMServer",
                    "org.kde.KSMServerInterface.logout",
                    &["0", "0", "0"],
                )
                .await?
        }
        _ => anyhow::bail!("unknown power action: {action}"),
    };
    Ok(())
}

pub(super) async fn battery_status(
    backend: &KdeBackend,
) -> anyhow::Result<Vec<protocol::BatteryInfo>> {
    let out = backend.sh("upower", &["-e"]).await?;
    let mut batteries = Vec::new();
    for line in out.lines() {
        if line.contains("battery") {
            let info = backend
                .sh("upower", &["-i", line.trim()])
                .await
                .unwrap_or_default();
            let pct = info
                .lines()
                .find(|l| l.trim().starts_with("percentage:"))
                .and_then(|l| l.split(':').nth(1))
                .and_then(|s| s.trim().trim_end_matches('%').parse::<f64>().ok())
                .unwrap_or(0.0);
            let state = info
                .lines()
                .find(|l| l.trim().starts_with("state:"))
                .and_then(|l| l.split(':').nth(1))
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            batteries.push(protocol::BatteryInfo {
                source: line.trim().to_string(),
                percentage: pct,
                state,
                time_remaining_minutes: None,
            });
        }
    }
    Ok(batteries)
}
