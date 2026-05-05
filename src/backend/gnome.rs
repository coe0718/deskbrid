use crate::backend::audio::spawn_audio_monitor;
use crate::backend::screencast::{self, ScreencastSession};
use crate::backend::types::{MonitorInfo, WindowInfo};
use crate::backend::{DesktopBackend, InputBackend};
use crate::events::EventBus;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{watch, Mutex};
use tokio::time::{self, Duration};
use tracing::{debug, info, warn};
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

const SHELL_DEST: &str = "org.gnome.Shell";
const SHELL_PATH: &str = "/org/gnome/Shell";
const SHELL_IFACE: &str = "org.gnome.Shell";
const DISPLAY_CONFIG_DEST: &str = "org.gnome.Mutter.DisplayConfig";
const DISPLAY_CONFIG_PATH: &str = "/org/gnome/Mutter/DisplayConfig";
const DISPLAY_CONFIG_IFACE: &str = "org.gnome.Mutter.DisplayConfig";
const MUTTER_DEST: &str = "org.gnome.Mutter.RemoteDesktop";
const MUTTER_PATH: &str = "/org/gnome/Mutter/RemoteDesktop";
const MUTTER_IFACE: &str = "org.gnome.Mutter.RemoteDesktop";

// deskbrid GNOME Shell extension for window management
const WM_DEST: &str = "org.deskbrid.WindowManager";
const WM_PATH: &str = "/org/deskbrid/WindowManager";
const WM_IFACE: &str = "org.deskbrid.WindowManager";
const SESSION_IFACE: &str = "org.gnome.Mutter.RemoteDesktop.Session";
const DEVICE_TYPES_ALL: u32 = 7;
const KEY_RELEASED: bool = false;
const KEY_PRESSED: bool = true;
const BUTTON_RELEASED: bool = false;
const BUTTON_PRESSED: bool = true;

type Mode = (
    String,
    i32,
    i32,
    f64,
    f64,
    Vec<f64>,
    HashMap<String, OwnedValue>,
);
type PhysicalMonitor = (
    (String, String, String, String),
    Vec<Mode>,
    HashMap<String, OwnedValue>,
);
type LogicalMonitor = (
    i32,
    i32,
    f64,
    u32,
    bool,
    Vec<(String, String, String, String)>,
    HashMap<String, OwnedValue>,
);

#[derive(Clone)]
pub struct GnomeBackend {
    conn: zbus::Connection,
    screencasts: Arc<Mutex<HashMap<u32, ScreencastSession>>>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct FocusWindow {
    title: String,
    app_id: String,
    pid: i64,
    workspace: i64,
    geometry: [i64; 4],
    wm_class: String,
}

#[derive(Clone)]
pub struct GnomeInputSession {
    conn: zbus::Connection,
    path: OwnedObjectPath,
    lock: Arc<Mutex<()>>,
    api: InputApi,
    cursor_x: Arc<Mutex<f64>>,
    cursor_y: Arc<Mutex<f64>>,
}

impl GnomeBackend {
    pub async fn new(event_bus: EventBus, shutdown: watch::Receiver<bool>) -> Result<Self> {
        let conn = zbus::Connection::session()
            .await
            .context("connecting to session bus")?;
        let backend = Self {
            conn,
            screencasts: Arc::new(Mutex::new(HashMap::new())),
        };
        backend.spawn_watchers(event_bus.clone());
        spawn_audio_monitor(event_bus, shutdown);
        Ok(backend)
    }

    fn spawn_watchers(&self, event_bus: EventBus) {
        let windows_backend = self.clone();
        let windows_bus = event_bus.clone();
        tokio::spawn(async move {
            if let Err(error) = windows_backend.watch_windows(windows_bus).await {
                warn!("window watcher failed: {error:#}");
            }
        });

        let notifications_backend = self.clone();
        let notifications_bus = event_bus.clone();
        tokio::spawn(async move {
            if let Err(error) = notifications_backend
                .watch_notifications(notifications_bus)
                .await
            {
                warn!("notification watcher failed: {error:#}");
            }
        });

        let idle_backend = self.clone();
        tokio::spawn(async move {
            if let Err(error) = idle_backend.watch_idle(event_bus).await {
                warn!("idle watcher failed: {error:#}");
            }
        });
    }

