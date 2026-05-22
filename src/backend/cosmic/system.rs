use super::*;
use crate::protocol;

pub(super) async fn screenshot(
    backend: &CosmicBackend,
    _monitor: Option<u32>,
    region: Option<protocol::Region>,
    _window_id: Option<String>,
) -> anyhow::Result<protocol::ScreenshotResult> {
    let path = format!(
        "/tmp/deskbrid/screenshot_{}.png",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );

    // Ensure tmp dir exists
    let _ = tokio::fs::create_dir_all("/tmp/deskbrid").await;

    if let Some(r) = region {
        let geo = format!("{}x{}+{}+{}", r.width, r.height, r.x, r.y);
        backend.sh("grim", &["-g", &geo, &path]).await?;
    } else {
        backend.sh("grim", &[&path]).await?;
    }

    // Get dimensions from the file
    let dims_output = backend.sh("identify", &["-format", "%w %h", &path]).await?;
    let dims: Vec<&str> = dims_output.split_whitespace().collect();
    let width = dims.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let height = dims.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);

    Ok(protocol::ScreenshotResult {
        path,
        width,
        height,
        format: "png".to_string(),
    })
}

// ─── Notifications ──────────────────────────────────

pub(super) async fn notification_send(
    backend: &CosmicBackend,
    _app_name: &str,
    title: &str,
    body: &str,
    urgency: &str,
) -> anyhow::Result<u32> {
    let u = match urgency {
        "low" => "low",
        "critical" => "critical",
        _ => "normal",
    };
    backend.sh("notify-send", &["-u", u, title, body]).await?;
    // notify-send doesn't return an ID; return 0
    Ok(0)
}

pub(super) async fn notification_close(_backend: &CosmicBackend, _id: u32) -> anyhow::Result<()> {
    // notify-send doesn't support close by ID
    Ok(())
}

// ─── System ─────────────────────────────────────────

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

pub(super) async fn network_status(
    backend: &CosmicBackend,
) -> anyhow::Result<protocol::NetworkStatusInfo> {
    // Reuse nmcli
    let output = backend
        .sh("nmcli", &["-t", "-f", "STATE", "general"])
        .await?;
    let connected = output.trim().starts_with("connected");
    Ok(protocol::NetworkStatusInfo {
        online: connected,
        net_type: if connected { "ethernet" } else { "none" }.to_string(),
    })
}

pub(super) async fn network_interfaces(
    backend: &CosmicBackend,
) -> anyhow::Result<Vec<protocol::NetworkInterfaceInfo>> {
    let output = backend
        .sh(
            "nmcli",
            &[
                "-t",
                "-f",
                "NAME,TYPE,DEVICE,STATE",
                "connection",
                "show",
                "--active",
            ],
        )
        .await?;
    let interfaces = output
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() < 4 {
                return None;
            }
            Some(protocol::NetworkInterfaceInfo {
                name: parts[0].to_string(),
                state: parts[3].to_string(),
                ipv4: None,
                ipv6: None,
            })
        })
        .collect();
    Ok(interfaces)
}

pub(super) async fn wifi_scan(
    backend: &CosmicBackend,
) -> anyhow::Result<Vec<protocol::WifiNetworkInfo>> {
    let output = backend
        .sh(
            "nmcli",
            &[
                "-t",
                "-f",
                "SSID,BSSID,SIGNAL,SECURITY",
                "device",
                "wifi",
                "list",
            ],
        )
        .await?;
    let networks = output
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() < 4 {
                return None;
            }
            Some(protocol::WifiNetworkInfo {
                ssid: parts[0].to_string(),
                strength: parts[2].parse().unwrap_or(0),
                secured: !parts[3].is_empty(),
                frequency: None,
            })
        })
        .collect();
    Ok(networks)
}

pub(super) async fn wifi_connect(
    backend: &CosmicBackend,
    ssid: &str,
    password: Option<&str>,
) -> anyhow::Result<()> {
    if let Some(pwd) = password {
        backend
            .sh(
                "nmcli",
                &["device", "wifi", "connect", ssid, "password", pwd],
            )
            .await?;
    } else {
        backend
            .sh("nmcli", &["device", "wifi", "connect", ssid])
            .await?;
    }
    Ok(())
}

// ─── Bluetooth ─────────────────────────────────────

pub(super) async fn bluetooth_list(
    _backend: &CosmicBackend,
) -> anyhow::Result<Vec<protocol::BluetoothDeviceInfo>> {
    Ok(vec![])
}

pub(super) async fn bluetooth_scan(
    _backend: &CosmicBackend,
    _duration: Option<u32>,
) -> anyhow::Result<()> {
    Ok(())
}

pub(super) async fn bluetooth_stop_scan(_backend: &CosmicBackend) -> anyhow::Result<()> {
    Ok(())
}

