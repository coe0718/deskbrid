//! COSMIC desktop backend — controls windows/workspaces via `cosmic-helper`
//! subprocess. Shares input, clipboard, screenshot, and notification tooling
//! with the Hyprland backend pattern.

use crate::backend::DesktopBackend;
use crate::protocol;
use async_trait::async_trait;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::Mutex;

pub struct CosmicBackend {
    #[allow(dead_code)]
    event_tx: tokio::sync::broadcast::Sender<protocol::DeskbridEvent>,
    #[allow(dead_code)]
    watchers: Arc<Mutex<HashMap<String, notify::RecommendedWatcher>>>,
    last_mouse: std::sync::Mutex<(f64, f64)>,
    wl_socket: Option<String>,
    xdg_runtime: String,
    /// Path to the cosmic-helper binary
    helper_path: String,
}

impl CosmicBackend {
    pub async fn new(
        event_tx: tokio::sync::broadcast::Sender<protocol::DeskbridEvent>,
    ) -> anyhow::Result<Self> {
        let xdg_runtime =
            std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/run/user/1000".to_string());
        let wl_socket = std::env::var("WAYLAND_DISPLAY").ok();

        // Find cosmic-helper binary: next to our binary, then on PATH
        let helper_path = Self::find_helper();

        let backend = Self {
            event_tx,
            watchers: Arc::new(Mutex::new(HashMap::new())),
            last_mouse: std::sync::Mutex::new((960.0, 540.0)),
            wl_socket,
            xdg_runtime,
            helper_path,
        };

        Ok(backend)
    }

    fn find_helper() -> String {
        // Check if sibling binary exists
        if let Ok(exe) = std::env::current_exe() {
            let sibling = exe.parent().unwrap().join("cosmic-helper");
            if sibling.exists() {
                return sibling.to_string_lossy().to_string();
            }
        }
        // Fallback to PATH
        "cosmic-helper".to_string()
    }

    /// Run cosmic-helper CLI and parse JSON output
    async fn helper_json(&self, args: &[&str]) -> anyhow::Result<serde_json::Value> {
        let output = Command::new(&self.helper_path)
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .env("XDG_RUNTIME_DIR", &self.xdg_runtime)
            .env(
                "WAYLAND_DISPLAY",
                self.wl_socket.as_deref().unwrap_or("wayland-0"),
            )
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("cosmic-helper failed: {}", stderr.trim());
        }

        let stdout = String::from_utf8(output.stdout)?;
        if stdout.trim().is_empty() || stdout.trim() == "null" {
            return Ok(serde_json::Value::Null);
        }

        Ok(serde_json::from_str(&stdout)?)
    }

    /// Run cosmic-helper CLI, check exit code and JSON response
    async fn helper_run(&self, args: &[&str]) -> anyhow::Result<()> {
        let output = Command::new(&self.helper_path)
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .env("XDG_RUNTIME_DIR", &self.xdg_runtime)
            .env(
                "WAYLAND_DISPLAY",
                self.wl_socket.as_deref().unwrap_or("wayland-0"),
            )
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "cosmic-helper '{}' failed: {}",
                args.join(" "),
                stderr.trim()
            );
        }

        // Check JSON response for {"ok": false} — helper may exit 0
        // even when the operation wasn't actually performed.
        if let Ok(body) = String::from_utf8(output.stdout) {
            if let Ok(resp) = serde_json::from_str::<serde_json::Value>(&body) {
                if resp.get("ok").and_then(|v| v.as_bool()) == Some(false) {
                    let detail = resp
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown error");
                    anyhow::bail!("cosmic-helper '{}' failed: {}", args.join(" "), detail);
                }
            }
        }

        Ok(())
    }

    /// Run a command and return stdout
    async fn sh(&self, cmd: &str, args: &[&str]) -> anyhow::Result<String> {
        let mut command = Command::new(cmd);
        command
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::piped());
        command.env("XDG_RUNTIME_DIR", &self.xdg_runtime);
        if let Some(sock) = &self.wl_socket {
            command.env("WAYLAND_DISPLAY", sock);
        }
        let output = command.output().await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("{} failed: {}", cmd, stderr.trim());
        }
        Ok(String::from_utf8(output.stdout)?)
    }
}