    async fn watch_windows(self, event_bus: EventBus) -> Result<()> {
        let mut ticker = time::interval(Duration::from_millis(500));
        let mut last_focus: Option<FocusWindow> = None;
        let mut previous_windows: HashSet<WindowInfo> = HashSet::new();

        loop {
            ticker.tick().await;

            match self.poll_focus().await {
                Ok(Some(focus)) if last_focus.as_ref() != Some(&focus) => {
                    event_bus.emit(
                        "window:focus",
                        serde_json::json!({
                            "title": focus.title,
                            "app_id": focus.app_id,
                            "pid": focus.pid,
                            "workspace": focus.workspace,
                            "geometry": focus.geometry,
                            "wm_class": focus.wm_class,
                        }),
                    );
                    last_focus = Some(focus);
                }
                Ok(_) => {}
                Err(error) => debug!("window focus poll failed: {error:#}"),
            }

            match self.list_windows().await {
                Ok(current) => {
                    let current_set: HashSet<_> = current.iter().cloned().collect();

                    for opened in current_set.difference(&previous_windows) {
                        event_bus.emit(
                            "window:open",
                            serde_json::json!({
                                "title": opened.title,
                                "app_id": opened.app_id,
                                "pid": opened.pid,
                                "workspace": opened.workspace,
                                "geometry": opened.geometry,
                            }),
                        );
                    }

                    for closed in previous_windows.difference(&current_set) {
                        event_bus.emit(
                            "window:close",
                            serde_json::json!({
                                "app_id": closed.app_id,
                                "pid": closed.pid,
                            }),
                        );
                    }

                    previous_windows = current_set;
                }
                Err(error) => debug!("window list poll failed: {error:#}"),
            }
        }
    }

    async fn watch_notifications(self, event_bus: EventBus) -> Result<()> {
        let mut child = Command::new("dbus-monitor")
            .args([
                "--session",
                "interface='org.freedesktop.Notifications',member='Notify'",
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .context("spawning dbus-monitor for notifications")?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("dbus-monitor stdout unavailable"))?;
        let mut lines = BufReader::new(stdout).lines();
        let mut block: Vec<String> = Vec::new();

        loop {
            match lines
                .next_line()
                .await
                .context("reading dbus-monitor output")?
            {
                Some(line) => {
                    if line.trim().is_empty() {
                        if let Some(event) = parse_notification_block(&block) {
                            event_bus.emit("notifications", event);
                        }
                        block.clear();
                        continue;
                    }
                    block.push(line);
                }
                None => {
                    warn!("notification monitor exited");
                    break;
                }
            }
        }

        Ok(())
    }

    async fn watch_idle(self, _event_bus: EventBus) -> Result<()> {
        debug!("idle monitor not implemented in phase 1");
        std::future::pending::<()>().await;
        Ok(())
    }

    async fn poll_focus(&self) -> Result<Option<FocusWindow>> {
        let script = r#"
            (() => {
              const m = global.display.focus_window;
              if (!m) return "null";
              const rect = m.get_frame_rect();
              return JSON.stringify({
                title: m.get_title() || "",
                app_id: m.get_wm_class() || "",
                pid: m.get_pid() || 0,
                workspace: m.get_workspace() ? m.get_workspace().index() : 0,
                geometry: [rect.x, rect.y, rect.width, rect.height],
                wm_class: m.get_wm_class() || ""
              });
            })()
        "#;
        self.eval_json(script).await
    }

    async fn eval_json<T>(&self, script: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let proxy = zbus::Proxy::new(&self.conn, SHELL_DEST, SHELL_PATH, SHELL_IFACE)
            .await
            .context("creating gnome shell proxy")?;
        let (success, result): (bool, String) = proxy
            .call("Eval", &(script))
            .await
            .context("calling org.gnome.Shell.Eval")?;

        if !success {
            return Err(anyhow!("shell eval returned unsuccessful result"));
        }

        serde_json::from_str(&result).with_context(|| format!("parsing shell eval json: {result}"))
    }

    async fn display_connectors(&self) -> Result<Vec<String>> {
        let proxy = zbus::Proxy::new(
            &self.conn,
            DISPLAY_CONFIG_DEST,
            DISPLAY_CONFIG_PATH,
            DISPLAY_CONFIG_IFACE,
        )
        .await
        .context("creating mutter display config proxy")?;

        let (_serial, monitors, _logical_monitors, _properties): (
            u32,
            Vec<PhysicalMonitor>,
            Vec<LogicalMonitor>,
            HashMap<String, OwnedValue>,
        ) = proxy
            .call("GetCurrentState", &())
            .await
            .context("calling org.gnome.Mutter.DisplayConfig.GetCurrentState")?;

        Ok(monitors
            .into_iter()
            .map(|((connector, _, _, _), _, _)| connector)
            .collect())
    }

    async fn monitor_connector(&self, monitor: u32) -> Result<String> {
        let connectors = self.display_connectors().await?;
        connectors
            .get(monitor as usize)
            .cloned()
            .ok_or_else(|| anyhow!("unknown monitor id: {monitor}"))
    }
}

#[async_trait]
impl DesktopBackend for GnomeBackend {
    async fn list_windows(&self) -> Result<Vec<WindowInfo>> {
        let proxy = zbus::Proxy::new(&self.conn, WM_DEST, WM_PATH, WM_IFACE)
            .await
            .context("creating deskbrid window manager proxy")?;
        let result: String = proxy
            .call("ListWindows", &())
            .await
            .context("calling ListWindows on deskbrid extension")?;
        serde_json::from_str(&result).with_context(|| format!("parsing window list json: {result}"))
    }

