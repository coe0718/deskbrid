use super::*;
use crate::protocol;
use crate::backend::DesktopBackend;
use async_trait::async_trait;

#[async_trait]
impl DesktopBackend for LabwcBackend {
impl DesktopBackend for LabwcBackend {
    async fn windows_list(&self) -> anyhow::Result<Vec<protocol::WindowInfo>> {
        if self.has_labwc_helper {
            let raw = self.helper_json(&["list-windows"]).await?;
            return Ok(parse_labwc_windows_json(&raw));
        }
        // wlrctl fallback
        let raw = self.sh("wlrctl", &["toplevel", "list"]).await?;
        let focused = self
            .sh("wlrctl", &["toplevel", "get-focus"])
            .await
            .ok()
            .map(|s| s.trim().to_string());
        Ok(parse_wlrctl_windows(&raw, focused.as_deref()))
    }

    async fn window_focus(&self, id: &str) -> anyhow::Result<()> {
        if self.has_labwc_helper {
            self.helper_json(&["activate", "--window-id", id])
                .await
                .map(|_| ())
        } else {
            self.sh("wlrctl", &["toplevel", "focus", id])
                .await
                .map(|_| ())
        }
    }

    async fn window_get(&self, id: &str) -> anyhow::Result<protocol::WindowInfo> {
        self.windows_list()
            .await?
            .into_iter()
            .find(|w| w.id == id)
            .ok_or_else(|| anyhow::anyhow!("window not found: {}", id))
    }

    async fn window_close(&self, id: &str) -> anyhow::Result<()> {
        if self.has_labwc_helper {
            self.helper_json(&["close", "--window-id", id])
                .await
                .map(|_| ())
        } else {
            self.sh("wlrctl", &["toplevel", "close", id])
                .await
                .map(|_| ())
        }
    }

    async fn window_minimize(&self, id: &str) -> anyhow::Result<()> {
        if self.has_labwc_helper {
            self.helper_json(&["minimize", "--window-id", id])
                .await
                .map(|_| ())
        } else {
            anyhow::bail!(
                "minimize not available via wlrctl; install labwc-helper for full support"
            )
        }
    }

    async fn window_maximize(&self, id: &str) -> anyhow::Result<()> {
        if self.has_labwc_helper {
            self.helper_json(&["maximize", "--window-id", id])
                .await
                .map(|_| ())
        } else {
            self.sh("wlrctl", &["toplevel", "maximize", id])
                .await
                .map(|_| ())
        }
    }

