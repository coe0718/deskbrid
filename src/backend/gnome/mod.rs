use crate::protocol;
use crate::protocol::{DeskbridEvent, Geometry};
use async_trait::async_trait;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::process::Command;
use tokio::sync::broadcast;
use zbus::zvariant;

pub(crate) mod audio;
pub(crate) mod bluetooth;
pub(crate) mod clipboard;
pub(crate) mod files;
pub(crate) mod input;
pub(crate) mod keysym;
pub(crate) mod monitor;
pub(crate) mod network;
pub(crate) mod notifications;
pub(crate) mod screenshot;
pub(crate) mod system;
pub(crate) mod windows;
pub(crate) mod workspace;

// ─── Backend struct ────────────────────────────────────

pub struct GnomeBackend {
    /// DBus session connection for standard freedesktop interfaces.
    pub(super) conn: zbus::Connection,
    /// Broadcast sender for push events to subscribed clients.
    pub(super) event_tx: broadcast::Sender<DeskbridEvent>,
    /// Active file watchers keyed by path.
    pub(super) watchers: Arc<Mutex<HashMap<String, notify::RecommendedWatcher>>>,
    /// Mutter RemoteDesktop session path for input injection via compositor.
    pub(super) rd_session_path: String,
    /// Mutter ScreenCast stream path for absolute mouse positioning.
    pub(super) sc_stream_path: String,
    /// Last known mouse position for relative delta calculation.
    pub(super) last_mouse: std::sync::Mutex<(f64, f64)>,
}

impl GnomeBackend {
    pub async fn new(event_tx: broadcast::Sender<DeskbridEvent>) -> anyhow::Result<Self> {
        let conn = zbus::Connection::session().await?;
        let mut backend = Self {
            conn,
            event_tx,
            watchers: Arc::new(Mutex::new(HashMap::new())),
            rd_session_path: String::new(),
            sc_stream_path: String::new(),
            last_mouse: std::sync::Mutex::new((960.0, 540.0)),
        };
        backend.init_remote_desktop().await?;
        // ScreenCast is best-effort — required for absolute mouse positioning.
        // Relative motion works without it.
        if let Err(e) = backend.init_screen_cast().await {
            tracing::warn!(
                "ScreenCast unavailable (absolute mouse positioning disabled): {}",
                e
            );
        }
        Ok(backend)
    }

    // ─── Shell helpers ──────────────────────────────────

