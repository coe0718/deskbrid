use super::*;
use crate::protocol;

pub(super) async fn screenshot(
    backend: &NiriBackend,
    _monitor: Option<u32>,
    region: Option<protocol::Region>,
    _window_id: Option<String>,
) -> anyhow::Result<protocol::ScreenshotResult> {
    let path = format!(
        "/tmp/deskbrid_screenshot_{}.png",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs()
    );

    let mut grim_args: Vec<String> = vec!["-t".into(), "png".into()];
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

pub(super) async fn notification_send(
    backend: &NiriBackend,
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

pub(super) async fn notification_close(backend: &NiriBackend, id: u32) -> anyhow::Result<()> {
    backend
        .sh("makoctl", &["dismiss", "-n", &id.to_string()])
        .await
        .map(|_| ())
}

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

pub(super) async fn network_status(
    backend: &NiriBackend,
) -> anyhow::Result<protocol::NetworkStatusInfo> {
    let out = backend
        .sh("nmcli", &["-t", "-f", "STATE", "general"])
        .await?;
    Ok(protocol::NetworkStatusInfo {
        online: out.to_lowercase().contains("connected"),
        net_type: String::new(),
    })
}

pub(super) async fn network_interfaces(
    backend: &NiriBackend,
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
    backend: &NiriBackend,
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
    backend: &NiriBackend,
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

pub(super) async fn bluetooth_list(
    backend: &NiriBackend,
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
    backend: &NiriBackend,
    _duration: Option<u32>,
) -> anyhow::Result<()> {
    backend
        .sh("bluetoothctl", &["scan", "on"])
        .await
        .map(|_| ())
}

pub(super) async fn bluetooth_stop_scan(backend: &NiriBackend) -> anyhow::Result<()> {
    backend
        .sh("bluetoothctl", &["scan", "off"])
        .await
        .map(|_| ())
}

pub(super) async fn bluetooth_connect(backend: &NiriBackend, address: &str) -> anyhow::Result<()> {
    backend
        .sh("bluetoothctl", &["connect", address])
        .await
        .map(|_| ())
}

pub(super) async fn bluetooth_disconnect(
    backend: &NiriBackend,
    address: &str,
) -> anyhow::Result<()> {
    backend
        .sh("bluetoothctl", &["disconnect", address])
        .await
        .map(|_| ())
}

pub(super) async fn files_watch(
    backend: &NiriBackend,
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
                let path_str = event
                    .paths
                    .first()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
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

pub(super) async fn files_unwatch(backend: &NiriBackend, path: &str) -> anyhow::Result<()> {
    backend
        .watchers
        .lock()
        .map_err(|e| anyhow::anyhow!("mutex poisoned: {}", e))?
        .remove(path);
    Ok(())
}

pub(super) async fn files_search(
    _backend: &NiriBackend,
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
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .take(max_results as usize)
        .map(|s| s.to_string())
        .collect())
}

pub(super) async fn audio_list_sinks(
    backend: &NiriBackend,
) -> anyhow::Result<Vec<protocol::AudioSinkInfo>> {
    let out = backend.sh("pactl", &["list", "sinks"]).await?;
    let mut sinks = Vec::new();
    let mut cur_id = 0u32;
    let mut cur_name = String::new();
    let mut cur_desc = String::new();
    let mut cur_vol: f64 = 0.0;
    let mut cur_muted = false;
    for line in out.lines() {
        let t = line.trim();
        if t.starts_with("Sink #") {
            if cur_id > 0 {
                sinks.push(protocol::AudioSinkInfo {
                    id: cur_id,
                    name: std::mem::take(&mut cur_name),
                    description: std::mem::take(&mut cur_desc),
                    volume: cur_vol,
                    muted: cur_muted,
                });
            }
            cur_id = t
                .strip_prefix("Sink #")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            cur_name.clear();
            cur_desc.clear();
            cur_vol = 0.0;
            cur_muted = false;
        } else if t.starts_with("Description: ") {
            cur_desc = t.strip_prefix("Description: ").unwrap_or("").to_string();
            cur_name = cur_desc.clone();
        } else if t.starts_with("Volume: ") {
            cur_vol = t
                .strip_prefix("Volume: ")
                .and_then(|v| {
                    v.split('%')
                        .next()
                        .and_then(|s| s.trim().parse::<u32>().ok())
                })
                .map(|v| v as f64 / 100.0)
                .unwrap_or(0.0);
        } else if t.starts_with("Mute: ") {
            cur_muted = t
                .strip_prefix("Mute: ")
                .map(|s| s.trim() == "yes")
                .unwrap_or(false);
        }
    }
    if cur_id > 0 {
        sinks.push(protocol::AudioSinkInfo {
            id: cur_id,
            name: cur_name,
            description: cur_desc,
            volume: cur_vol,
            muted: cur_muted,
        });
    }
    Ok(sinks)
}

pub(super) async fn audio_set_sink_volume(
    backend: &NiriBackend,
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

pub(super) async fn monitor_set_primary(
    _backend: &NiriBackend,
    _output: &str,
) -> anyhow::Result<()> {
    Ok(())
}

pub(super) async fn monitor_set_resolution(
    _backend: &NiriBackend,
    _output: &str,
    _w: u32,
    _h: u32,
    _rr: Option<f64>,
) -> anyhow::Result<()> {
    Ok(())
}

pub(super) async fn monitor_set_scale(
    _backend: &NiriBackend,
    _output: &str,
    _scale: f64,
) -> anyhow::Result<()> {
    Ok(())
}

pub(super) async fn monitor_set_rotation(
    _backend: &NiriBackend,
    _output: &str,
    _rot: &str,
) -> anyhow::Result<()> {
    Ok(())
}

pub(super) async fn monitor_set_enabled(
    _backend: &NiriBackend,
    _output: &str,
    _enabled: bool,
) -> anyhow::Result<()> {
    Ok(())
}
