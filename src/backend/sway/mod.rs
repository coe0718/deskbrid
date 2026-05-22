use crate::backend::DesktopBackend;
use crate::protocol;
use crate::protocol::DeskbridEvent;
use async_trait::async_trait;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::process::Command;
use tokio::sync::broadcast;

pub(crate) mod helpers;

use helpers::*;

pub struct SwayBackend {
    pub(super) event_tx: broadcast::Sender<DeskbridEvent>,
    pub(super) watchers: Arc<Mutex<HashMap<String, notify::RecommendedWatcher>>>,
    pub(super) sway_socket: Option<String>,
    pub(super) xdg_runtime: String,
}

impl SwayBackend {
    pub async fn new(event_tx: broadcast::Sender<DeskbridEvent>) -> anyhow::Result<Self> {
        let sway_socket = std::env::var("SWAYSOCK").ok();
        let xdg_runtime =
            std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/run/user/1000".to_string());

        if sway_socket.is_none() {
            eprintln!("[deskbrid] WARN: SWAYSOCK not set — swaymsg may fail");
        }

        Ok(Self {
            event_tx,
            watchers: Arc::new(Mutex::new(HashMap::new())),
            sway_socket,
            xdg_runtime,
        })
    }

    async fn swaymsg_json(&self, args: &[&str]) -> anyhow::Result<serde_json::Value> {
        let mut cmd = Command::new("swaymsg");
        cmd.args(args).stdin(Stdio::null()).stderr(Stdio::piped());
        self.apply_env(&mut cmd);
        let output = cmd.output().await?;
        if !output.status.success() {
            anyhow::bail!(
                "swaymsg failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(serde_json::from_str(&String::from_utf8(output.stdout)?)?)
    }

    /// Run arbitrary swaymsg commands (no JSON output expected).
    async fn swaymsg_raw(&self, args: &[&str]) -> anyhow::Result<()> {
        let mut cmd = Command::new("swaymsg");
        cmd.args(args).stdin(Stdio::null()).stderr(Stdio::piped());
        self.apply_env(&mut cmd);
        let output = cmd.output().await?;
        if !output.status.success() {
            anyhow::bail!(
                "swaymsg failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(())
    }

    async fn sh(&self, cmd: &str, args: &[&str]) -> anyhow::Result<String> {
        let mut c = Command::new(cmd);
        c.args(args).stdin(Stdio::null()).stderr(Stdio::piped());
        self.apply_env(&mut c);
        let out = c.output().await?;
        if !out.status.success() {
            anyhow::bail!(
                "{} failed: {}",
                cmd,
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(String::from_utf8(out.stdout)?.trim().to_string())
    }

    fn apply_env(&self, cmd: &mut Command) {
        cmd.env("XDG_RUNTIME_DIR", &self.xdg_runtime);
        if let Some(ref sock) = self.sway_socket {
            cmd.env("SWAYSOCK", sock);
        }
    }

    async fn ydotool(&self, args: &[&str]) -> anyhow::Result<()> {
        self.sh("ydotool", args).await.map(|_| ())
    }
}

#[async_trait]
impl DesktopBackend for SwayBackend {
    // ─── Windows ──────────────────────────────────────

    async fn windows_list(&self) -> anyhow::Result<Vec<protocol::WindowInfo>> {
        let tree = self.swaymsg_json(&["-t", "get_tree"]).await?;
        Ok(parse_sway_tree_windows(&tree))
    }

    async fn window_focus(&self, id: &str) -> anyhow::Result<()> {
        self.swaymsg_raw(&["[con_id=", id, "]", "focus"]).await
    }

    async fn window_get(&self, id: &str) -> anyhow::Result<protocol::WindowInfo> {
        let windows = self.windows_list().await?;
        windows
            .into_iter()
            .find(|w| w.id == id)
            .ok_or_else(|| anyhow::anyhow!("window not found: {}", id))
    }

    async fn window_close(&self, id: &str) -> anyhow::Result<()> {
        self.swaymsg_raw(&["[con_id=", id, "]", "kill"]).await
    }

    async fn window_minimize(&self, id: &str) -> anyhow::Result<()> {
        self.swaymsg_raw(&["[con_id=", id, "]", "move", "scratchpad"])
            .await
    }

    async fn window_maximize(&self, id: &str) -> anyhow::Result<()> {
        self.swaymsg_raw(&["[con_id=", id, "]", "fullscreen", "toggle"])
            .await
    }

    async fn window_move_resize(
        &self,
        id: &str,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> anyhow::Result<()> {
        self.swaymsg_raw(&[
            "[con_id=",
            id,
            "]",
            "move",
            "absolute",
            "position",
            &x.to_string(),
            &y.to_string(),
        ])
        .await?;
        self.swaymsg_raw(&[
            "[con_id=",
            id,
            "]",
            "resize",
            "set",
            &width.to_string(),
            &height.to_string(),
        ])
        .await
    }

    // ─── Workspaces ───────────────────────────────────

    async fn workspaces_list(&self) -> anyhow::Result<Vec<protocol::WorkspaceInfo>> {
        let raw = self.swaymsg_json(&["-t", "get_workspaces"]).await?;
        Ok(parse_sway_workspaces(&raw))
    }

    async fn workspace_switch(&self, id: u32) -> anyhow::Result<()> {
        self.swaymsg_raw(&["workspace", "number", &id.to_string()])
            .await
    }

    async fn workspace_move_window(
        &self,
        window_id: &str,
        workspace_id: u32,
        _follow: bool,
    ) -> anyhow::Result<()> {
        self.swaymsg_raw(&[
            "[con_id=",
            window_id,
            "]",
            "move",
            "container",
            "to",
            "workspace",
            "number",
            &workspace_id.to_string(),
        ])
        .await
    }

    // ─── Input (ydotool) ─────────────────────────────

    async fn keyboard_type(&self, text: &str) -> anyhow::Result<()> {
        self.ydotool(&["type", text]).await
    }

    async fn keyboard_key(&self, key: &str) -> anyhow::Result<()> {
        self.ydotool(&["key", key]).await
    }

    async fn keyboard_combo(&self, keys: &[String]) -> anyhow::Result<()> {
        for key in keys {
            self.ydotool(&["key", &format!("{}:1", key)]).await?;
        }
        for key in keys.iter().rev() {
            self.ydotool(&["key", &format!("{}:0", key)]).await?;
        }
        Ok(())
    }

    async fn mouse_move(&self, x: f64, y: f64) -> anyhow::Result<()> {
        self.ydotool(&["mousemove", "--absolute", &x.to_string(), &y.to_string()])
            .await
    }

    async fn mouse_click(&self, button: &str) -> anyhow::Result<()> {
        let btn: u8 = match button.to_lowercase().as_str() {
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

    // ─── Clipboard ────────────────────────────────────

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

    // ─── Screenshot ───────────────────────────────────

    async fn screenshot(
        &self,
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
            let outputs = self.swaymsg_json(&["-t", "get_outputs"]).await?;
            let monitors = parse_sway_outputs(&outputs);
            monitors
                .get(monitor_id as usize)
                .map(|m| m.name.clone())
                .unwrap_or_default()
        } else {
            let outputs = self.swaymsg_json(&["-t", "get_outputs"]).await?;
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
        self.apply_env(&mut cmd);
        let out = cmd.output().await?;
        if !out.status.success() {
            anyhow::bail!(
                "grim failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }

        let dims = self.sh("identify", &["-format", "%w %h", &path]).await.ok();
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

    // ─── System ───────────────────────────────────────

    async fn system_info(&self) -> anyhow::Result<protocol::SystemInfo> {
        let version = self
            .sh("swaymsg", &["-t", "get_version"])
            .await
            .unwrap_or_default();
        let monitors = self
            .swaymsg_json(&["-t", "get_outputs"])
            .await
            .map(|v| parse_sway_outputs(&v))
            .unwrap_or_default();
        let workspaces = self
            .swaymsg_json(&["-t", "get_workspaces"])
            .await
            .map(|v| parse_sway_workspaces(&v))
            .unwrap_or_default();
        let current_ws = workspaces
            .iter()
            .find(|w| w.is_active)
            .map(|w| w.id)
            .unwrap_or(0);
        let idle = self.idle_seconds().await.unwrap_or(0);

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

    async fn network_status(&self) -> anyhow::Result<protocol::NetworkStatusInfo> {
        let out = self.sh("nmcli", &["-t", "-f", "STATE", "general"]).await?;
        let online = out.to_lowercase().contains("connected");
        Ok(protocol::NetworkStatusInfo {
            online,
            net_type: String::new(),
        })
    }

    async fn network_interfaces(&self) -> anyhow::Result<Vec<protocol::NetworkInterfaceInfo>> {
        let out = self
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

    async fn wifi_scan(&self) -> anyhow::Result<Vec<protocol::WifiNetworkInfo>> {
        let _ = self.sh("nmcli", &["device", "wifi", "rescan"]).await;
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let out = self
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

    async fn wifi_connect(&self, ssid: &str, password: Option<&str>) -> anyhow::Result<()> {
        let mut args = vec!["device", "wifi", "connect", ssid];
        if let Some(pw) = password {
            args.push("password");
            args.push(pw);
        }
        self.sh("nmcli", &args).await.map(|_| ())
    }

    // ─── Bluetooth ────────────────────────────────────

    async fn bluetooth_list(&self) -> anyhow::Result<Vec<protocol::BluetoothDeviceInfo>> {
        let out = self.sh("bluetoothctl", &["devices"]).await?;
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

    // ─── Files ────────────────────────────────────────

    async fn files_watch(
        &self,
        path: &str,
        recursive: bool,
        _patterns: Option<&[String]>,
    ) -> anyhow::Result<()> {
        use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
        let watched_path = path.to_string();
        let tx = self.event_tx.clone();

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

        self.watchers
            .lock()
            .map_err(|e| anyhow::anyhow!("mutex poisoned: {}", e))?
            .insert(watched_path, watcher);
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
        let stdout = String::from_utf8_lossy(&out.stdout);
        Ok(stdout
            .lines()
            .take(max_results as usize)
            .map(|s| s.to_string())
            .collect())
    }

    // ─── Audio ────────────────────────────────────────

    async fn audio_list_sinks(&self) -> anyhow::Result<Vec<protocol::AudioSinkInfo>> {
        let out = self.sh("pactl", &["list", "sinks"]).await?;
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

    // ─── Monitor ──────────────────────────────────────

    async fn monitor_set_primary(&self, output: &str) -> anyhow::Result<()> {
        self.swaymsg_raw(&["focus", "output", output]).await
    }

    async fn monitor_set_resolution(
        &self,
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
        self.swaymsg_raw(&[&format!("output {} resolution {}", output, mode)])
            .await
    }

    async fn monitor_set_scale(&self, output: &str, scale: f64) -> anyhow::Result<()> {
        self.swaymsg_raw(&[&format!("output {} scale {:.2}", output, scale)])
            .await
    }

    async fn monitor_set_rotation(&self, output: &str, rotation: &str) -> anyhow::Result<()> {
        let rot = match rotation {
            "normal" | "0" => "0",
            "left" | "90" => "90",
            "right" | "270" => "270",
            "inverted" | "180" => "180",
            _ => anyhow::bail!("unsupported rotation: {}", rotation),
        };
        self.swaymsg_raw(&[&format!("output {} transform {}", output, rot)])
            .await
    }

    async fn monitor_set_enabled(&self, output: &str, enabled: bool) -> anyhow::Result<()> {
        let action = if enabled { "enable" } else { "disable" };
        self.swaymsg_raw(&[&format!("output {} {}", output, action)])
            .await
    }
}