    async fn focus_window(
        &self,
        app_id: Option<&str>,
        title: Option<&str>,
        exact: bool,
    ) -> Result<()> {
        if app_id.is_none() && title.is_none() {
            return Err(anyhow!("window:focus requires app_id or title"));
        }

        let proxy = zbus::Proxy::new(&self.conn, WM_DEST, WM_PATH, WM_IFACE)
            .await
            .context("creating deskbrid window manager proxy")?;
        let success: bool = proxy
            .call(
                "FocusWindow",
                &(app_id.unwrap_or_default(), title.unwrap_or_default(), exact),
            )
            .await
            .context("calling FocusWindow on deskbrid extension")?;

        if success {
            Ok(())
        } else {
            Err(anyhow!("no matching window found"))
        }
    }

    async fn focused_window(&self) -> Result<Option<WindowInfo>> {
        let proxy = zbus::Proxy::new(&self.conn, WM_DEST, WM_PATH, WM_IFACE)
            .await
            .context("creating deskbrid window manager proxy")?;
        let result: String = proxy
            .call("FocusedWindow", &())
            .await
            .context("calling FocusedWindow on deskbrid extension")?;
        if result == "null" {
            Ok(None)
        } else {
            serde_json::from_str(&result)
                .map(Some)
                .with_context(|| format!("parsing focused window json: {result}"))
        }
    }

    async fn list_displays(&self) -> Result<Vec<MonitorInfo>> {
        let proxy = zbus::Proxy::new(
            &self.conn,
            DISPLAY_CONFIG_DEST,
            DISPLAY_CONFIG_PATH,
            DISPLAY_CONFIG_IFACE,
        )
        .await
        .context("creating mutter display config proxy")?;

        let (_serial, monitors, logical_monitors, _properties): (
            u32,
            Vec<PhysicalMonitor>,
            Vec<LogicalMonitor>,
            HashMap<String, OwnedValue>,
        ) = proxy
            .call("GetCurrentState", &())
            .await
            .context("calling org.gnome.Mutter.DisplayConfig.GetCurrentState")?;

        let mut scales_by_connector = HashMap::new();
        for (_, _, scale, _, _, connector_refs, _) in &logical_monitors {
            for (connector, _, _, _) in connector_refs {
                scales_by_connector.insert(connector.clone(), *scale);
            }
        }

        let mut result = Vec::with_capacity(monitors.len());
        for (index, ((connector, _, _, _), modes, _)) in monitors.iter().enumerate() {
            let current_mode = modes
                .iter()
                .find(|(_, _, _, _, _, _, properties)| property_bool(properties, "is-current"))
                .or_else(|| modes.first())
                .ok_or_else(|| anyhow!("monitor {connector} reported no modes"))?;

            result.push(MonitorInfo {
                id: index as u32,
                width: current_mode.1,
                height: current_mode.2,
                scale: scales_by_connector
                    .get(connector)
                    .copied()
                    .unwrap_or(current_mode.4),
                refresh: current_mode.3.round().max(0.0) as u32,
            });
        }

        Ok(result)
    }