    /// Run a command, return stdout as String. Fails on non-zero exit.
    pub(super) async fn sh(&self, cmd: &str, args: &[&str]) -> anyhow::Result<String> {
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
    pub(super) async fn sh_ok(&self, cmd: &str, args: &[&str]) -> bool {
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

    pub(super) async fn sh_owned(&self, cmd: &str, args: Vec<String>) -> anyhow::Result<String> {
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        self.sh(cmd, &refs).await
    }

    // ─── Extension DBus helpers ─────────────────────────

    /// Path to the GNOME Shell extension's DBus object.
    const EXT_BUS: &'static str = "org.deskbrid.WindowManager";
    const EXT_PATH: &'static str = "/org/deskbrid/WindowManager";
    const EXT_IFACE: &'static str = "org.deskbrid.WindowManager";

    /// Call an extension DBus method via gdbus. Returns raw string.
    pub(super) async fn ext_call_parsed(
        &self,
        method: &str,
        extra_args: &[&str],
    ) -> anyhow::Result<String> {
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

    pub(super) async fn ext_call_bool(
        &self,
        method: &str,
        extra_args: &[&str],
    ) -> anyhow::Result<()> {
        let raw = self.ext_call_parsed(method, extra_args).await?;
        if raw.contains("true") {
            Ok(())
        } else {
            anyhow::bail!("GNOME extension method {} returned false", method)
        }
    }

    pub(super) async fn resolve_window(&self, id: &str) -> anyhow::Result<protocol::WindowInfo> {
        if id.trim().is_empty() {
            anyhow::bail!("window id must not be empty");
        }

        let raw = self.ext_call_parsed("ListWindows", &[]).await?;
        let windows = parse_extension_json_windows(&raw)?;
        let id_l = id.to_lowercase();

        windows
            .iter()
            .find(|w| w.id.eq_ignore_ascii_case(id))
            .cloned()
            .or_else(|| {
                windows
                    .iter()
                    .find(|w| w.app_id.eq_ignore_ascii_case(id))
                    .cloned()
            })
            .or_else(|| {
                windows
                    .iter()
                    .find(|w| w.title.eq_ignore_ascii_case(id))
                    .cloned()
            })
            .or_else(|| {
                windows
                    .iter()
                    .find(|w| {
                        w.app_id.to_lowercase().contains(&id_l)
                            || w.title.to_lowercase().contains(&id_l)
                    })
                    .cloned()
            })
            .ok_or_else(|| anyhow::anyhow!("window not found: {}", id))
    }

    // ─── Remote Desktop input injection ─────────────────

    async fn init_remote_desktop(&mut self) -> anyhow::Result<()> {
        let reply = self
            .conn
            .call_method(
                Some("org.gnome.Mutter.RemoteDesktop"),
                "/org/gnome/Mutter/RemoteDesktop",
                Some("org.gnome.Mutter.RemoteDesktop"),
                "CreateSession",
                &(),
            )
            .await?;
        let path: zbus::zvariant::OwnedObjectPath = reply.body().deserialize()?;

        self.conn
            .call_method(
                Some("org.gnome.Mutter.RemoteDesktop"),
                path.as_str(),
                Some("org.gnome.Mutter.RemoteDesktop.Session"),
                "Start",
                &(),
            )
            .await?;

        self.rd_session_path = path.to_string();
        tracing::info!("RemoteDesktop session started: {}", self.rd_session_path);
        Ok(())
    }

    async fn init_screen_cast(&mut self) -> anyhow::Result<()> {
        use std::collections::HashMap;

        let props: HashMap<&str, zbus::zvariant::Value<'_>> = HashMap::new();
        let reply = self
            .conn
            .call_method(
                Some("org.gnome.Mutter.ScreenCast"),
                "/org/gnome/Mutter/ScreenCast",
                Some("org.gnome.Mutter.ScreenCast"),
                "CreateSession",
                &(props,),
            )
            .await?;
        let session_path: zbus::zvariant::OwnedObjectPath = reply.body().deserialize()?;

        self.conn
            .call_method(
                Some("org.gnome.Mutter.ScreenCast"),
                session_path.as_str(),
                Some("org.gnome.Mutter.ScreenCast.Session"),
                "Start",
                &(),
            )
            .await?;

        let mut monitor_candidates = Vec::new();
        if let Ok(monitors) = self.get_monitors().await {
            if let Some(primary) = monitors
                .iter()
                .find(|m| m.primary)
                .or_else(|| monitors.first())
            {
                monitor_candidates.push(primary.name.clone());
            }
            for m in monitors {
                if !monitor_candidates.iter().any(|n| n == &m.name) {
                    monitor_candidates.push(m.name);
                }
            }
        }
        if !monitor_candidates.iter().any(|n| n == "DP-1") {
            monitor_candidates.push("DP-1".to_string());
        }

        let stream_props: HashMap<&str, zbus::zvariant::Value<'_>> = HashMap::new();
        let mut last_err: Option<anyhow::Error> = None;
        for connector in monitor_candidates {
            tracing::info!("Trying ScreenCast monitor: {}", connector);
            match self
                .conn
                .call_method(
                    Some("org.gnome.Mutter.ScreenCast"),
                    session_path.as_str(),
                    Some("org.gnome.Mutter.ScreenCast.Session"),
                    "RecordMonitor",
                    &(connector.as_str(), stream_props.clone()),
                )
                .await
            {
                Ok(reply) => {
                    let stream_path: zbus::zvariant::OwnedObjectPath =
                        reply.body().deserialize()?;
                    self.sc_stream_path = stream_path.to_string();
                    tracing::info!("ScreenCast stream created: {}", self.sc_stream_path);
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!("RecordMonitor failed for {}: {}", connector, e);
                    last_err = Some(e.into());
                }
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("failed to record any monitor")))
    }

    /// Call a void method on the RemoteDesktop session.
    pub(super) async fn rd_call<B>(&self, method: &str, body: &B) -> anyhow::Result<()>
    where
        B: serde::Serialize + zbus::zvariant::Type,
    {
        self.conn
            .call_method(
                Some("org.gnome.Mutter.RemoteDesktop"),
                self.rd_session_path.as_str(),
                Some("org.gnome.Mutter.RemoteDesktop.Session"),
                method,
                body,
            )
            .await?;
        Ok(())
    }

    /// Press or release a keysym through the Mutter compositor pipeline.
    pub(super) async fn rd_keysym(&self, keysym: u32, pressed: bool) -> anyhow::Result<()> {
        self.rd_call("NotifyKeyboardKeysym", &(keysym, pressed))
            .await
    }

    /// Press or release a mouse button.
    pub(super) async fn rd_button(&self, button: i32, pressed: bool) -> anyhow::Result<()> {
        self.rd_call("NotifyPointerButton", &(button, pressed))
            .await
    }
}

// ─── Trait implementation ──────────────────────────────

#[async_trait]
impl crate::backend::DesktopBackend for GnomeBackend {
    async fn windows_list(&self) -> anyhow::Result<Vec<protocol::WindowInfo>> {
        self.windows_list_inner().await
    }

    async fn window_focus(&self, id: &str) -> anyhow::Result<()> {
        self.window_focus_inner(id).await
    }

    async fn window_get(&self, id: &str) -> anyhow::Result<protocol::WindowInfo> {
        self.resolve_window(id).await
    }

    async fn window_close(&self, id: &str) -> anyhow::Result<()> {
        self.window_close_inner(id).await
    }

    async fn window_minimize(&self, id: &str) -> anyhow::Result<()> {
        self.window_minimize_inner(id).await
    }

    async fn window_maximize(&self, id: &str) -> anyhow::Result<()> {
        self.window_maximize_inner(id).await
    }

    async fn window_move_resize(
        &self,
        id: &str,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> anyhow::Result<()> {
        self.window_move_resize_inner(id, x, y, width, height).await
    }

    async fn workspaces_list(&self) -> anyhow::Result<Vec<protocol::WorkspaceInfo>> {
        self.workspaces_list_inner().await
    }

    async fn workspace_switch(&self, id: u32) -> anyhow::Result<()> {
        self.workspace_switch_inner(id).await
    }

    async fn workspace_move_window(
        &self,
        window_id: &str,
        workspace_id: u32,
        follow: bool,
    ) -> anyhow::Result<()> {
        self.workspace_move_window_inner(window_id, workspace_id)
            .await?;
        if follow {
            self.workspace_switch_inner(workspace_id).await?;
        }
        Ok(())
    }

    async fn keyboard_type(&self, text: &str) -> anyhow::Result<()> {
        self.keyboard_type_inner(text).await
    }

    async fn keyboard_key(&self, key: &str) -> anyhow::Result<()> {
        self.keyboard_key_inner(key).await
    }

    async fn keyboard_combo(&self, keys: &[String]) -> anyhow::Result<()> {
        self.keyboard_combo_inner(keys).await
    }

    async fn mouse_move(&self, x: f64, y: f64) -> anyhow::Result<()> {
        self.mouse_move_inner(x, y).await
    }

    async fn mouse_click(&self, button: &str) -> anyhow::Result<()> {
        self.mouse_click_inner(button).await
    }

    async fn mouse_scroll(&self, dx: f64, dy: f64) -> anyhow::Result<()> {
        self.mouse_scroll_inner(dx, dy).await
    }

    async fn clipboard_read(&self) -> anyhow::Result<String> {
        self.clipboard_read_inner().await
    }

    async fn clipboard_write(&self, text: &str) -> anyhow::Result<()> {
        self.clipboard_write_inner(text).await
    }

    async fn screenshot(
        &self,
        monitor: Option<u32>,
        region: Option<protocol::Region>,
        window_id: Option<String>,
    ) -> anyhow::Result<protocol::ScreenshotResult> {
        self.screenshot_inner(monitor, region, window_id).await
    }

    async fn notification_send(
        &self,
        app_name: &str,
        title: &str,
        body: &str,
        urgency: &str,
    ) -> anyhow::Result<u32> {
        self.notification_send_inner(app_name, title, body, urgency)
            .await
    }

    async fn notification_close(&self, id: u32) -> anyhow::Result<()> {
        self.notification_close_inner(id).await
    }

    async fn system_info(&self) -> anyhow::Result<protocol::SystemInfo> {
        self.system_info_inner().await
    }

    async fn idle_seconds(&self) -> anyhow::Result<u64> {
        self.idle_seconds_inner().await
    }

    async fn power_action(&self, action: &str) -> anyhow::Result<()> {
        self.power_action_inner(action).await
    }

    async fn battery_status(&self) -> anyhow::Result<Vec<protocol::BatteryInfo>> {
        self.battery_status_inner().await
    }

    async fn network_status(&self) -> anyhow::Result<protocol::NetworkStatusInfo> {
        self.network_status_inner().await
    }

    async fn network_interfaces(&self) -> anyhow::Result<Vec<protocol::NetworkInterfaceInfo>> {
        self.network_interfaces_inner().await
    }

    async fn wifi_scan(&self) -> anyhow::Result<Vec<protocol::WifiNetworkInfo>> {
        self.wifi_scan_inner().await
    }

    async fn wifi_connect(&self, ssid: &str, password: Option<&str>) -> anyhow::Result<()> {
        self.wifi_connect_inner(ssid, password).await
    }

    async fn bluetooth_list(&self) -> anyhow::Result<Vec<protocol::BluetoothDeviceInfo>> {
        self.bluetooth_list_inner().await
    }

    async fn bluetooth_scan(&self, duration: Option<u32>) -> anyhow::Result<()> {
        self.bluetooth_scan_inner(duration).await
    }

    async fn bluetooth_stop_scan(&self) -> anyhow::Result<()> {
        self.bluetooth_stop_scan_inner().await
    }

    async fn bluetooth_connect(&self, address: &str) -> anyhow::Result<()> {
        self.bluetooth_connect_inner(address).await
    }

    async fn bluetooth_disconnect(&self, address: &str) -> anyhow::Result<()> {
        self.bluetooth_disconnect_inner(address).await
    }

    async fn files_watch(
        &self,
        path: &str,
        recursive: bool,
        patterns: Option<&[String]>,
    ) -> anyhow::Result<()> {
        self.files_watch_inner(path, recursive, patterns).await
    }

    async fn files_unwatch(&self, path: &str) -> anyhow::Result<()> {
        self.files_unwatch_inner(path).await
    }

    async fn files_search(
        &self,
        pattern: &str,
        root: Option<&str>,
        max_results: u32,
    ) -> anyhow::Result<Vec<String>> {
        self.files_search_inner(pattern, root, max_results).await
    }

    async fn audio_list_sinks(&self) -> anyhow::Result<Vec<protocol::AudioSinkInfo>> {
        self.audio_list_sinks_inner().await
    }

    async fn audio_set_sink_volume(&self, sink_id: u32, volume: f64) -> anyhow::Result<()> {
        self.audio_set_sink_volume_inner(sink_id, volume).await
    }

    async fn monitor_set_primary(&self, output: &str) -> anyhow::Result<()> {
        self.monitor_set_primary_inner(output).await
    }

    async fn monitor_set_resolution(
        &self,
        output: &str,
        width: u32,
        height: u32,
        refresh_rate: Option<f64>,
    ) -> anyhow::Result<()> {
        self.monitor_set_resolution_inner(output, width, height, refresh_rate)
            .await
    }

    async fn monitor_set_scale(&self, output: &str, scale: f64) -> anyhow::Result<()> {
        self.monitor_set_scale_inner(output, scale).await
    }

    async fn monitor_set_rotation(&self, output: &str, rotation: &str) -> anyhow::Result<()> {
        self.monitor_set_rotation_inner(output, rotation).await
    }

    async fn monitor_set_enabled(&self, output: &str, enabled: bool) -> anyhow::Result<()> {
        self.monitor_set_enabled_inner(output, enabled).await
    }
}

// ─── Private helpers shared across submodules ──────────

impl GnomeBackend {
    pub(super) async fn idle_seconds_inner(&self) -> anyhow::Result<u64> {
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

    pub(super) async fn get_monitors(&self) -> anyhow::Result<Vec<protocol::MonitorInfo>> {
        let mut monitors = Vec::new();
        if let Ok(out) = self.sh("gnome-randr", &[]).await {
            parse_gnome_randr(&out, &mut monitors);
            if !monitors.is_empty() {
                return Ok(monitors);
            }
        }
        if let Ok(out) = self.sh("wlr-randr", &[]).await {
            parse_wlr_randr(&out, &mut monitors);
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
            enabled: true,
            x: 0,
            y: 0,
            refresh_rate: None,
            rotation: "normal".into(),
        });
        Ok(monitors)
    }

    pub(super) async fn get_workspace_count(&self) -> anyhow::Result<u32> {
        if let Ok(raw) = self.ext_call_parsed("WorkspacesList", &[]).await {
            let count = raw.matches("('").count() as u32;
            if count > 0 {
                return Ok(count);
            }
        }
        Ok(1)
    }

    pub(super) async fn get_current_workspace(&self) -> anyhow::Result<u32> {
        if let Ok(raw) = self.ext_call_parsed("ActiveWorkspace", &[]).await {
            if let Some(start) = raw.find("uint32 ") {
                let num_str = &raw[start + 7..];
                if let Some(end) = num_str.find(|c: char| !c.is_ascii_digit()) {
                    return Ok(num_str[..end].parse().unwrap_or(0));
                }
            }
        }
        Ok(0)
    }

    pub(super) async fn get_upower_property<
        T: serde::de::DeserializeOwned + zbus::zvariant::Type,
    >(
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

    pub(super) async fn get_nm_property<T: serde::de::DeserializeOwned + zbus::zvariant::Type>(
        &self,
        path: &str,
        prop: &str,
    ) -> anyhow::Result<T> {
        let reply = self
            .conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                path,
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &("org.freedesktop.NetworkManager.Device", prop),
            )
            .await?;
        let val: T = reply.body().deserialize()?;
        Ok(val)
    }

    pub(super) async fn get_nm_ip4_address(&self, config_path: &str) -> Option<String> {
        let reply = self
            .conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                config_path,
                Some("org.freedesktop.DBus.Properties"),
                "GetAll",
                &("org.freedesktop.NetworkManager.IP4Config",),
            )
            .await
            .ok()?;
        let props: std::collections::HashMap<String, zvariant::OwnedValue> =
            reply.body().deserialize().ok()?;

        let addresses = props.get("AddressData")?;
        let arr = addresses.downcast_ref::<zvariant::Array>().ok()?;
        for entry in arr.iter() {
            if let Ok(inner) = entry.downcast_ref::<zvariant::Structure>() {
                let fields = inner.fields();
                if let Some(v) = fields.first() {
                    if let Ok(s) = v.downcast_ref::<zvariant::Str>() {
                        return Some(s.to_string());
                    }
                }
            }
        }
        None
    }

    pub(super) async fn find_bluetooth_adapter(&self) -> anyhow::Result<String> {
        let reply = self
            .conn
            .call_method(
                Some("org.bluez"),
                "/",
                Some("org.freedesktop.DBus.ObjectManager"),
                "GetManagedObjects",
                &(),
            )
            .await?;

        let managed: std::collections::HashMap<
            zvariant::OwnedObjectPath,
            std::collections::HashMap<String, zvariant::OwnedValue>,
        > = reply.body().deserialize()?;

        for (path, ifaces) in &managed {
            if ifaces.contains_key("org.bluez.Adapter1") {
                return Ok(path.as_str().to_string());
            }
        }
        anyhow::bail!("no Bluetooth adapter found")
    }

    pub(super) fn device_path(&self, address: &str) -> String {
        let normalized = address.replace(':', "_").to_uppercase();
        format!("/org/bluez/hci0/dev_{}", normalized)
    }
}

// ─── Free helper functions ─────────────────────────────

/// Parse the JSON string returned by the extension's ListWindows() method.
pub(super) fn parse_extension_json_windows(raw: &str) -> anyhow::Result<Vec<protocol::WindowInfo>> {
    let inner = raw.trim().trim_start_matches('(').trim_end_matches(')');
    let json_str = inner
        .trim()
        .trim_start_matches('\'')
        .trim_end_matches(',')
        .trim()
        .trim_end_matches('\'');
    let parsed: Vec<serde_json::Value> = serde_json::from_str(json_str)?;

    let windows: Vec<protocol::WindowInfo> = parsed
        .into_iter()
        .map(|w| protocol::WindowInfo {
            id: w["id"]
                .as_u64()
                .map(|n| n.to_string())
                .unwrap_or_else(|| w["id"].as_str().unwrap_or("").to_string()),
            title: w["title"].as_str().unwrap_or("").to_string(),
            app_id: w["app_id"].as_str().unwrap_or("").to_string(),
            workspace_id: w["workspace_index"].as_u64().unwrap_or(0) as u32,
            is_focused: w["focused"].as_bool().unwrap_or(false),
            is_minimized: w["minimized"].as_bool().unwrap_or(false),
            geometry: {
                let geo = &w["geometry"];
                if let Some(arr) = geo.as_array() {
                    let x = arr.first().and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                    let y = arr.get(1).and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                    let width = arr.get(2).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    let height = arr.get(3).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    Some(Geometry {
                        x,
                        y,
                        width,
                        height,
                    })
                } else {
                    None
                }
            },
            pid: w["pid"].as_u64().map(|p| p as u32),
        })
        .collect();
    Ok(windows)
}

fn parse_gnome_randr(out: &str, monitors: &mut Vec<protocol::MonitorInfo>) {
    let mut current_name = String::new();
    let mut current_width = 1920u32;
    let mut current_height = 1080u32;
    let mut current_scale = 1.0f64;
    let mut idx = 0u32;
    for line in out.lines() {
        if line.starts_with("  ") || line.trim().is_empty() {
            if line.contains("x") && line.contains('@') {
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
            if line.to_lowercase().contains("scale") {
                current_scale = line
                    .split(':')
                    .nth(1)
                    .unwrap_or("1.0")
                    .trim()
                    .parse()
                    .unwrap_or(1.0);
            }
            continue;
        }
        if !current_name.is_empty() {
            monitors.push(protocol::MonitorInfo {
                id: idx,
                name: current_name.clone(),
                width: current_width,
                height: current_height,
                scale: current_scale,
                primary: idx == 0,
                enabled: true,
                x: 0,
                y: 0,
                refresh_rate: None,
                rotation: "normal".into(),
            });
            idx += 1;
        }
        current_name = line.split_whitespace().next().unwrap_or("").to_string();
    }
    if !current_name.is_empty() {
        monitors.push(protocol::MonitorInfo {
            id: idx,
            name: current_name,
            width: current_width,
            height: current_height,
            scale: current_scale,
            primary: idx == 0,
            enabled: true,
            x: 0,
            y: 0,
            refresh_rate: None,
            rotation: "normal".into(),
        });
    }
}

fn parse_wlr_randr(out: &str, monitors: &mut Vec<protocol::MonitorInfo>) {
    let mut current_name = String::new();
    let mut current_width = 1920u32;
    let mut current_height = 1080u32;
    let mut current_scale = 1.0f64;
    let mut idx = 0u32;

    for line in out.lines() {
        if !line.starts_with(' ') && !line.is_empty() {
            if !current_name.is_empty() {
                monitors.push(protocol::MonitorInfo {
                    id: idx,
                    name: current_name.clone(),
                    width: current_width,
                    height: current_height,
                    scale: current_scale,
                    primary: idx == 0,
                    enabled: true,
                    x: 0,
                    y: 0,
                    refresh_rate: None,
                    rotation: "normal".into(),
                });
                idx += 1;
            }
            current_name = line.split(' ').next().unwrap_or("").to_string();
        }
        if line.contains("current") {
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
            enabled: true,
            x: 0,
            y: 0,
            refresh_rate: None,
            rotation: "normal".into(),
        });
    }
}
