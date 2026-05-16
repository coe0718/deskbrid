use crate::protocol;
use crate::protocol::DeskbridEvent;
use async_trait::async_trait;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::process::Command;
use tokio::sync::broadcast;

// ─── HyprBackend struct ─────────────────────────────────

pub struct HyprBackend {
    /// Broadcast sender for push events to subscribed clients.
    event_tx: broadcast::Sender<DeskbridEvent>,
    /// Active file watchers keyed by path.
    watchers: Arc<Mutex<HashMap<String, notify::RecommendedWatcher>>>,
    /// Last known mouse position for relative delta calculation.
    last_mouse: std::sync::Mutex<(f64, f64)>,
    /// Cached monitor info from hyprctl monitors.
    monitors: std::sync::Mutex<Vec<protocol::MonitorInfo>>,
    /// Auto-detected Hyprland instance signature for IPC.
    instance_sig: Option<String>,
    /// Auto-detected WAYLAND_DISPLAY value.
    wl_socket: Option<String>,
    /// XDG_RUNTIME_DIR for Wayland client connections.
    xdg_runtime: String,
}

impl HyprBackend {
    pub async fn new(event_tx: broadcast::Sender<DeskbridEvent>) -> anyhow::Result<Self> {
        // Auto-detect the Hyprland instance and Wayland socket
        let (instance_sig, wl_socket) = detect_hypr_instance();
        let xdg_runtime =
            std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/run/user/1000".to_string());
        if let Some(ref sig) = instance_sig {
            if sig.is_empty() {
                eprintln!(
                    "[deskbrid] WARN: detected empty instance sig (found dirs but name empty)"
                );
            } else {
                eprintln!("[deskbrid] detected Hyprland instance: {sig}");
            }
        } else {
            eprintln!("[deskbrid] WARN: no Hyprland instance detected (xdg={xdg_runtime})");
        }

        let backend = Self {
            event_tx,
            watchers: Arc::new(Mutex::new(HashMap::new())),
            last_mouse: std::sync::Mutex::new((960.0, 540.0)),
            monitors: std::sync::Mutex::new(Vec::new()),
            instance_sig,
            wl_socket,
            xdg_runtime,
        };
        // Cache monitor list on startup
        if let Ok(monitors) = backend.monitors_inner().await
            && let Ok(mut m) = backend.monitors.lock()
        {
            *m = monitors;
        }
        Ok(backend)
    }

    // ─── hyprctl helpers ────────────────────────────────

