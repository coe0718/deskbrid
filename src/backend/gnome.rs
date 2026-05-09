use crate::protocol;
use crate::protocol::{Geometry, Region};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;
use tracing::debug;
use zbus::zvariant;

// ─── Backend struct ────────────────────────────────────

pub struct GnomeBackend {
    /// DBus session connection for standard freedesktop interfaces.
    conn: zbus::Connection,
}

impl GnomeBackend {
    pub async fn new() -> anyhow::Result<Self> {
        let conn = zbus::Connection::session().await?;
        Ok(Self { conn })
    }

    // ─── Shell helpers ──────────────────────────────────

    /// Run a command, return stdout as String. Fails on non-zero exit.
    async fn sh(&self, cmd: &str, args: &[&str]) -> anyhow::Result<String> {
        let output = Command::new(cmd)
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("{} failed: {}", cmd, stderr.trim());
        }
        Ok(String::from_utf8(output.stdout)?.trim().to_string())
    }

    /// Run a command, return true if exit code is 0 (ignore output).
    async fn sh_ok(&self, cmd: &str, args: &[&str]) -> bool {
        Command::new(cmd)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    // ─── Extension DBus helpers ─────────────────────────

    /// Path to the GNOME Shell extension's DBus object.
    const EXT_BUS: &'static str = "org.gnome.Shell.Extensions.Deskbrid";
    const EXT_PATH: &'static str = "/org/gnome/Shell/Extensions/Deskbrid";
    const EXT_IFACE: &'static str = "org.gnome.Shell.Extensions.Deskbrid";

    /// Call an extension DBus method via gdbus. Returns raw string.
    async fn ext_call_parsed(&self, method: &str, extra_args: &[&str]) -> anyhow::Result<String> {
        let method_full = format!("{}.{}", Self::EXT_IFACE, method);
        let mut args = vec![
            "call",
            "--session",
            "--dest",
            Self::EXT_BUS,
            "--object-path",
            Self::EXT_PATH,
            "--method",
            &method_full,
        ];
        args.extend(extra_args);
        self.sh("gdbus", &args).await
    }
}

// ─── Trait implementation ───────────────────────────────

#[async_trait]
impl crate::backend::DesktopBackend for GnomeBackend {
    // ═══════════════════════════════════════════════════════
    //  WINDOWS
    // ═══════════════════════════════════════════════════════

    async fn windows_list(&self) -> anyhow::Result<Vec<protocol::WindowInfo>> {
        // Call extension: WindowsList() → a(sua{sv})
        let raw = self.ext_call_parsed("WindowsList", &[]).await?;
        parse_gdbus_window_list(&raw)
    }

    async fn window_focus(&self, id: &str) -> anyhow::Result<()> {
        self.ext_call_parsed("FocusWindow", &[id]).await?;
        Ok(())
    }

    async fn window_get(&self, id: &str) -> anyhow::Result<protocol::WindowInfo> {
        let raw = self.ext_call_parsed("GetWindow", &[id]).await?;
        parse_gdbus_single_window(&raw)
    }

    // ═══════════════════════════════════════════════════════
    //  WORKSPACES
    // ═══════════════════════════════════════════════════════

    async fn workspaces_list(&self) -> anyhow::Result<Vec<protocol::WorkspaceInfo>> {
        let raw = self.ext_call_parsed("WorkspacesList", &[]).await?;
        // gdbus returns: [('Workspace 1', true), ('Workspace 2', false)]
        parse_gdbus_workspace_list(&raw)
    }

    async fn workspace_switch(&self, id: u32) -> anyhow::Result<()> {
        self.ext_call_parsed("SwitchWorkspace", &[&id.to_string()])
            .await?;
        Ok(())
    }

