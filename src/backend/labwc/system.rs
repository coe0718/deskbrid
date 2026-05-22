use super::*;
use crate::protocol;

pub(super) async fn screenshot(
    backend: &LabwcBackend,
    _m: Option<u32>,
    region: Option<protocol::Region>,
    _w: Option<String>,
) -> anyhow::Result<protocol::ScreenshotResult> {
    let path = format!(
        "/tmp/deskbrid_screenshot_{}.png",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs()
    );
    let mut args: Vec<String> = vec!["-t".into(), "png".into()];
    if let Some(r) = region {
        args.push("-g".into());
        args.push(format!("{},{} {}x{}", r.x, r.y, r.width, r.height));
    }
    args.push(path.clone());
    let mut cmd = Command::new("grim");
    cmd.args(&args).stdin(Stdio::null()).stderr(Stdio::piped());
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
    let (w, h) = if let Some(d) = dims {
        let p: Vec<&str> = d.split_whitespace().collect();
        (
            p.first().and_then(|s| s.parse().ok()).unwrap_or(0),
            p.get(1).and_then(|s| s.parse().ok()).unwrap_or(0),
        )
    } else {
        (0, 0)
    };
    Ok(protocol::ScreenshotResult {
        path,
        width: w,
        height: h,
        format: "png".into(),
    })
}

pub(super) async fn notification_send(
    backend: &LabwcBackend,
    a: &str,
    t: &str,
    b: &str,
    u: &str,
) -> anyhow::Result<u32> {
    let out = backend
        .sh("notify-send", &["-a", a, "-u", u, "--print-id", t, b])
        .await?;
    Ok(out.parse().unwrap_or(0))
}

pub(super) async fn notification_close(backend: &LabwcBackend, id: u32) -> anyhow::Result<()> {
    backend
        .sh("makoctl", &["dismiss", "-n", &id.to_string()])
        .await
        .map(|_| ())
}

pub(super) async fn system_info(backend: &LabwcBackend) -> anyhow::Result<protocol::SystemInfo> {
    let ver = backend
        .sh("labwc", &["--version"])
        .await
        .unwrap_or_default();
    let monitors = backend
        .sh("wlr-randr", &[])
        .await
        .map(|_| vec![])
        .unwrap_or_default();
    let idle = backend.idle_seconds().await.unwrap_or(0);
    Ok(protocol::SystemInfo {
        desktop: "Labwc".into(),
        desktop_version: ver.trim().to_string(),
        compositor: format!("labwc {}", ver.trim()),
        session_type: "wayland".into(),
        monitors,
        workspace_count: 1,
        current_workspace: 1,
        idle_seconds: idle,
    })
}

pub(super) async fn idle_seconds(_backend: &LabwcBackend) -> anyhow::Result<u64> {
    let out = Command::new("sh")
        .arg("-c")
        .arg("find /dev/input -name 'event*' -printf '%T@\n' 2>/dev/null | sort -rn | head -1")
        .output()
        .await?;
    let latest: f64 = String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse()
        .unwrap_or(0.0);
    Ok((std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
        - latest) as u64)
}

pub(super) async fn power_action(backend: &LabwcBackend, a: &str) -> anyhow::Result<()> {
    match a {
        "suspend" => backend.sh("systemctl", &["suspend"]).await.map(|_| ()),
        "shutdown" => backend.sh("systemctl", &["poweroff"]).await.map(|_| ()),
        "reboot" => backend.sh("systemctl", &["reboot"]).await.map(|_| ()),
        "lock" => backend.sh("loginctl", &["lock-session"]).await.map(|_| ()),
        _ => anyhow::bail!("unsupported power action: {}", a),
    }
}