    /// Run `hyprctl` with JSON output, return parsed JSON value.
    async fn hyprctl_json(&self, args: &[&str]) -> anyhow::Result<serde_json::Value> {
        let mut cmd = Command::new("hyprctl");
        cmd.args(args)
            .arg("-j")
            .stdin(Stdio::null())
            .stderr(Stdio::piped());
        if let Some(sig) = &self.instance_sig {
            cmd.env("HYPRLAND_INSTANCE_SIGNATURE", sig);
        }
        let output = cmd.output().await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("hyprctl failed: {}", stderr.trim());
        }
        let stdout = String::from_utf8(output.stdout)?;
        Ok(serde_json::from_str(&stdout)?)
    }

    /// Run `hyprctl dispatch` (no JSON output, just success/fail).
    async fn hyprctl_dispatch(&self, dispatch: &str) -> anyhow::Result<()> {
        let mut cmd = std::process::Command::new("hyprctl");
        cmd.arg("dispatch")
            .arg(dispatch)
            .stdin(Stdio::null())
            .stderr(Stdio::piped());
        if let Some(sig) = &self.instance_sig {
            cmd.env("HYPRLAND_INSTANCE_SIGNATURE", sig);
        }
        // Convert to tokio::process::Command for async execution
        let output = tokio::process::Command::from(cmd).output().await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("hyprctl dispatch '{}' failed: {}", dispatch, stderr.trim());
        }
        Ok(())
    }

    /// Run a shell command and return stdout.
    async fn sh(&self, cmd: &str, args: &[&str]) -> anyhow::Result<String> {
        let mut command = Command::new(cmd);
        command
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::piped());
        // Set wayland env for grim, wl-clipboard, etc.
        command.env("XDG_RUNTIME_DIR", &self.xdg_runtime);
        if let Some(sock) = &self.wl_socket {
            command.env("WAYLAND_DISPLAY", sock);
        }
        let output = command.output().await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("{} failed: {}", cmd, stderr.trim());
        }
        Ok(String::from_utf8(output.stdout)?.trim().to_string())
    }

    /// Run a command, return true if exit code is 0.
    async fn sh_ok(&self, cmd: &str, args: &[&str]) -> bool {
        let mut command = Command::new(cmd);
        command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        command.env("XDG_RUNTIME_DIR", &self.xdg_runtime);
        if let Some(sock) = &self.wl_socket {
            command.env("WAYLAND_DISPLAY", sock);
        }
        if let Some(sig) = &self.instance_sig
            && !sig.is_empty()
        {
            command.env("HYPRLAND_INSTANCE_SIGNATURE", sig);
        }
        command.status().await.map(|s| s.success()).unwrap_or(false)
    }

    // ─── Internal helpers ────────────────────────────────

    async fn monitors_inner(&self) -> anyhow::Result<Vec<protocol::MonitorInfo>> {
        let json = self.hyprctl_json(&["monitors"]).await?;
        let arr = json
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("expected array"))?;
        let mut monitors = Vec::new();
        for (i, m) in arr.iter().enumerate() {
            monitors.push(protocol::MonitorInfo {
                id: m.get("id").and_then(|v| v.as_i64()).unwrap_or(i as i64) as u32,
                name: m
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                width: m.get("width").and_then(|v| v.as_u64()).unwrap_or(1920) as u32,
                height: m.get("height").and_then(|v| v.as_u64()).unwrap_or(1080) as u32,
                scale: m.get("scale").and_then(|v| v.as_f64()).unwrap_or(1.0),
                primary: i == 0,
            });
        }
        Ok(monitors)
    }

    fn hyprctl_client_to_window(c: &serde_json::Value) -> protocol::WindowInfo {
        let geometry = protocol::Geometry {
            x: c.get("at")
                .and_then(|v| v.as_array())
                .and_then(|a| a.first())
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32,
            y: c.get("at")
                .and_then(|v| v.as_array())
                .and_then(|a| a.get(1))
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32,
            width: c
                .get("size")
                .and_then(|v| v.as_array())
                .and_then(|a| a.first())
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32,
            height: c
                .get("size")
                .and_then(|v| v.as_array())
                .and_then(|a| a.get(1))
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32,
        };
        protocol::WindowInfo {
            id: c
                .get("address")
                .and_then(|v| v.as_str())
                .unwrap_or("0")
                .to_string(),
            title: c
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            app_id: c
                .get("class")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            workspace_id: c
                .get("workspace")
                .and_then(|v| v.get("id"))
                .and_then(|v| v.as_i64())
                .unwrap_or(1) as u32,
            is_focused: c
                .get("focusHistoryID")
                .and_then(|v| v.as_i64())
                .unwrap_or(-1)
                == 0,
            is_minimized: false,
            geometry: Some(geometry),
            pid: c.get("pid").and_then(|v| v.as_u64()).map(|v| v as u32),
        }
    }
}

// ─── Trait implementation ───────────────────────────────

#[async_trait]
impl crate::backend::DesktopBackend for HyprBackend {
    // ═══════════════════════════════════════════════════════
    //  WINDOWS
    // ═══════════════════════════════════════════════════════