#[async_trait]
impl DesktopBackend for CosmicBackend {
    // ─── Windows ────────────────────────────────────────
    async fn windows_list(&self) -> anyhow::Result<Vec<protocol::WindowInfo>> {
        let json = self.helper_json(&["list-windows"]).await?;
        // Parse the JSON array, converting cosmic helper format to protocol format
        #[derive(serde::Deserialize)]
        struct HelperWindow {
            window_id: u64,
            title: Option<String>,
            app_id: Option<String>,
            pid: Option<u32>,
            x: Option<i32>,
            y: Option<i32>,
            width: Option<u32>,
            height: Option<u32>,
            focused: bool,
            minimized: bool,
            workspace_id: Option<u32>,
        }
        let helper_windows: Vec<HelperWindow> = serde_json::from_value(json)?;
        let windows = helper_windows
            .into_iter()
            .map(|w| protocol::WindowInfo {
                id: w.window_id.to_string(),
                title: w.title.unwrap_or_default(),
                app_id: w.app_id.unwrap_or_default(),
                workspace_id: w.workspace_id.unwrap_or(0),
                is_focused: w.focused,
                is_minimized: w.minimized,
                geometry: match (w.x, w.y, w.width, w.height) {
                    (Some(x), Some(y), Some(width), Some(height)) => Some(protocol::Geometry {
                        x,
                        y,
                        width,
                        height,
                    }),
                    _ => None,
                },
                pid: w.pid,
            })
            .collect();
        Ok(windows)
    }

    async fn window_focus(&self, id: &str) -> anyhow::Result<()> {
        let nid: u64 = id.parse().unwrap_or(0);
        self.helper_run(&["activate", "--window-id", &nid.to_string()])
            .await
    }

    async fn window_get(&self, id: &str) -> anyhow::Result<protocol::WindowInfo> {
        let windows = self.windows_list().await?;
        windows
            .into_iter()
            .find(|w| w.id == id)
            .ok_or_else(|| anyhow::anyhow!("window {} not found", id))
    }

    async fn window_close(&self, id: &str) -> anyhow::Result<()> {
        let nid: u64 = id.parse().unwrap_or(0);
        self.helper_run(&["close", "--window-id", &nid.to_string()])
            .await
    }

    async fn window_minimize(&self, id: &str) -> anyhow::Result<()> {
        let nid: u64 = id.parse().unwrap_or(0);
        self.helper_run(&["minimize", "--window-id", &nid.to_string()])
            .await
    }

    async fn window_maximize(&self, id: &str) -> anyhow::Result<()> {
        let nid: u64 = id.parse().unwrap_or(0);
        self.helper_run(&["maximize", "--window-id", &nid.to_string()])
            .await
    }

    async fn window_move_resize(
        &self,
        _id: &str,
        _x: i32,
        _y: i32,
        _width: u32,
        _height: u32,
    ) -> anyhow::Result<()> {
        anyhow::bail!("window move/resize not yet supported on COSMIC")
    }

    // ─── Workspaces ─────────────────────────────────────
    async fn workspaces_list(&self) -> anyhow::Result<Vec<protocol::WorkspaceInfo>> {
        let json = self.helper_json(&["workspace-list"]).await?;
        let workspaces: Vec<protocol::WorkspaceInfo> = serde_json::from_value(json)?;
        Ok(workspaces)
    }

    async fn workspace_switch(&self, id: u32) -> anyhow::Result<()> {
        self.helper_run(&["workspace-activate", "--id", &id.to_string()])
            .await
    }

    async fn workspace_move_window(
        &self,
        window_id: &str,
        workspace_id: u32,
        _follow: bool,
    ) -> anyhow::Result<()> {
        let nid: u64 = window_id.parse().unwrap_or(0);
        self.helper_run(&[
            "move-to-workspace",
            "--window-id",
            &nid.to_string(),
            "--workspace-id",
            &workspace_id.to_string(),
        ])
        .await
    }

    // ─── Input ──────────────────────────────────────────
    async fn keyboard_type(&self, text: &str) -> anyhow::Result<()> {
        let escaped = shell_escape(text);
        self.sh("ydotool", &["type", &escaped]).await?;
        Ok(())
    }

    async fn keyboard_key(&self, key: &str) -> anyhow::Result<()> {
        self.sh("ydotool", &["key", key]).await?;
        Ok(())
    }

    async fn keyboard_combo(&self, keys: &[String]) -> anyhow::Result<()> {
        // ydotool uses + for combos like "ctrl+alt+t"
        let combo = keys.join("+");
        self.sh("ydotool", &["key", &combo]).await?;
        Ok(())
    }

    async fn mouse_move(&self, x: f64, y: f64) -> anyhow::Result<()> {
        self.sh(
            "ydotool",
            &["mousemove", "--absolute", &x.to_string(), &y.to_string()],
        )
        .await?;
        *self.last_mouse.lock().unwrap() = (x, y);
        Ok(())
    }

    async fn mouse_click(&self, button: &str) -> anyhow::Result<()> {
        let b = match button {
            "left" => "1",
            "middle" => "2",
            "right" => "3",
            _ => anyhow::bail!("unknown button: {}", button),
        };
        self.sh("ydotool", &["click", b]).await?;
        Ok(())
    }

    async fn mouse_scroll(&self, _dx: f64, dy: f64) -> anyhow::Result<()> {
        if dy >= 0.0 {
            self.sh("ydotool", &["click", "4"]).await.map(|_| ())
        } else {
            self.sh("ydotool", &["click", "5"]).await.map(|_| ())
        }
    }