    async fn create_input_session(&self) -> Result<Box<dyn InputBackend>> {
        Ok(Box::new(GnomeInputSession::new().await?))
    }

    async fn send_notification(&self, summary: &str, body: &str, urgency: &str) -> Result<u32> {
        let proxy = zbus::Proxy::new(
            &self.conn,
            "org.freedesktop.Notifications",
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
        )
        .await
        .context("creating notifications proxy")?;

        let urgency_byte = match urgency {
            "low" => 0_u8,
            "critical" => 2_u8,
            _ => 1_u8,
        };
        let hints = std::collections::HashMap::<&str, zbus::zvariant::Value<'_>>::from([(
            "urgency",
            zbus::zvariant::Value::from(urgency_byte),
        )]);
        let actions: Vec<&str> = Vec::new();

        proxy
            .call(
                "Notify",
                &("deskbrid", 0_u32, "", summary, body, actions, hints, -1_i32),
            )
            .await
            .context("sending notification")
    }

    async fn screenshot(&self, monitor: Option<u32>) -> Result<Value> {
        let connector = self.monitor_connector(monitor.unwrap_or(0)).await?;
        let result = screencast::screenshot(&self.conn, &connector).await?;
        Ok(serde_json::json!({
            "path": result.path,
            "width": result.width,
            "height": result.height,
        }))
    }

    async fn start_screencast(&self, monitor: u32, framerate: u32) -> Result<Value> {
        let connector = self.monitor_connector(monitor).await?;
        let started = screencast::start_screencast(&self.conn, &connector, framerate).await?;
        self.screencasts
            .lock()
            .await
            .insert(started.node_id, started.session);

        Ok(serde_json::json!({
            "node_id": started.node_id,
            "width": started.width,
            "height": started.height,
        }))
    }

    async fn stop_screencast(&self, node_id: u32) -> Result<()> {
        let session = self
            .screencasts
            .lock()
            .await
            .remove(&node_id)
            .ok_or_else(|| anyhow!("unknown screencast node_id: {node_id}"))?;
        screencast::stop_screencast(&self.conn, &session).await
    }

    fn desktop_name(&self) -> &'static str {
        "GNOME"
    }

    fn capabilities(&self) -> &'static [&'static str] {
        #[cfg(feature = "pipewire")]
        {
            &[
                "window",
                "notifications",
                "display",
                "idle",
                "inject",
                "screenshot",
                "screencast",
                "audio",
            ]
        }
        #[cfg(not(feature = "pipewire"))]
        &[
            "window",
            "notifications",
            "display",
            "idle",
            "inject",
            "screenshot",
        ]
    }
}

/// Detected RemoteDesktop API version — GNOME 43+ uses `(dddd)`,
/// GNOME 42 uses `(sdd)` which requires a screencast stream for absolute
/// motion. We fall back to relative motion + button injection for old API.
#[derive(Clone, Copy, PartialEq)]
enum InputApi {
    New,
    Old,
}

/// Detect the RemoteDesktop API by introspecting the session's
/// NotifyPointerMotionAbsolute method signature.
async fn detect_input_api(conn: &zbus::Connection, path: &OwnedObjectPath) -> Result<InputApi> {
    let introspect_proxy = zbus::Proxy::new(
        conn,
        MUTTER_DEST,
        path.as_str(),
        "org.freedesktop.DBus.Introspectable",
    )
    .await
    .context("creating introspection proxy")?;

    let xml: String = introspect_proxy
        .call("Introspect", &())
        .await
        .context("introspecting session")?;

    if xml.contains("type=\"s\"") {
        info!("old RemoteDesktop API detected — using relative motion + position tracking");
        Ok(InputApi::Old)
    } else {
        info!("new RemoteDesktop API detected — using absolute motion injection");
        Ok(InputApi::New)
    }
}

