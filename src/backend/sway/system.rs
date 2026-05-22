use super::*;
use crate::protocol;

pub(super) async fn screenshot(
    backend: &SwayBackend,
    monitor: Option<u32>,
    region: Option<protocol::Region>,
    _window_id: Option<String>,
) -> anyhow::Result<protocol::ScreenshotResult> {
    let path = format!(
        "/tmp/deskbrid_screenshot_{}.png",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs()
    );

    let output_name = if let Some(monitor_id) = monitor {
        let outputs = backend.swaymsg_json(&["-t", "get_outputs"]).await?;
        let monitors = parse_sway_outputs(&outputs);
        monitors
            .get(monitor_id as usize)
            .map(|m| m.name.clone())
            .unwrap_or_default()
    } else {
        let outputs = backend.swaymsg_json(&["-t", "get_outputs"]).await?;
        let monitors = parse_sway_outputs(&outputs);
        monitors
            .iter()
            .find(|m| m.primary)
            .map(|m| m.name.clone())
            .unwrap_or_default()
    };

    let mut grim_args: Vec<String> = vec!["-t".into(), "png".into()];
    grim_args.push(format!("-o{}", output_name));
    if let Some(region) = region {
        grim_args.push("-g".into());
        grim_args.push(format!(
            "{},{} {}x{}",
            region.x, region.y, region.width, region.height
        ));
    }
    grim_args.push(path.clone());

    let mut cmd = Command::new("grim");
    cmd.args(&grim_args)
        .stdin(Stdio::null())
        .stderr(Stdio::piped());
    backend.apply_env(&mut cmd);
    let out = cmd.output().await?;
    if !out.status.success() {
        anyhow::bail!(
            "grim failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }

    let dims = backend
        .sh("identify", &["-format", "%w %h", &path])
        .await
        .ok();
    let (width, height) = if let Some(ref dim) = dims {
        let parts: Vec<&str> = dim.split_whitespace().collect();
        (
            parts.first().and_then(|s| s.parse().ok()).unwrap_or(0),
            parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0),
        )
    } else {
        (0, 0)
    };

    Ok(protocol::ScreenshotResult {
        path,
        width,
        height,
        format: "png".into(),
    })
}

// ─── Notifications ────────────────────────────────

pub(super) async fn notification_send(
    backend: &SwayBackend,
    app_name: &str,
    title: &str,
    body: &str,
    urgency: &str,
) -> anyhow::Result<u32> {
    let out = backend
        .sh(
            "notify-send",
            &["-a", app_name, "-u", urgency, "--print-id", title, body],
        )
        .await?;
    Ok(out.parse().unwrap_or(0))
}

pub(super) async fn notification_close(backend: &SwayBackend, id: u32) -> anyhow::Result<()> {
    backend
        .sh("makoctl", &["dismiss", "-n", &id.to_string()])
        .await
        .map(|_| ())
}

// ─── System ───────────────────────────────────────

pub(super) async fn system_info(backend: &SwayBackend) -> anyhow::Result<protocol::SystemInfo> {
    let version = backend
        .sh("swaymsg", &["-t", "get_version"])
        .await
        .unwrap_or_default();
    let monitors = backend
        .swaymsg_json(&["-t", "get_outputs"])
        .await
        .map(|v| parse_sway_outputs(&v))
        .unwrap_or_default();
    let workspaces = backend
        .swaymsg_json(&["-t", "get_workspaces"])
        .await
        .map(|v| parse_sway_workspaces(&v))
        .unwrap_or_default();
    let current_ws = workspaces
        .iter()
        .find(|w| w.is_active)
        .map(|w| w.id)
        .unwrap_or(0);
    let idle = backend.idle_seconds().await.unwrap_or(0);

    Ok(protocol::SystemInfo {
        desktop: "Sway".into(),
        desktop_version: version.trim().to_string(),
        compositor: format!("sway {}", version.trim()),
        session_type: "wayland".into(),
        monitors,
        workspace_count: workspaces.len() as u32,
        current_workspace: current_ws,
        idle_seconds: idle,
    })
}

pub(super) async fn idle_seconds(_backend: &SwayBackend) -> anyhow::Result<u64> {
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

pub(super) async fn power_action(backend: &SwayBackend, action: &str) -> anyhow::Result<()> {
    match action {
        "suspend" => backend.sh("systemctl", &["suspend"]).await.map(|_| ()),
        "shutdown" => backend.sh("systemctl", &["poweroff"]).await.map(|_| ()),
        "reboot" => backend.sh("systemctl", &["reboot"]).await.map(|_| ()),
        "lock" => backend.sh("loginctl", &["lock-session"]).await.map(|_| ()),
        _ => anyhow::bail!("unsupported power action: {}", action),
    }
}

pub(super) async fn battery_status(
    _backend: &SwayBackend,
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

// ─── Network ──────────────────────────────────────

pub(super) async fn network_status(
    backend: &SwayBackend,
) -> anyhow::Result<protocol::NetworkStatusInfo> {
    let out = backend
        .sh("nmcli", &["-t", "-f", "STATE", "general"])
        .await?;
    let online = out.to_lowercase().contains("connected");
    Ok(protocol::NetworkStatusInfo {
        online,
        net_type: String::new(),
    })
}

pub(super) async fn network_interfaces(
    backend: &SwayBackend,
) -> anyhow::Result<Vec<protocol::NetworkInterfaceInfo>> {
    let out = backend
        .sh("nmcli", &["-t", "-f", "DEVICE,TYPE,STATE", "device"])
        .await?;
    Ok(out
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 {
                Some(protocol::NetworkInterfaceInfo {
                    name: parts[0].to_string(),
                    state: parts.get(1).unwrap_or(&"").to_string(),
                    ipv4: None,
                    ipv6: None,
                })
            } else {
                None
            }
        })
        .collect())
}

pub(super) async fn wifi_scan(
    backend: &SwayBackend,
) -> anyhow::Result<Vec<protocol::WifiNetworkInfo>> {
    let _ = backend.sh("nmcli", &["device", "wifi", "rescan"]).await;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    let out = backend
        .sh(
            "nmcli",
            &["-t", "-f", "SSID,SIGNAL,SECURITY", "device", "wifi", "list"],
        )
        .await?;
    Ok(out
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 && !parts[0].is_empty() {
                Some(protocol::WifiNetworkInfo {
                    ssid: parts[0].to_string(),
                    strength: parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0),
                    secured: parts
                        .get(2)
                        .map(|s| !s.is_empty() && s != &"")
                        .unwrap_or(false),
                    frequency: None,
                })
            } else {
                None
            }
        })
        .collect())
}