    // ─── Clipboard ──────────────────────────────────────
    async fn clipboard_read(&self) -> anyhow::Result<String> {
        self.sh("wl-paste", &[]).await.map(|s| s.trim().to_string())
    }

    async fn clipboard_write(&self, text: &str) -> anyhow::Result<()> {
        self.sh("wl-copy", &[text]).await?;
        Ok(())
    }

    // ─── Screenshot ─────────────────────────────────────
    async fn screenshot(
        &self,
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
            self.sh("grim", &["-g", &geo, &path]).await?;
        } else {
            self.sh("grim", &[&path]).await?;
        }

        // Get dimensions from the file
        let dims_output = self.sh("identify", &["-format", "%w %h", &path]).await?;
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
    async fn notification_send(
        &self,
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
        self.sh("notify-send", &["-u", u, title, body]).await?;
        // notify-send doesn't return an ID; return 0
        Ok(0)
    }

    async fn notification_close(&self, _id: u32) -> anyhow::Result<()> {
        // notify-send doesn't support close by ID
        Ok(())
    }

    // ─── System ─────────────────────────────────────────
    async fn system_info(&self) -> anyhow::Result<protocol::SystemInfo> {
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

    async fn idle_seconds(&self) -> anyhow::Result<u64> {
        // Simple: check /dev/input/event* modification time
        // Fallback to 0
        Ok(0)
    }

    async fn power_action(&self, action: &str) -> anyhow::Result<()> {
        match action {
            "suspend" | "sleep" => self.sh("systemctl", &["suspend"]).await.map(|_| ()),
            "hibernate" => self.sh("systemctl", &["hibernate"]).await.map(|_| ()),
            "poweroff" | "shutdown" => self.sh("systemctl", &["poweroff"]).await.map(|_| ()),
            "reboot" => self.sh("systemctl", &["reboot"]).await.map(|_| ()),
            "lock" => self.sh("loginctl", &["lock-session"]).await.map(|_| ()),
            _ => anyhow::bail!("unknown power action: {}", action),
        }
    }

    async fn battery_status(&self) -> anyhow::Result<Vec<protocol::BatteryInfo>> {
        // Read /sys/class/power_supply/BAT*
        let mut batteries = Vec::new();
        let entries = match std::fs::read_dir("/sys/class/power_supply/") {
            Ok(rd) => rd.flatten().collect::<Vec<_>>(),
            Err(_) => {
                return Ok(batteries); // empty list on inaccessible sysfs
            }
        };

        for entry in entries
        {
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
    async fn network_status(&self) -> anyhow::Result<protocol::NetworkStatusInfo> {
        // Reuse nmcli
        let output = self.sh("nmcli", &["-t", "-f", "STATE", "general"]).await?;
        let connected = output.trim().contains("connected");
        Ok(protocol::NetworkStatusInfo {
            online: connected,
            net_type: if connected { "ethernet" } else { "none" }.to_string(),
        })
    }

    async fn network_interfaces(&self) -> anyhow::Result<Vec<protocol::NetworkInterfaceInfo>> {
        let output = self
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

    async fn wifi_scan(&self) -> anyhow::Result<Vec<protocol::WifiNetworkInfo>> {
        let output = self
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

    async fn wifi_connect(&self, ssid: &str, password: Option<&str>) -> anyhow::Result<()> {
        if let Some(pwd) = password {
            self.sh(
                "nmcli",
                &["device", "wifi", "connect", ssid, "password", pwd],
            )
            .await?;
        } else {
            self.sh("nmcli", &["device", "wifi", "connect", ssid])
                .await?;
        }
        Ok(())
    }

    // ─── Bluetooth ─────────────────────────────────────
    async fn bluetooth_list(&self) -> anyhow::Result<Vec<protocol::BluetoothDeviceInfo>> {
        Ok(vec![])
    }

    async fn bluetooth_scan(&self, _duration: Option<u32>) -> anyhow::Result<()> {
        Ok(())
    }

    async fn bluetooth_stop_scan(&self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn bluetooth_connect(&self, _address: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn bluetooth_disconnect(&self, _address: &str) -> anyhow::Result<()> {
        Ok(())
    }

    // ─── Files ──────────────────────────────────────────
    async fn files_watch(
        &self,
        _path: &str,
        _recursive: bool,
        _patterns: Option<&[String]>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn files_unwatch(&self, _path: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn files_search(
        &self,
        pattern: &str,
        _root: Option<&str>,
        max_results: u32,
    ) -> anyhow::Result<Vec<String>> {
        // Reuse `find` like the other backends
        let output = self
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
    async fn audio_list_sinks(&self) -> anyhow::Result<Vec<protocol::AudioSinkInfo>> {
        Ok(vec![])
    }

    async fn audio_set_sink_volume(&self, _sink_id: u32, _volume: f64) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Shell-escape text for ydotool (simple single-quote wrapping)
fn shell_escape(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}