    async fn windows_list(&self) -> anyhow::Result<Vec<protocol::WindowInfo>> {
        let json = self.hyprctl_json(&["clients"]).await?;
        let arr = json
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("expected JSON array"))?;
        Ok(arr.iter().map(Self::hyprctl_client_to_window).collect())
    }

    async fn window_focus(&self, id: &str) -> anyhow::Result<()> {
        let windows = self.windows_list().await?;
        let id_l = id.to_lowercase();

        let target = windows
            .iter()
            .find(|w| w.id.eq_ignore_ascii_case(id))
            .or_else(|| windows.iter().find(|w| w.app_id.eq_ignore_ascii_case(id)))
            .or_else(|| windows.iter().find(|w| w.title.eq_ignore_ascii_case(id)))
            .or_else(|| {
                windows.iter().find(|w| {
                    w.app_id.to_lowercase().contains(&id_l)
                        || w.title.to_lowercase().contains(&id_l)
                })
            })
            .ok_or_else(|| anyhow::anyhow!("no window matched id: {}", id))?;

        self.hyprctl_dispatch(&format!("focuswindow address:{}", target.id))
            .await
    }

    async fn window_get(&self, id: &str) -> anyhow::Result<protocol::WindowInfo> {
        let windows = self.windows_list().await?;
        windows
            .into_iter()
            .find(|w| w.id == id || w.app_id.contains(id) || w.title.contains(id))
            .ok_or_else(|| anyhow::anyhow!("window not found: {}", id))
    }

    // ═══════════════════════════════════════════════════════
    //  WORKSPACES
    // ═══════════════════════════════════════════════════════

    async fn workspaces_list(&self) -> anyhow::Result<Vec<protocol::WorkspaceInfo>> {
        let json = self.hyprctl_json(&["workspaces"]).await?;
        let arr = json
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("expected array"))?;
        Ok(arr
            .iter()
            .map(|w| protocol::WorkspaceInfo {
                id: w.get("id").and_then(|v| v.as_i64()).unwrap_or(0) as u32,
                name: w
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                is_active: w.get("monitor").and_then(|v| v.as_str()).is_some(),
            })
            .collect())
    }

    async fn workspace_switch(&self, id: u32) -> anyhow::Result<()> {
        self.hyprctl_dispatch(&format!("workspace {}", id)).await
    }

    async fn workspace_move_window(
        &self,
        window_id: &str,
        workspace_id: u32,
        _follow: bool,
    ) -> anyhow::Result<()> {
        // Find the window to get its address
        let windows = self.windows_list().await?;
        let target = windows
            .iter()
            .find(|w| w.id == window_id || w.app_id.contains(window_id))
            .ok_or_else(|| anyhow::anyhow!("window not found: {}", window_id))?;
        // hyprctl dispatch movetoworkspacesilent <workspace>,address:<hex>
        self.hyprctl_dispatch(&format!(
            "movetoworkspacesilent {},address:{}",
            workspace_id, target.id
        ))
        .await
    }

    // ═══════════════════════════════════════════════════════
    //  INPUT  (via ydotool)
    // ═══════════════════════════════════════════════════════

    async fn keyboard_type(&self, text: &str) -> anyhow::Result<()> {
        // ydotool type types text character by character
        self.sh("ydotool", &["type", text]).await?;
        Ok(())
    }

    async fn keyboard_key(&self, key: &str) -> anyhow::Result<()> {
        let ydotool_key = ydotool_key_name(key);
        self.sh("ydotool", &["key", &ydotool_key]).await?;
        Ok(())
    }

    async fn keyboard_combo(&self, keys: &[String]) -> anyhow::Result<()> {
        if keys.is_empty() {
            return Ok(());
        }
        let combo: Vec<String> = keys.iter().map(|k| ydotool_key_name(k)).collect();
        // ydotool key combo is "key1+key2:hold"
        // For simplicity, press them in sequence: press+hold all modifiers, tap final, release
        for (i, key) in combo.iter().enumerate() {
            if i < combo.len() - 1 {
                // Modifier: press and hold
                self.sh("ydotool", &["key", &format!("{}:1", key)]).await?;
            } else {
                // Final key: tap
                self.sh("ydotool", &["key", key]).await?;
            }
        }
        // Release modifiers
        for key in combo.iter().take(combo.len().saturating_sub(1)) {
            self.sh("ydotool", &["key", &format!("{}:0", key)]).await?;
        }
        Ok(())
    }

    async fn mouse_move(&self, x: f64, y: f64) -> anyhow::Result<()> {
        let (last_x, last_y) = {
            let pos = self.last_mouse.lock().unwrap();
            *pos
        };
        let mut _dx = x - last_x;
        let mut _dy = y - last_y;
        {
            let mut pos = self.last_mouse.lock().unwrap();
            *pos = (x, y);
        }
        self.sh(
            "ydotool",
            &[
                "mousemove",
                "--absolute",
                &format!("{}", x as i32),
                &format!("{}", y as i32),
            ],
        )
        .await?;
        Ok(())
    }

    async fn mouse_click(&self, button: &str) -> anyhow::Result<()> {
        let btn_id = match button {
            "left" => "0",
            "middle" => "1",
            "right" => "2",
            _ => anyhow::bail!("unknown button: {}", button),
        };
        self.sh("ydotool", &["click", btn_id]).await?;
        Ok(())
    }

    async fn mouse_scroll(&self, dx: f64, dy: f64) -> anyhow::Result<()> {
        if dx == 0.0 && dy == 0.0 {
            return Ok(());
        }
        // ydotool mousemove --wheel <horizontal> <vertical>
        // Positive dy = scroll down, positive dx = scroll right
        self.sh(
            "ydotool",
            &[
                "mousemove",
                "--wheel",
                &format!("{}", dx as i32),
                &format!("{}", dy as i32),
            ],
        )
        .await?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    //  CLIPBOARD  (wl-clipboard)
    // ═══════════════════════════════════════════════════════

    async fn clipboard_read(&self) -> anyhow::Result<String> {
        self.sh("wl-paste", &[]).await
    }

    async fn clipboard_write(&self, text: &str) -> anyhow::Result<()> {
        let mut child = Command::new("wl-copy")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;
        use tokio::io::AsyncWriteExt;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes()).await?;
        }
        let status = child.wait().await?;
        if !status.success() {
            anyhow::bail!("wl-copy failed");
        }
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    //  SCREENSHOT  (grim)
    // ═══════════════════════════════════════════════════════

    async fn screenshot(
        &self,
        monitor: Option<u32>,
        region: Option<protocol::Region>,
        window_id: Option<String>,
    ) -> anyhow::Result<protocol::ScreenshotResult> {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        let path = format!("/tmp/deskbrid_screenshot_{}.png", ts);

        // Window screenshot via geometry
        if let Some(ref wid) = window_id {
            let info = self.window_get(wid).await?;
            if let Some(geo) = info.geometry {
                let region_str = format!("{}x{}+{}+{}", geo.width, geo.height, geo.x, geo.y);
                self.sh("grim", &["-g", &region_str, &path]).await?;
                return Ok(protocol::ScreenshotResult {
                    path: path.clone(),
                    width: geo.width,
                    height: geo.height,
                    format: "png".into(),
                });
            }
        }

        // Region screenshot
        if let Some(ref r) = region {
            let region_str = format!("{}x{}+{}+{}", r.width, r.height, r.x, r.y);
            self.sh("grim", &["-g", &region_str, &path]).await?;
            return Ok(protocol::ScreenshotResult {
                path: path.clone(),
                width: r.width,
                height: r.height,
                format: "png".into(),
            });
        }

        // Full screen or specific monitor
        if let Some(idx) = monitor {
            let monitors = {
                let m = self.monitors.lock().unwrap();
                m.clone()
            };
            let name = monitors
                .get(idx as usize)
                .map(|m| m.name.clone())
                .unwrap_or_else(|| idx.to_string());
            self.sh("grim", &["-o", &name, &path]).await?;
        } else {
            self.sh("grim", &[&path]).await?;
        }

        // Get dimensions from the file
        let dims = get_png_dimensions(&path)?;
        Ok(protocol::ScreenshotResult {
            path,
            width: dims.0,
            height: dims.1,
            format: "png".into(),
        })
    }

    // ═══════════════════════════════════════════════════════
    //  NOTIFICATIONS  (freedesktop DBus)
    // ═══════════════════════════════════════════════════════

    async fn notification_send(
        &self,
        app_name: &str,
        title: &str,
        body: &str,
        urgency: &str,
    ) -> anyhow::Result<u32> {
        let urgency_byte = match urgency {
            "low" => "low",
            "normal" => "normal",
            "critical" => "critical",
            _ => "normal",
        };
        let output = self
            .sh(
                "notify-send",
                &[
                    "--app-name",
                    app_name,
                    "--urgency",
                    urgency_byte,
                    "--print-id",
                    title,
                    body,
                ],
            )
            .await?;
        let id: u32 = output.parse().unwrap_or(0);
        Ok(id)
    }

    async fn notification_close(&self, id: u32) -> anyhow::Result<()> {
        // notify-send doesn't have close by ID. Use makoctl if available, else ignore.
        if self.sh_ok("makoctl", &["dismiss", &id.to_string()]).await {
            return Ok(());
        }
        // Non-fatal — notification will expire naturally
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    //  SYSTEM
    // ═══════════════════════════════════════════════════════

    async fn system_info(&self) -> anyhow::Result<protocol::SystemInfo> {
        let version = self
            .hyprctl_json(&["version"])
            .await
            .map(|v| {
                // hyprctl -j version returns: {"version":"v0.54.3","branch":"v0.54.3",...}
                v.get("version")
                    .and_then(|s| s.as_str())
                    .unwrap_or("unknown")
                    .to_string()
            })
            .unwrap_or_else(|_| "unknown".into());

        let session_type = if self.wl_socket.is_some() {
            "wayland"
        } else if std::env::var("DISPLAY").is_ok() {
            "x11"
        } else {
            "unknown"
        };

        let monitors = {
            let m = self.monitors.lock().unwrap();
            m.clone()
        };
        let workspaces = self.workspaces_list().await.unwrap_or_default();
        let workspace_count = workspaces.len() as u32;
        let current_workspace = workspaces
            .iter()
            .find(|w| w.is_active)
            .map(|w| w.id)
            .unwrap_or(1);
        let idle_seconds = self.idle_seconds_inner().await.unwrap_or(0);

        Ok(protocol::SystemInfo {
            desktop: "Hyprland".into(),
            desktop_version: version,
            compositor: "hyprland".into(),
            session_type: session_type.into(),
            monitors,
            workspace_count,
            current_workspace,
            idle_seconds,
        })
    }

    async fn idle_seconds(&self) -> anyhow::Result<u64> {
        self.idle_seconds_inner().await
    }

    async fn power_action(&self, action: &str) -> anyhow::Result<()> {
        match action {
            "suspend" => {
                self.sh("systemctl", &["suspend"]).await?;
            }
            "hibernate" => {
                self.sh("systemctl", &["hibernate"]).await?;
            }
            "shutdown" | "poweroff" => {
                self.sh("systemctl", &["poweroff"]).await?;
            }
            "reboot" | "restart" => {
                self.sh("systemctl", &["reboot"]).await?;
            }
            "lock" => {
                // Try multiple lockers in order
                if !self.sh_ok("loginctl", &["lock-session"]).await {
                    self.sh("hyprctl", &["dispatch", "exec", "loginctl lock-session"])
                        .await?;
                }
            }
            "logout" => {
                self.sh("hyprctl", &["dispatch", "exit"]).await?;
            }
            _ => anyhow::bail!("unsupported power action: {}", action),
        }
        Ok(())
    }

    async fn battery_status(&self) -> anyhow::Result<Vec<protocol::BatteryInfo>> {
        // Parse /sys/class/power_supply for battery info
        let mut batteries = Vec::new();
        let dirs = if let Ok(entries) = std::fs::read_dir("/sys/class/power_supply") {
            entries
        } else {
            return Ok(batteries);
        };

        for entry in dirs.flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if !name.starts_with("BAT") {
                continue;
            }

            let read_sys =
                |file: &str| -> Option<String> { std::fs::read_to_string(path.join(file)).ok() };

            let capacity = read_sys("capacity")
                .and_then(|s| s.trim().parse::<f64>().ok())
                .unwrap_or(0.0);
            let status = read_sys("status")
                .map(|s| s.trim().to_lowercase())
                .unwrap_or_else(|| "unknown".into());
            let energy_now = read_sys("energy_now")
                .and_then(|s| s.trim().parse::<f64>().ok())
                .unwrap_or(0.0);
            let power_now = read_sys("power_now")
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

    // ═══════════════════════════════════════════════════════
    //  NETWORK  (NetworkManager via nmcli)
    // ═══════════════════════════════════════════════════════

    async fn network_status(&self) -> anyhow::Result<protocol::NetworkStatusInfo> {
        let online = if self
            .sh_ok("nmcli", &["networking", "connectivity", "check"])
            .await
        {
            true
        } else {
            self.sh_ok("ping", &["-c", "1", "-W", "2", "8.8.8.8"]).await
        };

        Ok(protocol::NetworkStatusInfo {
            online,
            net_type: if online {
                "wifi_or_ethernet".into()
            } else {
                "offline".into()
            },
        })
    }

    async fn network_interfaces(&self) -> anyhow::Result<Vec<protocol::NetworkInterfaceInfo>> {
        // Parse `nmcli -t -f DEVICE,STATE,IP4.ADDRESS dev status`
        let output = self
            .sh(
                "nmcli",
                &["-t", "-f", "DEVICE,STATE,IP4.ADDRESS", "dev", "status"],
            )
            .await
            .unwrap_or_default();

        let mut ifaces = Vec::new();
        for line in output.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() < 2 {
                continue;
            }
            let name = parts[0].to_string();
            if name == "lo" || name.is_empty() {
                continue;
            }
            let state = match *parts.get(1).unwrap_or(&"") {
                "connected" => "connected".to_string(),
                "connecting" => "connecting".to_string(),
                _ => "disconnected".to_string(),
            };
            let ipv4 = parts
                .get(2)
                .filter(|s| !s.is_empty())
                .map(|s| s.split('/').next().unwrap_or(s).to_string());

            ifaces.push(protocol::NetworkInterfaceInfo {
                name,
                state,
                ipv4,
                ipv6: None,
            });
        }

        Ok(ifaces)
    }

    async fn wifi_scan(&self) -> anyhow::Result<Vec<protocol::WifiNetworkInfo>> {
        // Trigger scan then list
        self.sh("nmcli", &["dev", "wifi", "rescan"]).await.ok();
        let output = self
            .sh(
                "nmcli",
                &["-t", "-f", "SSID,SIGNAL,SECURITY", "dev", "wifi", "list"],
            )
            .await
            .unwrap_or_default();

        let mut networks = Vec::new();
        for line in output.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.is_empty() || parts[0].is_empty() {
                continue;
            }
            let ssid = parts[0].to_string();
            let signal: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
            let security = parts.get(2).unwrap_or(&"").to_string();

            networks.push(protocol::WifiNetworkInfo {
                ssid,
                strength: signal,
                secured: !security.is_empty() && security != "--",
                frequency: None,
            });
        }
        Ok(networks)
    }

    async fn wifi_connect(&self, ssid: &str, password: Option<&str>) -> anyhow::Result<()> {
        match password {
            Some(pw) => {
                self.sh("nmcli", &["dev", "wifi", "connect", ssid, "password", pw])
                    .await?;
            }
            None => {
                self.sh("nmcli", &["dev", "wifi", "connect", ssid]).await?;
            }
        }
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    //  BLUETOOTH  (via bluetoothctl)
    // ═══════════════════════════════════════════════════════

    async fn bluetooth_list(&self) -> anyhow::Result<Vec<protocol::BluetoothDeviceInfo>> {
        let output = self
            .sh("bluetoothctl", &["devices"])
            .await
            .unwrap_or_default();
        let mut devices = Vec::new();
        for line in output.lines() {
            // Format: Device MAC Name
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            if parts.len() >= 3 {
                devices.push(protocol::BluetoothDeviceInfo {
                    name: parts[2].to_string(),
                    address: parts[1].to_string(),
                    connected: false,
                    paired: true,
                    rssi: None,
                });
            }
        }
        Ok(devices)
    }

    async fn bluetooth_scan(&self, _duration: Option<u32>) -> anyhow::Result<()> {
        // Start scan (non-blocking, runs until stopped)
        self.sh("bluetoothctl", &["scan", "on"]).await.ok();
        Ok(())
    }

    async fn bluetooth_stop_scan(&self) -> anyhow::Result<()> {
        self.sh("bluetoothctl", &["scan", "off"]).await.ok();
        Ok(())
    }

    async fn bluetooth_connect(&self, address: &str) -> anyhow::Result<()> {
        self.sh("bluetoothctl", &["connect", address]).await?;
        Ok(())
    }

    async fn bluetooth_disconnect(&self, address: &str) -> anyhow::Result<()> {
        self.sh("bluetoothctl", &["disconnect", address]).await?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    //  FILES
    // ═══════════════════════════════════════════════════════

    async fn files_watch(
        &self,
        path: &str,
        recursive: bool,
        patterns: Option<&[String]>,
    ) -> anyhow::Result<()> {
        use notify::*;

        let event_tx = self.event_tx.clone();
        let watch_path = path.to_string();
        let recursive_mode = if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        let _pattern_filter = patterns.map(|p| p.to_vec());

        let mut watcher =
            notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                if let Ok(event) = res {
                    let _path_str = event.paths.first().map(|p| p.to_string_lossy().to_string());
                    let event_kind = if event.kind.is_create() {
                        "create"
                    } else if event.kind.is_modify() {
                        "modify"
                    } else if event.kind.is_remove() {
                        "remove"
                    } else {
                        "other"
                    };

                    let path_str = event.paths.first().map(|p| p.to_string_lossy().to_string());
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();

                    match event_kind {
                        "create" => {
                            let _ = event_tx.send(DeskbridEvent::FileCreated {
                                path: path_str.unwrap_or_default(),
                                timestamp: ts,
                            });
                        }
                        "modify" => {
                            let _ = event_tx.send(DeskbridEvent::FileModified {
                                path: path_str.unwrap_or_default(),
                                timestamp: ts,
                            });
                        }
                        "remove" => {
                            let _ = event_tx.send(DeskbridEvent::FileDeleted {
                                path: path_str.unwrap_or_default(),
                                timestamp: ts,
                            });
                        }
                        _ => {}
                    }
                }
            })?;

        watcher.watch(std::path::Path::new(&watch_path), recursive_mode)?;

        let mut watchers = self.watchers.lock().unwrap();
        watchers.insert(watch_path, watcher);
        Ok(())
    }

    async fn files_unwatch(&self, path: &str) -> anyhow::Result<()> {
        let mut watchers = self.watchers.lock().unwrap();
        watchers.remove(path);
        Ok(())
    }

    async fn files_search(
        &self,
        pattern: &str,
        root: Option<&str>,
        max_results: u32,
    ) -> anyhow::Result<Vec<String>> {
        let root_path = root.unwrap_or(".");
        // Use find — no shell pipes, truncate results in Rust
        let output = self
            .sh(
                "find",
                &[root_path, "-type", "f", "-name", pattern, "-maxdepth", "5"],
            )
            .await
            .unwrap_or_default();

        Ok(output
            .lines()
            .take(max_results as usize)
            .map(|l| l.to_string())
            .collect())
    }

    // ═══════════════════════════════════════════════════════
    //  AUDIO
    // ═══════════════════════════════════════════════════════

    async fn audio_list_sinks(&self) -> anyhow::Result<Vec<protocol::AudioSinkInfo>> {
        // Use pactl list sinks short (PipeWire provides pactl compat)
        let output = self
            .sh("pactl", &["list", "short", "sinks"])
            .await
            .unwrap_or_default();
        let mut sinks = Vec::new();
        for line in output.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 {
                sinks.push(protocol::AudioSinkInfo {
                    id: parts[0].parse().unwrap_or(0),
                    name: parts[1].to_string(),
                    description: String::new(),
                    volume: 1.0,
                    muted: false,
                });
            }
        }
        Ok(sinks)
    }

    async fn audio_set_sink_volume(&self, sink_id: u32, volume: f64) -> anyhow::Result<()> {
        let vol_pct = (volume * 100.0) as u32;
        self.sh(
            "pactl",
            &[
                "set-sink-volume",
                &sink_id.to_string(),
                &format!("{}%", vol_pct),
            ],
        )
        .await?;
        Ok(())
    }
}