impl GnomeInputSession {
    pub async fn new() -> Result<Self> {
        let conn = zbus::Connection::session()
            .await
            .context("connecting to session bus for input injection")?;
        let proxy = zbus::Proxy::new(&conn, MUTTER_DEST, MUTTER_PATH, MUTTER_IFACE)
            .await
            .context("creating remote desktop proxy")?;

        let path: OwnedObjectPath = proxy
            .call("CreateSession", &())
            .await
            .context("creating remote desktop session")?;

        let session_proxy = zbus::Proxy::new(&conn, MUTTER_DEST, path.as_str(), SESSION_IFACE)
            .await
            .context("creating remote desktop session proxy")?;
        let start_result: Result<(), zbus::Error> =
            session_proxy.call("Start", &(DEVICE_TYPES_ALL)).await;
        if start_result.is_err() {
            let _: () = session_proxy
                .call("Start", &())
                .await
                .context("starting remote desktop session")?;
        }

        let api = detect_input_api(&conn, &path)
            .await
            .unwrap_or(InputApi::New);

        Ok(Self {
            conn,
            path,
            lock: Arc::new(Mutex::new(())),
            api,
            cursor_x: Arc::new(Mutex::new(960.0)),
            cursor_y: Arc::new(Mutex::new(540.0)),
        })
    }

    async fn send_sequence(&self, sequence: &[(u32, bool)]) -> Result<()> {
        for (keycode, shift) in sequence {
            if *shift {
                self.notify_keyboard(KEY_PRESSED, 42).await?;
            }
            self.tap_key(*keycode).await?;
            if *shift {
                self.notify_keyboard(KEY_RELEASED, 42).await?;
            }
        }
        Ok(())
    }

    async fn tap_key(&self, keycode: u32) -> Result<()> {
        self.notify_keyboard(KEY_PRESSED, keycode).await?;
        self.notify_keyboard(KEY_RELEASED, keycode).await?;
        Ok(())
    }

    async fn notify_keyboard(&self, keystate: bool, keycode: u32) -> Result<()> {
        let proxy = self.session_proxy().await?;
        let _: () = proxy
            .call("NotifyKeyboardKeycode", &(keycode, keystate))
            .await
            .with_context(|| {
                format!("injecting keyboard event keystate={keystate} keycode={keycode}")
            })?;
        Ok(())
    }

    async fn notify_pointer_motion_absolute(
        &self,
        x: f64,
        y: f64,
        _stream_width: f64,
        _stream_height: f64,
    ) -> Result<()> {
        let proxy = self.session_proxy().await?;
        match self.api {
            InputApi::New => {
                let _: () = proxy
                    .call(
                        "NotifyPointerMotionAbsolute",
                        &(x, y, _stream_width, _stream_height),
                    )
                    .await
                    .context("injecting absolute pointer motion")?;
                Ok(())
            }
            InputApi::Old => {
                // Use NotifyPointerMotionRelative — works on all GNOME versions
                // without a screencast stream. Compute delta from tracked position.
                //
                // Note: tracked position may drift if the user moves the cursor
                // between commands. Absolute motion on GNOME 43+ avoids this.
                let mut cx = self.cursor_x.lock().await;
                let mut cy = self.cursor_y.lock().await;
                let dx = x - *cx;
                let dy = y - *cy;
                let _: () = proxy
                    .call("NotifyPointerMotionRelative", &(dx, dy))
                    .await
                    .context("injecting relative pointer motion (old API fallback)")?;
                *cx = x;
                *cy = y;
                Ok(())
            }
        }
    }

    async fn notify_pointer_button(&self, button: i32, state: bool) -> Result<()> {
        let proxy = self.session_proxy().await?;
        match proxy.call("NotifyPointerButton", &(button, state)).await {
            Ok(()) => Ok(()),
            Err(e) => {
                warn!("NotifyPointerButton DBus error: {e:#}");
                Err(anyhow::anyhow!("injecting pointer button event: {e:#}"))
            }
        }
    }

    async fn notify_pointer_axis(&self, axis: u32, value: f64, finish: bool) -> Result<()> {
        let proxy = self.session_proxy().await?;
        let _: () = proxy
            .call("NotifyPointerAxis", &(axis, value, finish))
            .await
            .context("injecting pointer axis event")?;
        Ok(())
    }