pub(super) async fn wifi_connect(
    backend: &SwayBackend,
    ssid: &str,
    password: Option<&str>,
) -> anyhow::Result<()> {
    let mut args = vec!["device", "wifi", "connect", ssid];
    if let Some(pw) = password {
        args.push("password");
        args.push(pw);
    }
    backend.sh("nmcli", &args).await.map(|_| ())
}

// ─── Bluetooth ────────────────────────────────────

pub(super) async fn bluetooth_list(
    backend: &SwayBackend,
) -> anyhow::Result<Vec<protocol::BluetoothDeviceInfo>> {
    let out = backend.sh("bluetoothctl", &["devices"]).await?;
    Ok(out
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            if parts.len() >= 3 {
                Some(protocol::BluetoothDeviceInfo {
                    address: parts[1].to_string(),
                    name: parts[2].to_string(),
                    paired: true,
                    connected: false,
                    rssi: None,
                })
            } else {
                None
            }
        })
        .collect())
}

pub(super) async fn bluetooth_scan(
    backend: &SwayBackend,
    _duration: Option<u32>,
) -> anyhow::Result<()> {
    backend
        .sh("bluetoothctl", &["scan", "on"])
        .await
        .map(|_| ())
}

pub(super) async fn bluetooth_stop_scan(backend: &SwayBackend) -> anyhow::Result<()> {
    backend
        .sh("bluetoothctl", &["scan", "off"])
        .await
        .map(|_| ())
}

pub(super) async fn bluetooth_connect(backend: &SwayBackend, address: &str) -> anyhow::Result<()> {
    backend
        .sh("bluetoothctl", &["connect", address])
        .await
        .map(|_| ())
}

pub(super) async fn bluetooth_disconnect(
    backend: &SwayBackend,
    address: &str,
) -> anyhow::Result<()> {
    backend
        .sh("bluetoothctl", &["disconnect", address])
        .await
        .map(|_| ())
}

// ─── Files ────────────────────────────────────────

pub(super) async fn files_watch(
    backend: &SwayBackend,
    path: &str,
    recursive: bool,
    _patterns: Option<&[String]>,
) -> anyhow::Result<()> {
    use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
    let watched_path = path.to_string();
    let tx = backend.event_tx.clone();

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let first_path = event.paths.first().cloned().unwrap_or_default();
                let path_str = first_path.to_string_lossy().to_string();
                match event.kind {
                    EventKind::Create(_) => {
                        let _ = tx.send(DeskbridEvent::FileCreated {
                            path: path_str,
                            timestamp: ts,
                        });
                    }
                    EventKind::Modify(_) => {
                        let _ = tx.send(DeskbridEvent::FileModified {
                            path: path_str,
                            timestamp: ts,
                        });
                    }
                    EventKind::Remove(_) => {
                        let _ = tx.send(DeskbridEvent::FileDeleted {
                            path: path_str,
                            timestamp: ts,
                        });
                    }
                    _ => {}
                }
            }
        },
        Config::default(),
    )?;

    let mode = if recursive {
        RecursiveMode::Recursive
    } else {
        RecursiveMode::NonRecursive
    };
    watcher.watch(std::path::Path::new(&watched_path), mode)?;

    backend
        .watchers
        .lock()
        .map_err(|e| anyhow::anyhow!("mutex poisoned: {}", e))?
        .insert(watched_path, watcher);
    Ok(())
}