pub(super) async fn bluetooth_connect(
    _backend: &CosmicBackend,
    _address: &str,
) -> anyhow::Result<()> {
    Ok(())
}

pub(super) async fn bluetooth_disconnect(
    _backend: &CosmicBackend,
    _address: &str,
) -> anyhow::Result<()> {
    Ok(())
}

// ─── Files ──────────────────────────────────────────

pub(super) async fn files_watch(
    _backend: &CosmicBackend,
    _path: &str,
    _recursive: bool,
    _patterns: Option<&[String]>,
) -> anyhow::Result<()> {
    Ok(())
}

pub(super) async fn files_unwatch(_backend: &CosmicBackend, _path: &str) -> anyhow::Result<()> {
    Ok(())
}

pub(super) async fn files_search(
    backend: &CosmicBackend,
    pattern: &str,
    _root: Option<&str>,
    max_results: u32,
) -> anyhow::Result<Vec<String>> {
    // Reuse `find` like the other backends
    let output = backend
        .sh(
            "find",
            &[".", "-iname", &format!("*{}*", pattern), "-type", "f"],
        )
        .await?;
    let results: Vec<String> = output
        .lines()
        .take(max_results as usize)
        .map(String::from)
        .collect();
    Ok(results)
}

// ─── Audio ──────────────────────────────────────────

pub(super) async fn audio_list_sinks(
    _backend: &CosmicBackend,
) -> anyhow::Result<Vec<protocol::AudioSinkInfo>> {
    Ok(vec![])
}

pub(super) async fn audio_set_sink_volume(
    _backend: &CosmicBackend,
    _sink_id: u32,
    _volume: f64,
) -> anyhow::Result<()> {
    Ok(())
}

// ═══════════════════════════════════════════════════════
// MONITOR (via cosmic-randr)
// ═══════════════════════════════════════════════════════

pub(super) async fn monitor_set_primary(
    backend: &CosmicBackend,
    output: &str,
) -> anyhow::Result<()> {
    // cosmic-randr has no "primary" concept — Wayland doesn't use it.
    // Use xwayland-primary as the closest equivalent.
    backend.sh("cosmic-randr", &["xwayland", output]).await?;
    Ok(())
}

pub(super) async fn monitor_set_resolution(
    backend: &CosmicBackend,
    output: &str,
    width: u32,
    height: u32,
    refresh_rate: Option<f64>,
) -> anyhow::Result<()> {
    let mut args = vec![
        "mode".to_string(),
        output.to_string(),
        width.to_string(),
        height.to_string(),
    ];
    if let Some(refresh) = refresh_rate {
        args.push("--refresh".to_string());
        args.push(format_monitor_float(refresh));
    }
    backend.sh_owned("cosmic-randr", args).await?;
    Ok(())
}

pub(super) async fn monitor_set_scale(
    backend: &CosmicBackend,
    output: &str,
    scale: f64,
) -> anyhow::Result<()> {
    // cosmic-randr mode --scale <value> — requires width+height too,
    // so we first list the current mode to preserve it.
    let list = backend
        .helper_json(&["list-monitors"])
        .await
        .unwrap_or_default();
    let current_w = list.get("width").and_then(|v| v.as_u64()).unwrap_or(1920) as u32;
    let current_h = list.get("height").and_then(|v| v.as_u64()).unwrap_or(1080) as u32;

    backend
        .sh_owned(
            "cosmic-randr",
            vec![
                "mode".to_string(),
                output.to_string(),
                current_w.to_string(),
                current_h.to_string(),
                "--scale".to_string(),
                format_monitor_float(scale),
            ],
        )
        .await?;
    Ok(())
}

pub(super) async fn monitor_set_rotation(
    backend: &CosmicBackend,
    output: &str,
    rotation: &str,
) -> anyhow::Result<()> {
    let transform = cosmic_transform(rotation)?;
    // cosmic-randr mode --transform <value> — needs width+height
    let list = backend
        .helper_json(&["list-monitors"])
        .await
        .unwrap_or_default();
    let current_w = list.get("width").and_then(|v| v.as_u64()).unwrap_or(1920) as u32;
    let current_h = list.get("height").and_then(|v| v.as_u64()).unwrap_or(1080) as u32;

    backend
        .sh_owned(
            "cosmic-randr",
            vec![
                "mode".to_string(),
                output.to_string(),
                current_w.to_string(),
                current_h.to_string(),
                "--transform".to_string(),
                transform.to_string(),
            ],
        )
        .await?;
    Ok(())
}

pub(super) async fn monitor_set_enabled(
    backend: &CosmicBackend,
    output: &str,
    enabled: bool,
) -> anyhow::Result<()> {
    let subcmd = if enabled { "enable" } else { "disable" };
    backend.sh("cosmic-randr", &[subcmd, output]).await?;
    Ok(())
}