pub(super) async fn battery_status(
    _backend: &LabwcBackend,
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

pub(super) async fn network_status(
    backend: &LabwcBackend,
) -> anyhow::Result<protocol::NetworkStatusInfo> {
    let o = backend
        .sh("nmcli", &["-t", "-f", "STATE", "general"])
        .await?;
    Ok(protocol::NetworkStatusInfo {
        online: o.to_lowercase().contains("connected"),
        net_type: String::new(),
    })
}

pub(super) async fn network_interfaces(
    backend: &LabwcBackend,
) -> anyhow::Result<Vec<protocol::NetworkInterfaceInfo>> {
    let o = backend
        .sh("nmcli", &["-t", "-f", "DEVICE,TYPE,STATE", "device"])
        .await?;
    Ok(o.lines()
        .filter_map(|l| {
            let p: Vec<&str> = l.split(':').collect();
            if p.len() >= 2 {
                Some(protocol::NetworkInterfaceInfo {
                    name: p[0].to_string(),
                    state: p.get(1).unwrap_or(&"").to_string(),
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
    backend: &LabwcBackend,
) -> anyhow::Result<Vec<protocol::WifiNetworkInfo>> {
    let _ = backend.sh("nmcli", &["device", "wifi", "rescan"]).await;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    let o = backend
        .sh(
            "nmcli",
            &["-t", "-f", "SSID,SIGNAL,SECURITY", "device", "wifi", "list"],
        )
        .await?;
    Ok(o.lines()
        .filter_map(|l| {
            let p: Vec<&str> = l.split(':').collect();
            if p.len() >= 2 && !p[0].is_empty() {
                Some(protocol::WifiNetworkInfo {
                    ssid: p[0].to_string(),
                    strength: p.get(1).and_then(|s| s.parse().ok()).unwrap_or(0),
                    secured: p.get(2).map(|s| !s.is_empty() && s != &"").unwrap_or(false),
                    frequency: None,
                })
            } else {
                None
            }
        })
        .collect())
}

pub(super) async fn wifi_connect(
    backend: &LabwcBackend,
    ssid: &str,
    pw: Option<&str>,
) -> anyhow::Result<()> {
    let mut a = vec!["device", "wifi", "connect", ssid];
    if let Some(p) = pw {
        a.push("password");
        a.push(p);
    }
    backend.sh("nmcli", &a).await.map(|_| ())
}

pub(super) async fn bluetooth_list(
    backend: &LabwcBackend,
) -> anyhow::Result<Vec<protocol::BluetoothDeviceInfo>> {
    let o = backend.sh("bluetoothctl", &["devices"]).await?;
    Ok(o.lines()
        .filter_map(|l| {
            let p: Vec<&str> = l.splitn(3, ' ').collect();
            if p.len() >= 3 {
                Some(protocol::BluetoothDeviceInfo {
                    address: p[1].to_string(),
                    name: p[2].to_string(),
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
    backend: &LabwcBackend,
    _unused: Option<u32>,
) -> anyhow::Result<()> {
    backend
        .sh("bluetoothctl", &["scan", "on"])
        .await
        .map(|_| ())
}

pub(super) async fn bluetooth_stop_scan(backend: &LabwcBackend) -> anyhow::Result<()> {
    backend
        .sh("bluetoothctl", &["scan", "off"])
        .await
        .map(|_| ())
}

pub(super) async fn bluetooth_connect(backend: &LabwcBackend, a: &str) -> anyhow::Result<()> {
    backend
        .sh("bluetoothctl", &["connect", a])
        .await
        .map(|_| ())
}

pub(super) async fn bluetooth_disconnect(backend: &LabwcBackend, a: &str) -> anyhow::Result<()> {
    backend
        .sh("bluetoothctl", &["disconnect", a])
        .await
        .map(|_| ())
}

pub(super) async fn files_watch(
    backend: &LabwcBackend,
    path: &str,
    recursive: bool,
    _unused: Option<&[String]>,
) -> anyhow::Result<()> {
    use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
    let wp = path.to_string();
    let tx = backend.event_tx.clone();
    let mut w = RecommendedWatcher::new(
        move |r: Result<notify::Event, notify::Error>| {
            if let Ok(e) = r {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let ps = e
                    .paths
                    .first()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                match e.kind {
                    EventKind::Create(_) => {
                        let _ = tx.send(DeskbridEvent::FileCreated {
                            path: ps,
                            timestamp: ts,
                        });
                    }
                    EventKind::Modify(_) => {
                        let _ = tx.send(DeskbridEvent::FileModified {
                            path: ps,
                            timestamp: ts,
                        });
                    }
                    EventKind::Remove(_) => {
                        let _ = tx.send(DeskbridEvent::FileDeleted {
                            path: ps,
                            timestamp: ts,
                        });
                    }
                    _ => {}
                }
            }
        },
        Config::default(),
    )?;
    let m = if recursive {
        RecursiveMode::Recursive
    } else {
        RecursiveMode::NonRecursive
    };
    w.watch(std::path::Path::new(&wp), m)?;
    backend
        .watchers
        .lock()
        .map_err(|e| anyhow::anyhow!("mutex poisoned: {}", e))?
        .insert(wp, w);
    Ok(())
}

pub(super) async fn files_unwatch(backend: &LabwcBackend, path: &str) -> anyhow::Result<()> {
    backend
        .watchers
        .lock()
        .map_err(|e| anyhow::anyhow!("mutex poisoned: {}", e))?
        .remove(path);
    Ok(())
}

pub(super) async fn files_search(
    _backend: &LabwcBackend,
    p: &str,
    r: Option<&str>,
    max: u32,
) -> anyhow::Result<Vec<String>> {
    let root = r.unwrap_or(".");
    let o = Command::new("find")
        .args([root, "-maxdepth", "5", "-iname", p, "-not", "-path", "*/.*"])
        .output()
        .await?;
    Ok(String::from_utf8_lossy(&o.stdout)
        .lines()
        .take(max as usize)
        .map(|s| s.to_string())
        .collect())
}

pub(super) async fn audio_list_sinks(
    backend: &LabwcBackend,
) -> anyhow::Result<Vec<protocol::AudioSinkInfo>> {
    let o = backend.sh("pactl", &["list", "sinks"]).await?;
    let mut sinks = Vec::new();
    let mut id = 0u32;
    let mut name = String::new();
    let mut desc = String::new();
    let mut vol: f64 = 0.0;
    let mut muted = false;
    for l in o.lines() {
        let t = l.trim();
        if t.starts_with("Sink #") {
            if id > 0 {
                sinks.push(protocol::AudioSinkInfo {
                    id,
                    name: std::mem::take(&mut name),
                    description: std::mem::take(&mut desc),
                    volume: vol,
                    muted,
                });
            }
            id = t
                .strip_prefix("Sink #")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            vol = 0.0;
            muted = false;
        } else if t.starts_with("Name: ") {
            name = t.strip_prefix("Name: ").unwrap_or("").to_string();
        } else if t.starts_with("Description: ") {
            desc = t.strip_prefix("Description: ").unwrap_or("").to_string();
        } else if t.starts_with("Volume:") {
            if let Some(pct) = t.split('/').nth(1) {
                vol = pct
                    .trim()
                    .strip_suffix('%')
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0)
                    / 100.0;
            }
        } else if t.starts_with("Mute: ") {
            muted = t.strip_prefix("Mute: ").unwrap_or("").trim() == "yes";
        }
    }
    if id > 0 {
        sinks.push(protocol::AudioSinkInfo {
            id,
            name,
            description: desc,
            volume: vol,
            muted,
        });
    }
    Ok(sinks)
}

pub(super) async fn audio_set_sink_volume(
    backend: &LabwcBackend,
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
    _backend: &LabwcBackend,
    _output: &str,
) -> anyhow::Result<()> {
    anyhow::bail!("monitor_set_primary not implemented on Labwc backend")
}

pub(super) async fn monitor_set_resolution(
    _backend: &LabwcBackend,
    _output: &str,
    _width: u32,
    _height: u32,
    _refresh_rate: Option<f64>,
) -> anyhow::Result<()> {
    anyhow::bail!("monitor_set_resolution not implemented on Labwc backend")
}

pub(super) async fn monitor_set_scale(
    _backend: &LabwcBackend,
    _output: &str,
    _scale: f64,
) -> anyhow::Result<()> {
    anyhow::bail!("monitor_set_scale not implemented on Labwc backend")
}

pub(super) async fn monitor_set_rotation(
    _backend: &LabwcBackend,
    _output: &str,
    _rotation: &str,
) -> anyhow::Result<()> {
    anyhow::bail!("monitor_set_rotation not implemented on Labwc backend")
}

pub(super) async fn monitor_set_enabled(
    _backend: &LabwcBackend,
    _output: &str,
    _enabled: bool,
) -> anyhow::Result<()> {
    anyhow::bail!("monitor_set_enabled not implemented on Labwc backend")
}