// ─── HyprBackend internal methods ───────────────────────

impl HyprBackend {
    /// Idle detection via /dev/input event timestamps.
    async fn idle_seconds_inner(&self) -> anyhow::Result<u64> {
        let mut newest: u64 = 0;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        if let Ok(entries) = std::fs::read_dir("/dev/input") {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with("event")
                    && let Ok(meta) = entry.metadata()
                    && let Ok(modified) = meta.modified()
                    && let Ok(ts) = modified.duration_since(std::time::UNIX_EPOCH)
                {
                    let secs = ts.as_secs();
                    if secs > newest && secs <= now {
                        newest = secs;
                    }
                }
            }
        }

        if newest > 0 {
            Ok(now.saturating_sub(newest))
        } else {
            Ok(0)
        }
    }
}

// ─── Detect Hyprland instance ────────────────────────────

/// Auto-detect the running Hyprland instance and Wayland display
/// by scanning the Hyprland socket directory.
fn detect_hypr_instance() -> (Option<String>, Option<String>) {
    let xdg_runtime =
        std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/run/user/1000".to_string());
    let hypr_dir = std::path::Path::new(&xdg_runtime).join("hypr");

    let entries = match std::fs::read_dir(&hypr_dir) {
        Ok(e) => e,
        Err(_) => return (None, None),
    };

    // Find the most recent instance by directory mtime
    let mut instances: Vec<(std::path::PathBuf, std::time::SystemTime)> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| {
            e.metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|t| (e.path(), t))
        })
        .collect();

    instances.sort_by_key(|item| std::cmp::Reverse(item.1));

    if let Some((path, _)) = instances.first() {
        let sig = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string());
        // Try to read .wayland_socket symlink, else default to "wayland-1"
        let wl_sock = std::fs::read_link(path.join(".wayland_socket"))
            .ok()
            .and_then(|p| {
                p.file_name()
                    .and_then(|n| n.to_str().map(|s| s.to_string()))
            })
            .or_else(|| Some("wayland-1".to_string()));

        (sig, wl_sock)
    } else {
        (None, None)
    }
}