    async fn window_move_resize(
        &self,
        _id: &str,
        _x: i32,
        _y: i32,
        _w: u32,
        _h: u32,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn workspaces_list(&self) -> anyhow::Result<Vec<protocol::WorkspaceInfo>> {
        Ok(vec![protocol::WorkspaceInfo {
            id: 1,
            name: "workspace-1".into(),
            is_active: true,
        }])
    }

    async fn workspace_switch(&self, _id: u32) -> anyhow::Result<()> {
        Ok(())
    }

    async fn workspace_move_window(&self, _w: &str, _ws: u32, _follow: bool) -> anyhow::Result<()> {
        Ok(())
    }

    async fn keyboard_type(&self, t: &str) -> anyhow::Result<()> {
        self.ydotool(&["type", t]).await
    }

    async fn keyboard_key(&self, k: &str) -> anyhow::Result<()> {
        self.ydotool(&["key", k]).await
    }

    async fn keyboard_combo(&self, keys: &[String]) -> anyhow::Result<()> {
        for k in keys {
            self.ydotool(&["key", &format!("{}:1", k)]).await?;
        }
        for k in keys.iter().rev() {
            self.ydotool(&["key", &format!("{}:0", k)]).await?;
        }
        Ok(())
    }

    async fn mouse_move(&self, x: f64, y: f64) -> anyhow::Result<()> {
        self.ydotool(&["mousemove", "--absolute", &x.to_string(), &y.to_string()])
            .await
    }

    async fn mouse_click(&self, b: &str) -> anyhow::Result<()> {
        let btn: u8 = match b {
            "left" => 1,
            "middle" => 2,
            "right" => 3,
            _ => 1,
        };
        self.ydotool(&["click", &btn.to_string()]).await
    }

    async fn mouse_scroll(&self, dx: f64, dy: f64) -> anyhow::Result<()> {
        if dy != 0.0 {
            self.ydotool(&["mousemove", "--wheel", "0", &format!("{}", dy as i32)])
                .await?;
        }
        if dx != 0.0 {
            self.ydotool(&["mousemove", "--wheel", &format!("{}", dx as i32), "0"])
                .await?;
        }
        Ok(())
    }

    async fn clipboard_read(&self) -> anyhow::Result<String> {
        self.sh("wl-paste", &[]).await
    }

    async fn clipboard_write(&self, text: &str) -> anyhow::Result<()> {
        let mut cmd = Command::new("wl-copy");
        cmd.stdin(Stdio::piped()).stderr(Stdio::piped());
        self.apply_env(&mut cmd);
        let mut child = cmd.spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(text.as_bytes()).await?;
        }
        let output = child.wait_with_output().await?;
        if !output.status.success() {
            anyhow::bail!(
                "wl-copy failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(())
    }

    async fn screenshot(
        &self,
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
        self.apply_env(&mut cmd);
        let out = cmd.output().await?;
        if !out.status.success() {
            anyhow::bail!(
                "grim failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        let dims = self.sh("identify", &["-format", "%w %h", &path]).await.ok();
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

    async fn notification_send(&self, a: &str, t: &str, b: &str, u: &str) -> anyhow::Result<u32> {
        let out = self
            .sh("notify-send", &["-a", a, "-u", u, "--print-id", t, b])
            .await?;
        Ok(out.parse().unwrap_or(0))
    }

    async fn notification_close(&self, id: u32) -> anyhow::Result<()> {
        self.sh("makoctl", &["dismiss", "-n", &id.to_string()])
            .await
            .map(|_| ())
    }

    async fn system_info(&self) -> anyhow::Result<protocol::SystemInfo> {
        let ver = self.sh("labwc", &["--version"]).await.unwrap_or_default();
        let monitors = self
            .sh("wlr-randr", &[])
            .await
            .map(|_| vec![])
            .unwrap_or_default();
        let idle = self.idle_seconds().await.unwrap_or(0);
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

    async fn idle_seconds(&self) -> anyhow::Result<u64> {
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

    async fn power_action(&self, a: &str) -> anyhow::Result<()> {
        match a {
            "suspend" => self.sh("systemctl", &["suspend"]).await.map(|_| ()),
            "shutdown" => self.sh("systemctl", &["poweroff"]).await.map(|_| ()),
            "reboot" => self.sh("systemctl", &["reboot"]).await.map(|_| ()),
            "lock" => self.sh("loginctl", &["lock-session"]).await.map(|_| ()),
            _ => anyhow::bail!("unsupported power action: {}", a),
        }
    }

    async fn battery_status(&self) -> anyhow::Result<Vec<protocol::BatteryInfo>> {
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

    async fn network_status(&self) -> anyhow::Result<protocol::NetworkStatusInfo> {
        let o = self.sh("nmcli", &["-t", "-f", "STATE", "general"]).await?;
        Ok(protocol::NetworkStatusInfo {
            online: o.to_lowercase().contains("connected"),
            net_type: String::new(),
        })
    }

    async fn network_interfaces(&self) -> anyhow::Result<Vec<protocol::NetworkInterfaceInfo>> {
        let o = self
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

    async fn wifi_scan(&self) -> anyhow::Result<Vec<protocol::WifiNetworkInfo>> {
        let _ = self.sh("nmcli", &["device", "wifi", "rescan"]).await;
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let o = self
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

    async fn wifi_connect(&self, ssid: &str, pw: Option<&str>) -> anyhow::Result<()> {
        let mut a = vec!["device", "wifi", "connect", ssid];
        if let Some(p) = pw {
            a.push("password");
            a.push(p);
        }
        self.sh("nmcli", &a).await.map(|_| ())
    }

    async fn bluetooth_list(&self) -> anyhow::Result<Vec<protocol::BluetoothDeviceInfo>> {
        let o = self.sh("bluetoothctl", &["devices"]).await?;
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

    async fn bluetooth_scan(&self, _: Option<u32>) -> anyhow::Result<()> {
        self.sh("bluetoothctl", &["scan", "on"]).await.map(|_| ())
    }

    async fn bluetooth_stop_scan(&self) -> anyhow::Result<()> {
        self.sh("bluetoothctl", &["scan", "off"]).await.map(|_| ())
    }

    async fn bluetooth_connect(&self, a: &str) -> anyhow::Result<()> {
        self.sh("bluetoothctl", &["connect", a]).await.map(|_| ())
    }

    async fn bluetooth_disconnect(&self, a: &str) -> anyhow::Result<()> {
        self.sh("bluetoothctl", &["disconnect", a])
            .await
            .map(|_| ())
    }

    async fn files_watch(
        &self,
        path: &str,
        recursive: bool,
        _: Option<&[String]>,
    ) -> anyhow::Result<()> {
        use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
        let wp = path.to_string();
        let tx = self.event_tx.clone();
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
        self.watchers
            .lock()
            .map_err(|e| anyhow::anyhow!("mutex poisoned: {}", e))?
            .insert(wp, w);
        Ok(())
    }

    async fn files_unwatch(&self, path: &str) -> anyhow::Result<()> {
        self.watchers
            .lock()
            .map_err(|e| anyhow::anyhow!("mutex poisoned: {}", e))?
            .remove(path);
        Ok(())
    }

    async fn files_search(
        &self,
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

    async fn audio_list_sinks(&self) -> anyhow::Result<Vec<protocol::AudioSinkInfo>> {
        let o = self.sh("pactl", &["list", "sinks"]).await?;
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

    async fn audio_set_sink_volume(&self, sink_id: u32, volume: f64) -> anyhow::Result<()> {
        self.sh(
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

    async fn monitor_set_primary(&self, _output: &str) -> anyhow::Result<()> {
        anyhow::bail!("monitor_set_primary not implemented on Labwc backend")
    }

    async fn monitor_set_resolution(
        &self,
        _output: &str,
        _width: u32,
        _height: u32,
        _refresh_rate: Option<f64>,
    ) -> anyhow::Result<()> {
        anyhow::bail!("monitor_set_resolution not implemented on Labwc backend")
    }

    async fn monitor_set_scale(&self, _output: &str, _scale: f64) -> anyhow::Result<()> {
        anyhow::bail!("monitor_set_scale not implemented on Labwc backend")
    }

    async fn monitor_set_rotation(&self, _output: &str, _rotation: &str) -> anyhow::Result<()> {
        anyhow::bail!("monitor_set_rotation not implemented on Labwc backend")
    }

    async fn monitor_set_enabled(&self, _output: &str, _enabled: bool) -> anyhow::Result<()> {
        anyhow::bail!("monitor_set_enabled not implemented on Labwc backend")
    }
}
