use super::*;
use crate::protocol;
use crate::backend::DesktopBackend;
use async_trait::async_trait;

#[async_trait]
impl DesktopBackend for WayfireBackend {
impl DesktopBackend for WayfireBackend {
    async fn windows_list(&self) -> anyhow::Result<Vec<protocol::WindowInfo>> {
        let raw = self.wf_ipc_json(&["-j"]).await?;
        Ok(parse_wayfire_views(&raw))
    }

    async fn window_focus(&self, id: &str) -> anyhow::Result<()> {
        self.sh("wf-ipc", &["focus-view", id]).await.map(|_| ())
    }

    async fn window_get(&self, id: &str) -> anyhow::Result<protocol::WindowInfo> {
        self.windows_list()
            .await?
            .into_iter()
            .find(|w| w.id == id)
            .ok_or_else(|| anyhow::anyhow!("window not found: {}", id))
    }

    async fn window_close(&self, id: &str) -> anyhow::Result<()> {
        self.sh("wf-ipc", &["close-view", id]).await.map(|_| ())
    }

    async fn window_minimize(&self, id: &str) -> anyhow::Result<()> {
        self.sh("wf-ipc", &["set-view-options", id, "minimized"])
            .await
            .map(|_| ())
    }

    async fn window_maximize(&self, id: &str) -> anyhow::Result<()> {
        self.sh("wf-ipc", &["set-view-options", id, "fullscreen"])
            .await
            .map(|_| ())
    }

    async fn window_move_resize(
        &self,
        _id: &str,
        _x: i32,
        _y: i32,
        _w: u32,
        _h: u32,
    ) -> anyhow::Result<()> {
        // wf-ipc doesn't support move/resize natively
        Ok(())
    }

    async fn workspaces_list(&self) -> anyhow::Result<Vec<protocol::WorkspaceInfo>> {
        Ok(parse_wayfire_workspaces(&serde_json::Value::Null))
    }

    async fn workspace_switch(&self, id: u32) -> anyhow::Result<()> {
        self.sh("wf-ipc", &["set-workspace", &id.to_string()])
            .await
            .map(|_| ())
    }

    async fn workspace_move_window(
        &self,
        window_id: &str,
        workspace_id: u32,
        _follow: bool,
    ) -> anyhow::Result<()> {
        self.sh(
            "wf-ipc",
            &["set-view-workspace", window_id, &workspace_id.to_string()],
        )
        .await
        .map(|_| ())
    }

    // ─── Shared wlroots infra ───

    async fn keyboard_type(&self, text: &str) -> anyhow::Result<()> {
        self.ydotool(&["type", text]).await
    }
    async fn keyboard_key(&self, key: &str) -> anyhow::Result<()> {
        self.ydotool(&["key", key]).await
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
    async fn mouse_click(&self, button: &str) -> anyhow::Result<()> {
        let btn: u8 = match button {
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
        let (width, height) = if let Some(d) = dims {
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
            width,
            height,
            format: "png".into(),
        })
    }

    async fn notification_send(
        &self,
        app_name: &str,
        title: &str,
        body: &str,
        urgency: &str,
    ) -> anyhow::Result<u32> {
        let out = self
            .sh(
                "notify-send",
                &["-a", app_name, "-u", urgency, "--print-id", title, body],
            )
            .await?;
        Ok(out.parse().unwrap_or(0))
    }
    async fn notification_close(&self, id: u32) -> anyhow::Result<()> {
        self.sh("makoctl", &["dismiss", "-n", &id.to_string()])
            .await
            .map(|_| ())
    }

    async fn system_info(&self) -> anyhow::Result<protocol::SystemInfo> {
        let ver = self.sh("wayfire", &["--version"]).await.unwrap_or_default();
        let monitors = self
            .wf_ipc_json(&["list-outputs", "-j"])
            .await
            .map(|v| parse_wayfire_outputs(&v))
            .unwrap_or_default();
        let idle = self.idle_seconds().await.unwrap_or(0);
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
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        Ok((now - latest) as u64)
    }

    async fn power_action(&self, action: &str) -> anyhow::Result<()> {
        match action {
            "suspend" => self.sh("systemctl", &["suspend"]).await.map(|_| ()),
            "shutdown" => self.sh("systemctl", &["poweroff"]).await.map(|_| ()),
            "reboot" => self.sh("systemctl", &["reboot"]).await.map(|_| ()),
            "lock" => self.sh("loginctl", &["lock-session"]).await.map(|_| ()),
            _ => anyhow::bail!("unsupported power action: {}", action),
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
    async fn bluetooth_scan(&self, _duration: Option<u32>) -> anyhow::Result<()> {
        self.sh("bluetoothctl", &["scan", "on"]).await.map(|_| ())
    }
    async fn bluetooth_stop_scan(&self) -> anyhow::Result<()> {
        self.sh("bluetoothctl", &["scan", "off"]).await.map(|_| ())
    }
    async fn bluetooth_connect(&self, address: &str) -> anyhow::Result<()> {
        self.sh("bluetoothctl", &["connect", address])
            .await
            .map(|_| ())
    }
    async fn bluetooth_disconnect(&self, address: &str) -> anyhow::Result<()> {
        self.sh("bluetoothctl", &["disconnect", address])
            .await
            .map(|_| ())
    }

    async fn files_watch(
        &self,
        path: &str,
        recursive: bool,
        _patterns: Option<&[String]>,
    ) -> anyhow::Result<()> {
        use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
        let wp = path.to_string();
        let tx = self.event_tx.clone();
        let mut w = RecommendedWatcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                if let Ok(e) = res {
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
        let mode = if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        w.watch(std::path::Path::new(&wp), mode)?;
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

    async fn audio_list_sinks(&self) -> anyhow::Result<Vec<protocol::AudioSinkInfo>> {
        let out = self.sh("pactl", &["list", "sinks"]).await?;
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

    // Monitor controls — wf-ipc doesn't support these yet
    async fn monitor_set_primary(&self, _output: &str) -> anyhow::Result<()> {
        Ok(())
    }
    async fn monitor_set_resolution(
        &self,
        _output: &str,
        _w: u32,
        _h: u32,
        _rr: Option<f64>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    async fn monitor_set_scale(&self, _output: &str, _scale: f64) -> anyhow::Result<()> {
        Ok(())
    }
    async fn monitor_set_rotation(&self, _output: &str, _rotation: &str) -> anyhow::Result<()> {
        Ok(())
    }
    async fn monitor_set_enabled(&self, _output: &str, _enabled: bool) -> anyhow::Result<()> {
        Ok(())
    }
}