    async fn workspace_move_window(
        &self,
        window_id: &str,
        workspace_id: u32,
        follow: bool,
    ) -> anyhow::Result<()> {
        self.ext_call_parsed(
            "MoveWindowToWorkspace",
            &[window_id, &workspace_id.to_string(), &follow.to_string()],
        )
        .await?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    //  INPUT
    // ═══════════════════════════════════════════════════════

    async fn keyboard_type(&self, text: &str) -> anyhow::Result<()> {
        // wtype types literal text
        self.sh("wtype", &[text]).await?;
        Ok(())
    }

    async fn keyboard_key(&self, key: &str) -> anyhow::Result<()> {
        self.sh("wtype", &["-k", key]).await?;
        Ok(())
    }

    async fn keyboard_combo(&self, keys: &[String]) -> anyhow::Result<()> {
        let mut args: Vec<&str> = Vec::new();
        let key_refs: Vec<String> = keys.iter().map(|k| k.to_lowercase()).collect();
        for k in &key_refs {
            args.push("-k");
            args.push(k);
        }
        self.sh("wtype", &args).await?;
        Ok(())
    }

    async fn mouse_move(&self, x: f64, y: f64) -> anyhow::Result<()> {
        self.sh(
            "ydotool",
            &["mousemove", "--absolute", &x.to_string(), &y.to_string()],
        )
        .await?;
        Ok(())
    }

    async fn mouse_click(&self, button: &str) -> anyhow::Result<()> {
        let btn = match button {
            "left" => "0xC0",
            "middle" => "0xC1",
            "right" => "0xC2",
            _ => anyhow::bail!("unknown button: {}", button),
        };
        // ydotool click <button> ; ydotool click --repeat 1 --next-delay 50 <button>
        self.sh("ydotool", &["click", btn]).await?;
        Ok(())
    }

    async fn mouse_scroll(&self, dx: f64, dy: f64) -> anyhow::Result<()> {
        if dy != 0.0 {
            let dir = if dy > 0.0 { "down" } else { "up" };
            let steps = dy.abs() as u32;
            for _ in 0..steps {
                self.sh(
                    "ydotool",
                    &["click", if dir == "up" { "0x40" } else { "0x41" }],
                )
                .await?;
            }
        }
        if dx != 0.0 {
            let dir = if dx > 0.0 { "right" } else { "left" };
            let steps = dx.abs() as u32;
            for _ in 0..steps {
                self.sh(
                    "ydotool",
                    &["click", if dir == "right" { "0x42" } else { "0x43" }],
                )
                .await?;
            }
        }
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    //  CLIPBOARD
    // ═══════════════════════════════════════════════════════

    async fn clipboard_read(&self) -> anyhow::Result<String> {
        self.sh("wl-paste", &[]).await
    }

    async fn clipboard_write(&self, text: &str) -> anyhow::Result<()> {
        // Pipe text into wl-copy
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
    //  SCREENSHOT
    // ═══════════════════════════════════════════════════════

    async fn screenshot(
        &self,
        monitor: Option<u32>,
        region: Option<Region>,
        window_id: Option<String>,
    ) -> anyhow::Result<protocol::ScreenshotResult> {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        let path = format!("/tmp/deskbrid_screenshot_{}.png", ts);

        // If window_id is set, use grim -g based on window geometry
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

        // Full screen (or specific monitor)
        if let Some(idx) = monitor {
            self.sh("grim", &["-o", &idx.to_string(), &path]).await?;
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
    //  NOTIFICATIONS
    // ═══════════════════════════════════════════════════════

    async fn notification_send(
        &self,
        app_name: &str,
        title: &str,
        body: &str,
        urgency: &str,
    ) -> anyhow::Result<u32> {
        let urgency_byte = match urgency {
            "low" => 0u8,
            "normal" => 1u8,
            "critical" => 2u8,
            _ => 1u8,
        };

        // org.freedesktop.Notifications.Notify(
        //   app_name, replaces_id, app_icon, summary, body,
        //   actions, hints, expire_timeout
        // ) → u32 (notification ID)
        let reply = self
            .conn
            .call_method(
                Some("org.freedesktop.Notifications"),
                "/org/freedesktop/Notifications",
                Some("org.freedesktop.Notifications"),
                "Notify",
                &(
                    app_name,
                    0u32,           // replaces_id
                    "",             // app_icon
                    title,          // summary
                    body,           // body
                    &[] as &[&str], // actions
                    &[("urgency", zvariant::Value::U8(urgency_byte))]
                        as &[(&str, zvariant::Value)],
                    5000i32, // expire_timeout ms (-1 = default)
                ),
            )
            .await?;
        let id: u32 = reply.body().deserialize()?;
        Ok(id)
    }

    async fn notification_close(&self, id: u32) -> anyhow::Result<()> {
        self.conn
            .call_method(
                Some("org.freedesktop.Notifications"),
                "/org/freedesktop/Notifications",
                Some("org.freedesktop.Notifications"),
                "CloseNotification",
                &(id,),
            )
            .await?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    //  SYSTEM
    // ═══════════════════════════════════════════════════════

    async fn system_info(&self) -> anyhow::Result<protocol::SystemInfo> {
        let _hostname = self.sh("hostname", &[]).await.unwrap_or_default();
        let version = self
            .sh("gnome-shell", &["--version"])
            .await
            .unwrap_or_else(|_| "unknown".into());
        // gnome-shell --version outputs "GNOME Shell 46.2" or similar
        let version = version
            .strip_prefix("GNOME Shell ")
            .unwrap_or(&version)
            .to_string();

        // Detect session type
        let session_type = if std::env::var("WAYLAND_DISPLAY").is_ok() {
            "wayland"
        } else if std::env::var("DISPLAY").is_ok() {
            "x11"
        } else {
            "unknown"
        };

        // Monitor info from Mutter DBus
        let monitors = self.get_monitors().await?;
        let workspace_count = self.get_workspace_count().await?;
        let current_workspace = self.get_current_workspace().await?;
        let idle_seconds = self.idle_seconds_inner().await.unwrap_or(0);

        Ok(protocol::SystemInfo {
            desktop: "GNOME".into(),
            desktop_version: version,
            compositor: "mutter".into(),
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
                // GNOME 46: use loginctl lock-session
                self.sh("loginctl", &["lock-session"]).await?;
            }
            "logout" => {
                self.sh("gnome-session-quit", &["--logout", "--no-prompt"])
                    .await?;
            }
            _ => anyhow::bail!("unsupported power action: {}", action),
        }
        Ok(())
    }

    async fn battery_status(&self) -> anyhow::Result<Vec<protocol::BatteryInfo>> {
        // Query UPower devices
        let reply = self
            .conn
            .call_method(
                Some("org.freedesktop.UPower"),
                "/org/freedesktop/UPower",
                Some("org.freedesktop.UPower"),
                "EnumerateDevices",
                &(),
            )
            .await?;
        let paths: Vec<zvariant::OwnedObjectPath> = reply.body().deserialize()?;

        let mut batteries = Vec::new();
        for path in &paths {
            let path_str = path.as_str();
            // Only process battery devices
            let type_reply = self
                .conn
                .call_method(
                    Some("org.freedesktop.UPower"),
                    path_str,
                    Some("org.freedesktop.DBus.Properties"),
                    "Get",
                    &("org.freedesktop.UPower.Device", "Type"),
                )
                .await;

            if let Ok(reply) = type_reply {
                let type_val: u32 = reply.body().deserialize().unwrap_or(0);
                if type_val != 2 {
                    // 2 = Battery
                    continue;
                }
            } else {
                continue;
            }

            // Get percentage and state
            let pct: f64 = self
                .get_upower_property(path_str, "Percentage")
                .await
                .unwrap_or(0.0);
            let state_val: u32 = self
                .get_upower_property(path_str, "State")
                .await
                .unwrap_or(0);
            let energy_rate: f64 = self
                .get_upower_property(path_str, "EnergyRate")
                .await
                .unwrap_or(0.0);
            let energy: f64 = self
                .get_upower_property(path_str, "Energy")
                .await
                .unwrap_or(0.0);

            let state = match state_val {
                1 => "charging",
                2 => "discharging",
                4 => "fully_charged",
                _ => "unknown",
            };

            let time_remaining = if state == "discharging" && energy_rate > 0.0 {
                Some(((energy / energy_rate) * 60.0) as u32)
            } else if state == "charging" && energy_rate > 0.0 {
                let remaining_energy = energy * (100.0 - pct) / 100.0;
                Some(((remaining_energy / energy_rate) * 60.0) as u32)
            } else {
                None
            };

            batteries.push(protocol::BatteryInfo {
                source: path_str.to_string(),
                percentage: pct,
                state: state.into(),
                time_remaining_minutes: time_remaining,
            });
        }

        Ok(batteries)
    }

    // ═══════════════════════════════════════════════════════
    //  NETWORK (stubs for Phase 2b)
    // ═══════════════════════════════════════════════════════

    async fn network_status(&self) -> anyhow::Result<protocol::NetworkStatusInfo> {
        // Quick ping check for basic online status
        let online = self.sh_ok("ping", &["-c", "1", "-W", "2", "8.8.8.8"]).await;
        Ok(protocol::NetworkStatusInfo {
            online,
            net_type: if online {
                "unknown".into()
            } else {
                "offline".into()
            },
        })
    }

    async fn network_interfaces(&self) -> anyhow::Result<Vec<protocol::NetworkInterfaceInfo>> {
        // Parse /proc/net/dev for basic interface list
        let out = self.sh("cat", &["/proc/net/dev"]).await.unwrap_or_default();
        let mut ifaces = Vec::new();
        for line in out.lines().skip(2) {
            let name = line.split(':').next().unwrap_or("").trim();
            if name.is_empty() || name == "lo" {
                continue;
            }
            ifaces.push(protocol::NetworkInterfaceInfo {
                name: name.to_string(),
                state: "up".into(),
                ipv4: None,
                ipv6: None,
            });
        }
        Ok(ifaces)
    }

    async fn wifi_scan(&self) -> anyhow::Result<Vec<protocol::WifiNetworkInfo>> {
        // Stub — NetworkManager DBus in Phase 2b
        Ok(vec![])
    }

    async fn wifi_connect(&self, _ssid: &str, _password: Option<&str>) -> anyhow::Result<()> {
        anyhow::bail!("WiFi connect not yet implemented (Phase 2b)")
    }

    // ═══════════════════════════════════════════════════════
    //  BLUETOOTH (stubs for Phase 2b)
    // ═══════════════════════════════════════════════════════

    async fn bluetooth_list(&self) -> anyhow::Result<Vec<protocol::BluetoothDeviceInfo>> {
        Ok(vec![])
    }

    async fn bluetooth_scan(&self, _duration: Option<u32>) -> anyhow::Result<()> {
        anyhow::bail!("Bluetooth scan not yet implemented (Phase 2b)")
    }

    async fn bluetooth_stop_scan(&self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn bluetooth_connect(&self, _address: &str) -> anyhow::Result<()> {
        anyhow::bail!("Bluetooth connect not yet implemented (Phase 2b)")
    }

    async fn bluetooth_disconnect(&self, _address: &str) -> anyhow::Result<()> {
        anyhow::bail!("Bluetooth disconnect not yet implemented (Phase 2b)")
    }

    // ═══════════════════════════════════════════════════════
    //  FILES
    // ═══════════════════════════════════════════════════════

    async fn files_watch(
        &self,
        path: &str,
        recursive: bool,
        _patterns: Option<&[String]>,
    ) -> anyhow::Result<()> {
        // We stash the watch registration in daemon.rs ConnectionState.
        // The actual notify watcher will be managed by the event loop (Phase 2c).
        // For now, validate the path exists.
        let meta = tokio::fs::metadata(path).await?;
        if !meta.is_dir() && !meta.is_file() {
            anyhow::bail!("path does not exist: {}", path);
        }
        debug!(
            "Registered file watch on {} (recursive={})",
            path, recursive
        );
        Ok(())
    }

    async fn files_unwatch(&self, path: &str) -> anyhow::Result<()> {
        debug!("Unregistered file watch on {}", path);
        Ok(())
    }

    async fn files_search(
        &self,
        pattern: &str,
        root: Option<&str>,
        max_results: u32,
    ) -> anyhow::Result<Vec<String>> {
        let base = root.unwrap_or(".");
        // Use fd if available, fall back to find
        if self.sh_ok("which", &["fd"]).await {
            let out = self
                .sh(
                    "fd",
                    &[
                        "--max-results",
                        &max_results.to_string(),
                        "--search-path",
                        base,
                        pattern,
                    ],
                )
                .await?;
            Ok(out.lines().map(|l| l.to_string()).collect())
        } else {
            let out = self
                .sh("find", &[base, "-name", pattern, "-maxdepth", "10"])
                .await?;
            let lines: Vec<String> = out
                .lines()
                .take(max_results as usize)
                .map(|l| l.to_string())
                .collect();
            Ok(lines)
        }
    }

    // ═══════════════════════════════════════════════════════
    //  AUDIO
    // ═══════════════════════════════════════════════════════

    async fn audio_list_sinks(&self) -> anyhow::Result<Vec<protocol::AudioSinkInfo>> {
        // Use pactl for PipeWire-PulseAudio compat
        let out = self.sh("pactl", &["list", "sinks"]).await?;
        parse_pactl_sinks(&out)
    }

    async fn audio_set_sink_volume(&self, sink_id: u32, volume: f64) -> anyhow::Result<()> {
        // pactl set-sink-volume <id> <volume>%
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

// ─── Private helpers ─────────────────────────────────────

impl GnomeBackend {
    async fn idle_seconds_inner(&self) -> anyhow::Result<u64> {
        let reply = self
            .conn
            .call_method(
                Some("org.gnome.Mutter.IdleMonitor"),
                "/org/gnome/Mutter/IdleMonitor/Core",
                Some("org.gnome.Mutter.IdleMonitor"),
                "GetIdletime",
                &(),
            )
            .await?;
        let ms: u64 = reply.body().deserialize()?;
        Ok(ms / 1000)
    }

    async fn get_monitors(&self) -> anyhow::Result<Vec<protocol::MonitorInfo>> {
        // Use extension if available, otherwise basic xrandr/wlr-randr
        // Try extension first
        if let Ok(raw) = self.ext_call_parsed("MonitorsList", &[]).await {
            if let Ok(monitors) = parse_gdbus_monitor_list(&raw) {
                return Ok(monitors);
            }
        }

        // Fallback: parse wlr-randr or just return a single placeholder
        let mut monitors = Vec::new();
        // Try wlr-randr (wlroots-based but sometimes available)
        if let Ok(out) = self.sh("wlr-randr", &[]).await {
            let mut current_name = String::new();
            let mut current_width = 1920u32;
            let mut current_height = 1080u32;
            let mut current_scale = 1.0f64;
            let mut idx = 0u32;

            for line in out.lines() {
                if !line.starts_with(' ') && !line.is_empty() {
                    // Header line, save previous
                    if !current_name.is_empty() {
                        monitors.push(protocol::MonitorInfo {
                            id: idx,
                            name: current_name.clone(),
                            width: current_width,
                            height: current_height,
                            scale: current_scale,
                            primary: idx == 0,
                        });
                        idx += 1;
                    }
                    current_name = line.split(' ').next().unwrap_or("").to_string();
                }
                if line.contains("current") {
                    // "   1920x1080 @ 60Hz"
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if let Some(res) = parts.first() {
                        let dims: Vec<&str> = res.split('x').collect();
                        if dims.len() == 2 {
                            current_width = dims[0].parse().unwrap_or(1920);
                            current_height = dims[1]
                                .split('@')
                                .next()
                                .unwrap_or("1080")
                                .parse()
                                .unwrap_or(1080);
                        }
                    }
                }
                if line.contains("Scale:") {
                    current_scale = line
                        .split("Scale:")
                        .nth(1)
                        .unwrap_or("1.0")
                        .trim()
                        .parse()
                        .unwrap_or(1.0);
                }
            }
            if !current_name.is_empty() {
                monitors.push(protocol::MonitorInfo {
                    id: idx,
                    name: current_name,
                    width: current_width,
                    height: current_height,
                    scale: current_scale,
                    primary: idx == 0,
                });
            }
            if !monitors.is_empty() {
                return Ok(monitors);
            }
        }

        // Absolute fallback: single 1920x1080 monitor
        monitors.push(protocol::MonitorInfo {
            id: 0,
            name: "Unknown".into(),
            width: 1920,
            height: 1080,
            scale: 1.0,
            primary: true,
        });
        Ok(monitors)
    }

    async fn get_workspace_count(&self) -> anyhow::Result<u32> {
        if let Ok(raw) = self.ext_call_parsed("WorkspacesList", &[]).await {
            // Count the tuples in the array
            let count = raw.matches("('").count() as u32;
            if count > 0 {
                return Ok(count);
            }
        }
        Ok(1)
    }

    async fn get_current_workspace(&self) -> anyhow::Result<u32> {
        if let Ok(raw) = self.ext_call_parsed("ActiveWorkspace", &[]).await {
            // Returns something like "(uint32 0,)"
            if let Some(start) = raw.find("uint32 ") {
                let num_str = &raw[start + 7..];
                if let Some(end) = num_str.find(|c: char| !c.is_ascii_digit()) {
                    return Ok(num_str[..end].parse().unwrap_or(0));
                }
            }
        }
        Ok(0)
    }

    async fn get_upower_property<T: serde::de::DeserializeOwned + zbus::zvariant::Type>(
        &self,
        path: &str,
        prop: &str,
    ) -> anyhow::Result<T> {
        let reply = self
            .conn
            .call_method(
                Some("org.freedesktop.UPower"),
                path,
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &("org.freedesktop.UPower.Device", prop),
            )
            .await?;
        let val: T = reply.body().deserialize()?;
        Ok(val)
    }
}

// ─── gdbus output parsers ────────────────────────────────

/// Parse gdbus WindowsList output: [('Firefox', '0x1a0000b', 'firefox', 0, false, true, ...), ...]
fn parse_gdbus_window_list(raw: &str) -> anyhow::Result<Vec<protocol::WindowInfo>> {
    let mut windows = Vec::new();
    // Strip outer brackets and split on "), ("
    let inner = raw.trim().trim_start_matches('[').trim_end_matches(']');
    if inner.is_empty() || inner == "()" {
        return Ok(windows);
    }

    for tuple_str in inner.split("), (") {
        let clean = tuple_str
            .trim()
            .trim_start_matches('(')
            .trim_end_matches(')');
        let parts: Vec<&str> = split_tuple(clean);
        if parts.len() < 5 {
            continue;
        }
        // Expected: title, id, app_id, workspace_id, is_focused, is_minimized, x, y, w, h
        let title = unquote(parts[0]);
        let id = unquote(parts[1]);
        let app_id = unquote(parts[2]);
        let workspace_id: u32 = parts[3].trim().parse().unwrap_or(0);
        let is_focused = parts[4].trim().to_lowercase() == "true";
        let is_minimized = if parts.len() > 5 {
            parts[5].trim().to_lowercase() == "true"
        } else {
            false
        };

        let geometry = if parts.len() >= 10 {
            let x: i32 = parts[6].trim().parse().unwrap_or(0);
            let y: i32 = parts[7].trim().parse().unwrap_or(0);
            let w: u32 = parts[8].trim().parse().unwrap_or(0);
            let h: u32 = parts[9].trim().parse().unwrap_or(0);
            Some(Geometry {
                x,
                y,
                width: w,
                height: h,
            })
        } else {
            None
        };

        windows.push(protocol::WindowInfo {
            id,
            title,
            app_id,
            workspace_id,
            is_focused,
            is_minimized,
            geometry,
            pid: None,
        });
    }
    Ok(windows)
}

fn parse_gdbus_single_window(raw: &str) -> anyhow::Result<protocol::WindowInfo> {
    let inner = raw.trim().trim_start_matches('(').trim_end_matches(')');
    let parts: Vec<&str> = split_tuple(inner);
    if parts.len() < 5 {
        anyhow::bail!("not enough fields in window tuple: {}", raw);
    }
    let title = unquote(parts[0]);
    let id = unquote(parts[1]);
    let app_id = unquote(parts[2]);
    let workspace_id: u32 = parts[3].trim().parse().unwrap_or(0);
    let is_focused = parts[4].trim().to_lowercase() == "true";
    let is_minimized = if parts.len() > 5 {
        parts[5].trim().to_lowercase() == "true"
    } else {
        false
    };

    let geometry = if parts.len() >= 10 {
        let x: i32 = parts[6].trim().parse().unwrap_or(0);
        let y: i32 = parts[7].trim().parse().unwrap_or(0);
        let w: u32 = parts[8].trim().parse().unwrap_or(0);
        let h: u32 = parts[9].trim().parse().unwrap_or(0);
        Some(Geometry {
            x,
            y,
            width: w,
            height: h,
        })
    } else {
        None
    };

    Ok(protocol::WindowInfo {
        id,
        title,
        app_id,
        workspace_id,
        is_focused,
        is_minimized,
        geometry,
        pid: None,
    })
}

fn parse_gdbus_workspace_list(raw: &str) -> anyhow::Result<Vec<protocol::WorkspaceInfo>> {
    let mut workspaces = Vec::new();
    let inner = raw.trim().trim_start_matches('[').trim_end_matches(']');
    if inner.is_empty() || inner == "()" {
        return Ok(workspaces);
    }

    let mut id = 0u32;
    for tuple_str in inner.split("), (") {
        let clean = tuple_str
            .trim()
            .trim_start_matches('(')
            .trim_end_matches(')');
        let parts: Vec<&str> = split_tuple(clean);
        if parts.is_empty() {
            continue;
        }
        let name = unquote(parts[0]);
        let is_active = if parts.len() > 1 {
            parts[1].trim().to_lowercase() == "true"
        } else {
            false
        };
        workspaces.push(protocol::WorkspaceInfo {
            id,
            name,
            is_active,
        });
        id += 1;
    }
    Ok(workspaces)
}

fn parse_gdbus_monitor_list(raw: &str) -> anyhow::Result<Vec<protocol::MonitorInfo>> {
    // Expect: [(0, 'DP-1', 2560, 1440, 1.0, true), ...]
    let mut monitors = Vec::new();
    let inner = raw.trim().trim_start_matches('[').trim_end_matches(']');
    if inner.is_empty() || inner == "()" {
        return Ok(monitors);
    }

    for tuple_str in inner.split("), (") {
        let clean = tuple_str
            .trim()
            .trim_start_matches('(')
            .trim_end_matches(')');
        let parts: Vec<&str> = split_tuple(clean);
        if parts.len() < 5 {
            continue;
        }
        let id: u32 = parts[0].trim().parse().unwrap_or(0);
        let name = unquote(parts[1]);
        let width: u32 = parts[2].trim().parse().unwrap_or(1920);
        let height: u32 = parts[3].trim().parse().unwrap_or(1080);
        let scale: f64 = parts[4].trim().parse().unwrap_or(1.0);
        let primary = parts.len() > 5 && parts[5].trim().to_lowercase() == "true";

        monitors.push(protocol::MonitorInfo {
            id,
            name,
            width,
            height,
            scale,
            primary,
        });
    }
    Ok(monitors)
}

fn parse_pactl_sinks(raw: &str) -> anyhow::Result<Vec<protocol::AudioSinkInfo>> {
    let mut sinks = Vec::new();
    let mut current_id = 0u32;
    let mut current_name = String::new();
    let mut current_desc = String::new();
    let mut current_volume = 0.0f64;
    let mut current_muted = false;
    let mut in_sink = false;

    for line in raw.lines() {
        if line.starts_with("Sink #") {
            if in_sink {
                sinks.push(protocol::AudioSinkInfo {
                    id: current_id,
                    name: current_name.clone(),
                    description: current_desc.clone(),
                    volume: current_volume,
                    muted: current_muted,
                });
            }
            in_sink = true;
            current_name.clear();
            current_desc.clear();
            current_volume = 0.0;
            current_muted = false;
            // "Sink #0"
            current_id = line
                .strip_prefix("Sink #")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        } else if in_sink {
            let trimmed = line.trim();
            if trimmed.starts_with("Name: ") {
                current_name = trimmed.strip_prefix("Name: ").unwrap_or("").to_string();
            } else if trimmed.starts_with("Description: ") {
                current_desc = trimmed
                    .strip_prefix("Description: ")
                    .unwrap_or("")
                    .to_string();
            } else if trimmed.starts_with("Mute: ") {
                current_muted = trimmed.contains("yes");
            } else if trimmed.starts_with("Volume:") {
                // "Volume: front-left: 32768 /  50% / -18.06 dB, ..."
                if let Some(pct) = trimmed.split('/').nth(1) {
                    current_volume = pct
                        .trim()
                        .trim_end_matches('%')
                        .parse::<f64>()
                        .unwrap_or(0.0)
                        / 100.0;
                }
            }
        }
    }
    if in_sink {
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

/// Get PNG dimensions from file header (minimal parser: width at byte 16, height at byte 20)
fn get_png_dimensions(path: &str) -> anyhow::Result<(u32, u32)> {
    let data = std::fs::read(path)?;
    if data.len() < 24 {
        anyhow::bail!("PNG file too small");
    }
    let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    Ok((width, height))
}

// ─── Utility parsers ────────────────────────────────────

/// Split a gdbus tuple string like "true, 'hello', 42" into parts, respecting quotes.
fn split_tuple(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut in_quote = false;
    let mut start = 0usize;

    for (i, ch) in s.char_indices() {
        match ch {
            '\'' => in_quote = !in_quote,
            '(' if !in_quote => depth += 1,
            ')' if !in_quote => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            ',' if !in_quote && depth == 0 => {
                parts.push(s[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    // last part
    if start < s.len() {
        parts.push(s[start..].trim());
    }
    parts
}

/// Strip single quotes and leading/trailing whitespace from a gdbus string value.
fn unquote(s: &str) -> String {
    let s = s.trim();
    // gdbus sometimes returns 'string' and sometimes just string
    if s.starts_with('\'') && s.ends_with('\'') {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}