pub(super) async fn files_unwatch(backend: &SwayBackend, path: &str) -> anyhow::Result<()> {
    backend
        .watchers
        .lock()
        .map_err(|e| anyhow::anyhow!("mutex poisoned: {}", e))?
        .remove(path);
    Ok(())
}

pub(super) async fn files_search(
    _backend: &SwayBackend,
    pattern: &str,
    root: Option<&str>,
    max_results: u32,
) -> anyhow::Result<Vec<String>> {
    let search_root = root.unwrap_or(".");
    let out = Command::new("find")
        .args([
            search_root,
            "-maxdepth",
            "5",
            "-iname",
            pattern,
            "-not",
            "-path",
            "*/.*",
        ])
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    Ok(stdout
        .lines()
        .take(max_results as usize)
        .map(|s| s.to_string())
        .collect())
}

// ─── Audio ────────────────────────────────────────

pub(super) async fn audio_list_sinks(
    backend: &SwayBackend,
) -> anyhow::Result<Vec<protocol::AudioSinkInfo>> {
    let out = backend.sh("pactl", &["list", "sinks"]).await?;
    let mut sinks = Vec::new();
    let mut current_id = 0u32;
    let mut current_name = String::new();
    let mut current_desc = String::new();
    let mut current_volume: f64 = 0.0;
    let mut current_muted = false;

    for line in out.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Sink #") {
            if current_id > 0 {
                sinks.push(protocol::AudioSinkInfo {
                    id: current_id,
                    name: std::mem::take(&mut current_name),
                    description: std::mem::take(&mut current_desc),
                    volume: current_volume,
                    muted: current_muted,
                });
            }
            current_id = trimmed
                .strip_prefix("Sink #")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            current_name.clear();
            current_desc.clear();
            current_volume = 0.0;
            current_muted = false;
        } else if trimmed.starts_with("Description: ") {
            current_desc = trimmed
                .strip_prefix("Description: ")
                .unwrap_or("")
                .to_string();
            current_name = current_desc.clone();
        } else if trimmed.starts_with("Volume: ") {
            if let Some(vol_str) = trimmed.strip_prefix("Volume: ") {
                current_volume = vol_str
                    .split('%')
                    .next()
                    .and_then(|s| s.trim().parse::<u32>().ok())
                    .map(|v| v as f64 / 100.0)
                    .unwrap_or(0.0);
            }
        } else if trimmed.starts_with("Mute: ") {
            current_muted = trimmed
                .strip_prefix("Mute: ")
                .map(|s| s.trim() == "yes")
                .unwrap_or(false);
        }
    }
    if current_id > 0 {
        sinks.push(protocol::AudioSinkInfo {
            id: current_id,
            name: current_name,
            description: current_desc,
            volume: current_volume,
            muted: current_muted,
        });
    }
    Ok(sinks)
}

pub(super) async fn audio_set_sink_volume(
    backend: &SwayBackend,
    sink_id: u32,
    volume: f64,
) -> anyhow::Result<()> {
    backend
        .sh(
            "pactl",
            &[
                "set-sink-volume",
                &sink_id.to_string(),
                &format!("{}%", (volume * 100.0) as u32),
            ],
        )
        .await
        .map(|_| ())
}

// ─── Monitor ──────────────────────────────────────

pub(super) async fn monitor_set_primary(backend: &SwayBackend, output: &str) -> anyhow::Result<()> {
    backend.swaymsg_raw(&["focus", "output", output]).await
}

pub(super) async fn monitor_set_resolution(
    backend: &SwayBackend,
    output: &str,
    width: u32,
    height: u32,
    refresh_rate: Option<f64>,
) -> anyhow::Result<()> {
    let mode = if let Some(rr) = refresh_rate {
        format!("{}x{}@{}Hz", width, height, rr)
    } else {
        format!("{}x{}", width, height)
    };
    backend
        .swaymsg_raw(&[&format!("output {} resolution {}", output, mode)])
        .await
}

pub(super) async fn monitor_set_scale(
    backend: &SwayBackend,
    output: &str,
    scale: f64,
) -> anyhow::Result<()> {
    backend
        .swaymsg_raw(&[&format!("output {} scale {:.2}", output, scale)])
        .await
}

pub(super) async fn monitor_set_rotation(
    backend: &SwayBackend,
    output: &str,
    rotation: &str,
) -> anyhow::Result<()> {
    let rot = match rotation {
        "normal" | "0" => "0",
        "left" | "90" => "90",
        "right" | "270" => "270",
        "inverted" | "180" => "180",
        _ => anyhow::bail!("unsupported rotation: {}", rotation),
    };
    backend
        .swaymsg_raw(&[&format!("output {} transform {}", output, rot)])
        .await
}

pub(super) async fn monitor_set_enabled(
    backend: &SwayBackend,
    output: &str,
    enabled: bool,
) -> anyhow::Result<()> {
    let action = if enabled { "enable" } else { "disable" };
    backend
        .swaymsg_raw(&[&format!("output {} {}", output, action)])
        .await
}