    async fn session_proxy(&self) -> Result<zbus::Proxy<'_>> {
        debug!("using input session {}", self.path.as_str());
        zbus::Proxy::new(&self.conn, MUTTER_DEST, self.path.as_str(), SESSION_IFACE)
            .await
            .context("creating session proxy")
    }
}

#[async_trait]
impl InputBackend for GnomeInputSession {
    async fn type_text(&self, text: &str) -> Result<()> {
        let _guard = self.lock.lock().await;
        for character in text.chars() {
            match character {
                '\n' => self.tap_key(28).await?,
                '\t' => self.tap_key(15).await?,
                ' ' => self.tap_key(57).await?,
                ch => {
                    let sequence = key_sequence_for_char(ch)
                        .ok_or_else(|| anyhow!("unsupported character for inject:type: {ch:?}"))?;
                    self.send_sequence(&sequence).await?;
                }
            }
        }
        Ok(())
    }

    async fn send_keys(&self, keys: &[String]) -> Result<()> {
        if keys.is_empty() {
            return Err(anyhow!("inject:key requires at least one key"));
        }

        let _guard = self.lock.lock().await;
        let mut keycodes = Vec::with_capacity(keys.len());
        for key in keys {
            keycodes.push(keycode_for_name(key).ok_or_else(|| anyhow!("unknown key: {key}"))?);
        }

        for keycode in &keycodes[..keycodes.len().saturating_sub(1)] {
            self.notify_keyboard(KEY_PRESSED, *keycode).await?;
        }
        if let Some(last) = keycodes.last().copied() {
            self.notify_keyboard(KEY_PRESSED, last).await?;
            self.notify_keyboard(KEY_RELEASED, last).await?;
        }
        for keycode in keycodes[..keycodes.len().saturating_sub(1)].iter().rev() {
            self.notify_keyboard(KEY_RELEASED, *keycode).await?;
        }
        Ok(())
    }

    async fn mouse_action(&self, params: &Value) -> Result<()> {
        let _guard = self.lock.lock().await;
        let kind = params
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("missing 'type' param"))?;
        match kind {
            "move" => {
                let x = params.get("x").and_then(Value::as_f64).unwrap_or(0.0);
                let y = params.get("y").and_then(Value::as_f64).unwrap_or(0.0);
                self.notify_pointer_motion_absolute(x, y, 1.0, 1.0).await?;
            }
            "click" => {
                let x = params.get("x").and_then(Value::as_f64).unwrap_or(0.0);
                let y = params.get("y").and_then(Value::as_f64).unwrap_or(0.0);
                let button_name = params
                    .get("button")
                    .and_then(Value::as_str)
                    .unwrap_or("left");
                let button = pointer_button_code(button_name)?;
                self.notify_pointer_motion_absolute(x, y, 1.0, 1.0).await?;
                self.notify_pointer_button(button as i32, BUTTON_PRESSED)
                    .await?;
                self.notify_pointer_button(button as i32, BUTTON_RELEASED)
                    .await?;
            }
            "scroll" => {
                let dx = params.get("dx").and_then(Value::as_f64).unwrap_or(0.0);
                let dy = params.get("dy").and_then(Value::as_f64).unwrap_or(0.0);
                if dx != 0.0 {
                    self.notify_pointer_axis(0, dx, true).await?;
                }
                if dy != 0.0 {
                    self.notify_pointer_axis(1, dy, true).await?;
                }
            }
            other => return Err(anyhow!("unsupported mouse action: {other}")),
        }
        Ok(())
    }
}

fn property_bool(properties: &HashMap<String, OwnedValue>, key: &str) -> bool {
    properties
        .get(key)
        .and_then(|value| bool::try_from(value.clone()).ok())
        .unwrap_or(false)
}