// ─── Helpers ────────────────────────────────────────────

/// Map a human-readable key name to ydotool keycode name.
fn ydotool_key_name(key: &str) -> String {
    match key.to_lowercase().as_str() {
        "return" | "enter" => "ENTER".into(),
        "tab" => "TAB".into(),
        "escape" | "esc" => "ESC".into(),
        "backspace" => "BACKSPACE".into(),
        "delete" | "del" => "DELETE".into(),
        "up" => "UP".into(),
        "down" => "DOWN".into(),
        "left" => "LEFT".into(),
        "right" => "RIGHT".into(),
        "home" => "HOME".into(),
        "end" => "END".into(),
        "page_up" | "pgup" => "PAGEUP".into(),
        "page_down" | "pgdn" => "PAGEDOWN".into(),
        "space" => "SPACE".into(),
        "shift" | "shift_l" | "shift_r" => "LEFTSHIFT".into(),
        "ctrl" | "control" | "control_l" | "ctrl_l" => "LEFTCTRL".into(),
        "alt" | "alt_l" => "LEFTALT".into(),
        "super" | "super_l" | "meta" | "win" | "windows" => "LEFTMETA".into(),
        other => other.to_string(),
    }
}

/// Simple PNG header parser for dimensions.
fn get_png_dimensions(path: &str) -> anyhow::Result<(u32, u32)> {
    let data = std::fs::read(path)?;
    if data.len() < 24 || &data[1..4] != b"PNG" {
        anyhow::bail!("not a PNG file");
    }
    let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    Ok((width, height))
}