fn parse_notification_block(block: &[String]) -> Option<serde_json::Value> {
    if !block.iter().any(|line| line.contains("member=Notify")) {
        return None;
    }

    let strings: Vec<String> = block
        .iter()
        .filter_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix("string ")
                .map(|value| value.trim_matches('"').to_string())
        })
        .collect();
    let uints: Vec<u64> = block
        .iter()
        .filter_map(|line| line.trim().strip_prefix("uint32 ")?.parse::<u64>().ok())
        .collect();

    if strings.len() < 4 {
        return None;
    }

    Some(serde_json::json!({
        "app": strings.first().cloned().unwrap_or_default(),
        "app_icon": strings.get(1).cloned().unwrap_or_default(),
        "summary": strings.get(2).cloned().unwrap_or_default(),
        "body": strings.get(3).cloned().unwrap_or_default(),
        "urgency": "normal",
        "id": uints.first().copied().unwrap_or(0),
    }))
}

fn pointer_button_code(button: &str) -> Result<u32> {
    match button {
        "left" => Ok(0x110),
        "right" => Ok(0x111),
        "middle" => Ok(0x112),
        other => Err(anyhow!("unsupported pointer button: {other}")),
    }
}

fn key_sequence_for_char(character: char) -> Option<Vec<(u32, bool)>> {
    // Direct punctuation mappings
    let (keycode, shifted) = match character {
        '.' => (52, true),
        ',' => (51, false),
        '-' => (12, false),
        '_' => (12, true),
        '/' => (53, false),
        '?' => (53, true),
        '!' => (2, true),
        '@' => (3, true),
        '#' => (4, true),
        '$' => (5, true),
        '%' => (6, true),
        '^' => (7, true),
        '&' => (8, true),
        '*' => (9, true),
        '(' => (10, true),
        ')' => (11, true),
        '\'' => (40, false),
        '"' => (40, true),
        ':' => (39, true),
        ';' => (39, false),
        '<' => (51, true),
        '>' => (52, true),
        '=' => (13, false),
        '+' => (13, true),
        '~' => (41, true),
        '`' => (41, false),
        '|' => (43, true),
        '\\' => (43, false),
        ch => {
            let lowercase = ch.to_ascii_lowercase();
            let mut key_name = [0_u8; 4];
            let key = lowercase.encode_utf8(&mut key_name);
            let kc = keycode_for_name(key)?;
            (kc, ch.is_ascii_uppercase())
        }
    };
    Some(vec![(keycode, shifted)])
}

fn keycode_for_name(name: &str) -> Option<u32> {
    match name.to_ascii_lowercase().as_str() {
        "a" => Some(30),
        "b" => Some(48),
        "c" => Some(46),
        "d" => Some(32),
        "e" => Some(18),
        "f" => Some(33),
        "g" => Some(34),
        "h" => Some(35),
        "i" => Some(23),
        "j" => Some(36),
        "k" => Some(37),
        "l" => Some(38),
        "m" => Some(50),
        "n" => Some(49),
        "o" => Some(24),
        "p" => Some(25),
        "q" => Some(16),
        "r" => Some(19),
        "s" => Some(31),
        "t" => Some(20),
        "u" => Some(22),
        "v" => Some(47),
        "w" => Some(17),
        "x" => Some(45),
        "y" => Some(21),
        "z" => Some(44),
        "0" => Some(11),
        "1" => Some(2),
        "2" => Some(3),
        "3" => Some(4),
        "4" => Some(5),
        "5" => Some(6),
        "6" => Some(7),
        "7" => Some(8),
        "8" => Some(9),
        "9" => Some(10),
        "enter" => Some(28),
        "tab" => Some(15),
        "escape" => Some(1),
        "backspace" => Some(14),
        "delete" => Some(111),
        "space" => Some(57),
        "ctrl" => Some(29),
        "control" => Some(29),
        "shift" => Some(42),
        "alt" => Some(56),
        "meta" => Some(125),
        "win" => Some(125),
        "super" => Some(125),
        "home" => Some(102),
        "end" => Some(107),
        "pageup" => Some(104),
        "pagedown" => Some(109),
        "insert" => Some(110),
        "left" => Some(105),
        "right" => Some(106),
        "up" => Some(103),
        "down" => Some(108),
        "f1" => Some(59),
        "f2" => Some(60),
        "f3" => Some(61),
        "f4" => Some(62),
        "f5" => Some(63),
        "f6" => Some(64),
        "f7" => Some(65),
        "f8" => Some(66),
        "f9" => Some(67),
        "f10" => Some(68),
        "f11" => Some(87),
        "f12" => Some(88),
        _ => None,
    }
}
