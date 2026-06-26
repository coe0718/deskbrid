use crate::protocol::{
    AudioSinkInfo, BacklightInfo, BatteryInfo, BluetoothDeviceInfo, Geometry, KeyboardLayout,
    MonitorInfo, NetworkInterfaceInfo, NetworkStatusInfo, PrintJob, PrintPrinter, Region,
    ScreenshotResult, SystemInfo, WifiNetworkInfo, WindowInfo, WorkspaceInfo,
};
use anyhow::Context;
use async_trait::async_trait;
use image::{ImageBuffer, Rgba};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::DesktopBackend;

pub struct MockBackend {
    state: Mutex<MockState>,
}

#[derive(Clone, serde::Deserialize)]
#[serde(default)]
struct MockScenario {
    windows: Vec<WindowInfo>,
    workspaces: Vec<WorkspaceInfo>,
    monitors: Vec<MonitorInfo>,
    clipboard: String,
    keyboard_layouts: Vec<KeyboardLayout>,
    current_layout_index: u32,
}

impl Default for MockScenario {
    fn default() -> Self {
        let state = MockState::default();
        Self {
            windows: state.windows,
            workspaces: state.workspaces,
            monitors: state.monitors,
            clipboard: state.clipboard,
            keyboard_layouts: state.keyboard_layouts,
            current_layout_index: state.current_layout_index,
        }
    }
}

#[derive(Clone)]
struct MockState {
    windows: Vec<WindowInfo>,
    workspaces: Vec<WorkspaceInfo>,
    monitors: Vec<MonitorInfo>,
    clipboard: String,
    keyboard_layouts: Vec<KeyboardLayout>,
    current_layout_index: u32,
    audio_sinks: Vec<AudioSinkInfo>,
    bluetooth_devices: Vec<BluetoothDeviceInfo>,
    watched_paths: HashSet<String>,
    desktop_settings: HashMap<String, String>,
    notifications: HashSet<u32>,
    next_notification_id: u32,
    mouse_x: f64,
    mouse_y: f64,
    typed_text: String,
    bluetooth_scanning: bool,
    backlight: BacklightInfo,
}

impl Default for MockState {
    fn default() -> Self {
        Self {
            windows: vec![
                WindowInfo {
                    id: "mock-terminal".into(),
                    title: "Mock Terminal".into(),
                    app_id: "org.mock.Terminal".into(),
                    workspace_id: 1,
                    is_focused: true,
                    is_minimized: false,
                    geometry: Some(Geometry {
                        x: 80,
                        y: 80,
                        width: 900,
                        height: 560,
                    }),
                    pid: Some(4242),
                },
                WindowInfo {
                    id: "mock-browser".into(),
                    title: "Mock Browser".into(),
                    app_id: "org.mock.Browser".into(),
                    workspace_id: 1,
                    is_focused: false,
                    is_minimized: false,
                    geometry: Some(Geometry {
                        x: 1020,
                        y: 80,
                        width: 820,
                        height: 720,
                    }),
                    pid: Some(4243),
                },
                WindowInfo {
                    id: "mock-editor".into(),
                    title: "Mock Editor".into(),
                    app_id: "org.mock.Editor".into(),
                    workspace_id: 2,
                    is_focused: false,
                    is_minimized: false,
                    geometry: Some(Geometry {
                        x: 140,
                        y: 120,
                        width: 1200,
                        height: 760,
                    }),
                    pid: Some(4244),
                },
            ],
            workspaces: vec![
                WorkspaceInfo {
                    id: 1,
                    name: "Mock Workspace 1".into(),
                    is_active: true,
                },
                WorkspaceInfo {
                    id: 2,
                    name: "Mock Workspace 2".into(),
                    is_active: false,
                },
            ],
            monitors: vec![MonitorInfo {
                id: 0,
                name: "mock-0".into(),
                width: 1920,
                height: 1080,
                scale: 1.0,
                primary: true,
                enabled: true,
                x: 0,
                y: 0,
                refresh_rate: Some(60.0),
                rotation: "normal".into(),
            }],
            clipboard: "mock clipboard".into(),
            keyboard_layouts: vec![
                KeyboardLayout {
                    index: 0,
                    name: "us".into(),
                    variant: None,
                    display_name: Some("English (US)".into()),
                },
                KeyboardLayout {
                    index: 1,
                    name: "de".into(),
                    variant: None,
                    display_name: Some("German".into()),
                },
            ],
            current_layout_index: 0,
            audio_sinks: vec![AudioSinkInfo {
                id: 0,
                name: "mock-sink".into(),
                description: "Mock Speakers".into(),
                volume: 0.5,
                muted: false,
            }],
            bluetooth_devices: vec![BluetoothDeviceInfo {
                address: "00:11:22:33:44:55".into(),
                name: "Mock Headset".into(),
                paired: true,
                connected: false,
                rssi: Some(-42),
            }],
            watched_paths: HashSet::new(),
            desktop_settings: HashMap::new(),
            notifications: HashSet::new(),
            next_notification_id: 1,
            mouse_x: 0.0,
            mouse_y: 0.0,
            typed_text: String::new(),
            bluetooth_scanning: false,
            backlight: BacklightInfo {
                device: "mock-backlight".into(),
                max_brightness: 1000,
                brightness: 500,
                percentage: 50,
            },
        }
    }
}

impl MockBackend {
    pub async fn new(
        _event_tx: tokio::sync::broadcast::Sender<crate::protocol::DeskbridEvent>,
    ) -> anyhow::Result<Self> {
        let state = match std::env::var_os("DESKBRID_MOCK_SCENARIO") {
            Some(path) => MockState::from_scenario_path(Path::new(&path))?,
            None => MockState::default(),
        };
        Ok(Self {
            state: Mutex::new(state),
        })
    }

    fn with_state<T>(&self, f: impl FnOnce(&MockState) -> anyhow::Result<T>) -> anyhow::Result<T> {
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("mock backend state lock poisoned"))?;
        f(&state)
    }

    fn with_state_mut<T>(
        &self,
        f: impl FnOnce(&mut MockState) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("mock backend state lock poisoned"))?;
        f(&mut state)
    }
}

impl MockState {
    fn from_scenario_path(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read mock scenario {}", path.display()))?;
        let scenario: MockScenario = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse mock scenario {}", path.display()))?;
        Ok(Self::from_scenario(scenario))
    }

    fn from_scenario(scenario: MockScenario) -> Self {
        let mut state = Self {
            windows: scenario.windows,
            workspaces: scenario.workspaces,
            monitors: scenario.monitors,
            clipboard: scenario.clipboard,
            keyboard_layouts: scenario.keyboard_layouts,
            current_layout_index: scenario.current_layout_index,
            ..Self::default()
        };
        state.ensure_consistent_focus();
        state
    }

    fn ensure_consistent_focus(&mut self) {
        if self.windows.iter().filter(|w| w.is_focused).count() == 1 {
            return;
        }
        for window in &mut self.windows {
            window.is_focused = false;
        }
        if let Some(window) = self.windows.first_mut() {
            window.is_focused = true;
        }
    }

    fn find_window(&self, id: &str) -> anyhow::Result<WindowInfo> {
        self.windows
            .iter()
            .find(|window| window.id == id || window.title == id || window.app_id == id)
            .cloned()
            .with_context(|| format!("mock window not found: {id}"))
    }

    fn find_window_mut(&mut self, id: &str) -> anyhow::Result<&mut WindowInfo> {
        self.windows
            .iter_mut()
            .find(|window| window.id == id || window.title == id || window.app_id == id)
            .with_context(|| format!("mock window not found: {id}"))
    }

    fn active_workspace_id(&self) -> u32 {
        self.workspaces
            .iter()
            .find(|workspace| workspace.is_active)
            .map(|workspace| workspace.id)
            .unwrap_or(1)
    }

    fn primary_monitor(&self) -> MonitorInfo {
        self.monitors
            .iter()
            .find(|monitor| monitor.primary && monitor.enabled)
            .or_else(|| self.monitors.iter().find(|monitor| monitor.enabled))
            .or_else(|| self.monitors.first())
            .cloned()
            .unwrap_or(MonitorInfo {
                id: 0,
                name: "mock-0".into(),
                width: 1920,
                height: 1080,
                scale: 1.0,
                primary: true,
                enabled: true,
                x: 0,
                y: 0,
                refresh_rate: Some(60.0),
                rotation: "normal".into(),
            })
    }

    fn monitor_by_id(&self, id: Option<u32>) -> MonitorInfo {
        id.and_then(|monitor_id| {
            self.monitors
                .iter()
                .find(|monitor| monitor.id == monitor_id)
                .cloned()
        })
        .unwrap_or_else(|| self.primary_monitor())
    }
}

fn write_mock_screenshot(width: u32, height: u32) -> anyhow::Result<PathBuf> {
    let path = std::env::temp_dir().join(format!(
        "deskbrid-mock-screenshot-{}-{}.png",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    let mut image = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(width.max(1), height.max(1));
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        let stripe = if (x / 24 + y / 24) % 2 == 0 { 230 } else { 210 };
        *pixel = Rgba([stripe, stripe, stripe, 255]);
    }
    image
        .save(&path)
        .with_context(|| format!("failed to write mock screenshot {}", path.display()))?;
    Ok(path)
}

fn parse_brightness(value: &str, max_brightness: u32) -> anyhow::Result<u32> {
    if let Some(percent) = value.strip_suffix('%') {
        let pct: u32 = percent
            .trim()
            .parse()
            .context("invalid brightness percent")?;
        if pct > 100 {
            anyhow::bail!("brightness percent must be 0..100");
        }
        return Ok(max_brightness.saturating_mul(pct) / 100);
    }
    let brightness: u32 = value.trim().parse().context("invalid brightness value")?;
    Ok(brightness.min(max_brightness))
}

#[async_trait]
impl DesktopBackend for MockBackend {
    async fn windows_list(&self) -> anyhow::Result<Vec<WindowInfo>> {
        self.with_state(|state| Ok(state.windows.clone()))
    }

    async fn window_focus(&self, id: &str) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            let resolved_id = state.find_window(id)?.id;
            for window in &mut state.windows {
                window.is_focused = window.id == resolved_id;
                if window.is_focused {
                    window.is_minimized = false;
                }
            }
            Ok(())
        })
    }

    async fn window_get(&self, id: &str) -> anyhow::Result<WindowInfo> {
        self.with_state(|state| state.find_window(id))
    }

    async fn window_close(&self, id: &str) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            let resolved = state.find_window(id)?.id;
            state.windows.retain(|window| window.id != resolved);
            state.ensure_consistent_focus();
            Ok(())
        })
    }

    async fn window_minimize(&self, id: &str) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            state.find_window_mut(id)?.is_minimized = true;
            Ok(())
        })
    }

    async fn window_maximize(&self, id: &str) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            let monitor = state.primary_monitor();
            let window = state.find_window_mut(id)?;
            window.is_minimized = false;
            window.geometry = Some(Geometry {
                x: monitor.x,
                y: monitor.y,
                width: monitor.width,
                height: monitor.height,
            });
            Ok(())
        })
    }

    async fn window_move_resize(
        &self,
        id: &str,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            let window = state.find_window_mut(id)?;
            window.geometry = Some(Geometry {
                x,
                y,
                width,
                height,
            });
            Ok(())
        })
    }

    async fn workspaces_list(&self) -> anyhow::Result<Vec<WorkspaceInfo>> {
        self.with_state(|state| Ok(state.workspaces.clone()))
    }

    async fn workspace_switch(&self, id: u32) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            if !state.workspaces.iter().any(|workspace| workspace.id == id) {
                anyhow::bail!("mock workspace not found: {id}");
            }
            for workspace in &mut state.workspaces {
                workspace.is_active = workspace.id == id;
            }
            Ok(())
        })
    }

    async fn workspace_move_window(
        &self,
        window_id: &str,
        workspace_id: u32,
        follow: bool,
    ) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            if !state
                .workspaces
                .iter()
                .any(|workspace| workspace.id == workspace_id)
            {
                anyhow::bail!("mock workspace not found: {workspace_id}");
            }
            state.find_window_mut(window_id)?.workspace_id = workspace_id;
            if follow {
                for workspace in &mut state.workspaces {
                    workspace.is_active = workspace.id == workspace_id;
                }
            }
            Ok(())
        })
    }

    async fn keyboard_type(&self, text: &str) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            state.typed_text.push_str(text);
            Ok(())
        })
    }

    async fn keyboard_key(&self, key: &str) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            state.typed_text.push_str(&format!("<{key}>"));
            Ok(())
        })
    }

    async fn keyboard_combo(&self, keys: &[String]) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            state.typed_text.push_str(&format!("<{}>", keys.join("+")));
            Ok(())
        })
    }

    async fn mouse_move(&self, x: f64, y: f64) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            state.mouse_x = x;
            state.mouse_y = y;
            Ok(())
        })
    }

    async fn mouse_click(&self, _button: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn mouse_scroll(&self, _dx: f64, _dy: f64) -> anyhow::Result<()> {
        Ok(())
    }

    async fn mouse_drag(
        &self,
        _from_x: f64,
        _from_y: f64,
        to_x: f64,
        to_y: f64,
        _button: &str,
        _duration_ms: Option<u64>,
    ) -> anyhow::Result<()> {
        self.mouse_move(to_x, to_y).await
    }

    async fn keyboard_layout_list(&self) -> anyhow::Result<Vec<KeyboardLayout>> {
        self.with_state(|state| Ok(state.keyboard_layouts.clone()))
    }

    async fn keyboard_layout_get(&self) -> anyhow::Result<KeyboardLayout> {
        self.with_state(|state| {
            state
                .keyboard_layouts
                .iter()
                .find(|layout| layout.index == state.current_layout_index)
                .cloned()
                .context("mock current keyboard layout not found")
        })
    }

    async fn keyboard_layout_set(
        &self,
        index: Option<u32>,
        name: Option<&str>,
        variant: Option<&str>,
    ) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            let layout = state
                .keyboard_layouts
                .iter()
                .find(|layout| {
                    index == Some(layout.index)
                        || name.is_some_and(|n| {
                            layout.name == n
                                && variant.is_none_or(|v| layout.variant.as_deref() == Some(v))
                        })
                })
                .context("mock keyboard layout not found")?;
            state.current_layout_index = layout.index;
            Ok(())
        })
    }

    async fn keyboard_layout_add(&self, name: &str, variant: Option<&str>) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            let next_index = state
                .keyboard_layouts
                .iter()
                .map(|layout| layout.index)
                .max()
                .unwrap_or(0)
                + 1;
            state.keyboard_layouts.push(KeyboardLayout {
                index: next_index,
                name: name.into(),
                variant: variant.map(String::from),
                display_name: Some(match variant {
                    Some(v) => format!("{name} ({v})"),
                    None => name.to_string(),
                }),
            });
            Ok(())
        })
    }

    async fn keyboard_layout_remove(&self, index: u32) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            let before = state.keyboard_layouts.len();
            state
                .keyboard_layouts
                .retain(|layout| layout.index != index);
            if state.keyboard_layouts.len() == before {
                anyhow::bail!("mock keyboard layout not found: {index}");
            }
            if state.current_layout_index == index {
                state.current_layout_index = state
                    .keyboard_layouts
                    .first()
                    .map(|layout| layout.index)
                    .unwrap_or(0);
            }
            Ok(())
        })
    }

    async fn clipboard_read(&self) -> anyhow::Result<String> {
        self.with_state(|state| Ok(state.clipboard.clone()))
    }

    async fn clipboard_write(&self, text: &str) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            state.clipboard = text.into();
            Ok(())
        })
    }

    async fn screenshot(
        &self,
        monitor: Option<u32>,
        region: Option<Region>,
        _window_id: Option<String>,
    ) -> anyhow::Result<ScreenshotResult> {
        let (width, height) = self.with_state(|state| {
            let monitor = state.monitor_by_id(monitor);
            Ok(region
                .map(|region| (region.width, region.height))
                .unwrap_or((monitor.width, monitor.height)))
        })?;
        let path = write_mock_screenshot(width, height)?;
        Ok(ScreenshotResult {
            path: path.to_string_lossy().to_string(),
            width,
            height,
            format: "png".into(),
        })
    }

    async fn notification_send(
        &self,
        _app_name: &str,
        _title: &str,
        _body: &str,
        _urgency: &str,
    ) -> anyhow::Result<u32> {
        self.with_state_mut(|state| {
            let id = state.next_notification_id;
            state.next_notification_id = state.next_notification_id.saturating_add(1);
            state.notifications.insert(id);
            Ok(id)
        })
    }

    async fn notification_close(&self, id: u32) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            state.notifications.remove(&id);
            Ok(())
        })
    }

    async fn system_info(&self) -> anyhow::Result<SystemInfo> {
        self.with_state(|state| {
            Ok(SystemInfo {
                desktop: "Mock".into(),
                desktop_version: env!("CARGO_PKG_VERSION").into(),
                compositor: "mock-compositor".into(),
                session_type: "mock".into(),
                monitors: state.monitors.clone(),
                workspace_count: state.workspaces.len() as u32,
                current_workspace: state.active_workspace_id(),
                idle_seconds: 0,
            })
        })
    }

    async fn idle_seconds(&self) -> anyhow::Result<u64> {
        Ok(0)
    }

    async fn power_action(&self, action: &str) -> anyhow::Result<()> {
        match action {
            "lock" | "suspend" | "hibernate" | "reboot" | "shutdown" | "logout" => Ok(()),
            other => anyhow::bail!("unsupported mock power action: {other}"),
        }
    }

    async fn battery_status(&self) -> anyhow::Result<Vec<BatteryInfo>> {
        Ok(vec![BatteryInfo {
            source: "mock-battery".into(),
            percentage: 88.0,
            state: "discharging".into(),
            time_remaining_minutes: Some(240),
        }])
    }

    async fn network_status(&self) -> anyhow::Result<NetworkStatusInfo> {
        Ok(NetworkStatusInfo {
            online: true,
            net_type: "mock".into(),
        })
    }

    async fn network_interfaces(&self) -> anyhow::Result<Vec<NetworkInterfaceInfo>> {
        Ok(vec![NetworkInterfaceInfo {
            name: "mock0".into(),
            state: "connected".into(),
            ipv4: Some("192.0.2.10".into()),
            ipv6: None,
        }])
    }

    async fn wifi_scan(&self) -> anyhow::Result<Vec<WifiNetworkInfo>> {
        Ok(vec![WifiNetworkInfo {
            ssid: "Deskbrid Mock Wi-Fi".into(),
            strength: 90,
            secured: true,
            frequency: Some(5200),
        }])
    }

    async fn wifi_connect(&self, _ssid: &str, _password: Option<&str>) -> anyhow::Result<()> {
        Ok(())
    }

    async fn bluetooth_list(&self) -> anyhow::Result<Vec<BluetoothDeviceInfo>> {
        self.with_state(|state| Ok(state.bluetooth_devices.clone()))
    }

    async fn bluetooth_scan(&self, _duration: Option<u32>) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            state.bluetooth_scanning = true;
            Ok(())
        })
    }

    async fn bluetooth_stop_scan(&self) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            state.bluetooth_scanning = false;
            Ok(())
        })
    }

    async fn bluetooth_connect(&self, address: &str) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            let device = state
                .bluetooth_devices
                .iter_mut()
                .find(|device| device.address == address)
                .with_context(|| format!("mock bluetooth device not found: {address}"))?;
            device.connected = true;
            Ok(())
        })
    }

    async fn bluetooth_disconnect(&self, address: &str) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            let device = state
                .bluetooth_devices
                .iter_mut()
                .find(|device| device.address == address)
                .with_context(|| format!("mock bluetooth device not found: {address}"))?;
            device.connected = false;
            Ok(())
        })
    }

    async fn files_watch(
        &self,
        path: &str,
        _recursive: bool,
        _patterns: Option<&[String]>,
    ) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            state.watched_paths.insert(path.into());
            Ok(())
        })
    }

    async fn files_unwatch(&self, path: &str) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            state.watched_paths.remove(path);
            Ok(())
        })
    }

    async fn files_search(
        &self,
        pattern: &str,
        root: Option<&str>,
        max_results: u32,
    ) -> anyhow::Result<Vec<String>> {
        let root = root.unwrap_or("/mock");
        Ok((0..max_results.min(3))
            .map(|i| format!("{root}/mock-{pattern}-{i}.txt"))
            .collect())
    }

    async fn audio_list_sinks(&self) -> anyhow::Result<Vec<AudioSinkInfo>> {
        self.with_state(|state| Ok(state.audio_sinks.clone()))
    }

    async fn audio_set_sink_volume(&self, sink_id: u32, volume: f64) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            let sink = state
                .audio_sinks
                .iter_mut()
                .find(|sink| sink.id == sink_id)
                .with_context(|| format!("mock audio sink not found: {sink_id}"))?;
            sink.volume = volume.clamp(0.0, 1.0);
            Ok(())
        })
    }

    async fn monitor_set_primary(&self, output: &str) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            if !state.monitors.iter().any(|monitor| monitor.name == output) {
                anyhow::bail!("mock monitor not found: {output}");
            }
            for monitor in &mut state.monitors {
                monitor.primary = monitor.name == output;
            }
            Ok(())
        })
    }

    async fn monitor_set_resolution(
        &self,
        output: &str,
        width: u32,
        height: u32,
        refresh_rate: Option<f64>,
    ) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            let monitor = state
                .monitors
                .iter_mut()
                .find(|monitor| monitor.name == output)
                .with_context(|| format!("mock monitor not found: {output}"))?;
            monitor.width = width;
            monitor.height = height;
            monitor.refresh_rate = refresh_rate;
            Ok(())
        })
    }

    async fn monitor_set_scale(&self, output: &str, scale: f64) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            let monitor = state
                .monitors
                .iter_mut()
                .find(|monitor| monitor.name == output)
                .with_context(|| format!("mock monitor not found: {output}"))?;
            monitor.scale = scale;
            Ok(())
        })
    }

    async fn monitor_set_rotation(&self, output: &str, rotation: &str) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            let monitor = state
                .monitors
                .iter_mut()
                .find(|monitor| monitor.name == output)
                .with_context(|| format!("mock monitor not found: {output}"))?;
            monitor.rotation = rotation.into();
            Ok(())
        })
    }

    async fn monitor_set_enabled(&self, output: &str, enabled: bool) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            let monitor = state
                .monitors
                .iter_mut()
                .find(|monitor| monitor.name == output)
                .with_context(|| format!("mock monitor not found: {output}"))?;
            monitor.enabled = enabled;
            Ok(())
        })
    }

    async fn desktop_get_setting(&self, schema: &str, key: &str) -> anyhow::Result<String> {
        self.with_state(|state| {
            Ok(state
                .desktop_settings
                .get(&format!("{schema}:{key}"))
                .cloned()
                .unwrap_or_else(|| "mock-value".into()))
        })
    }

    async fn desktop_set_setting(
        &self,
        schema: &str,
        key: &str,
        value: &str,
    ) -> anyhow::Result<()> {
        self.with_state_mut(|state| {
            state
                .desktop_settings
                .insert(format!("{schema}:{key}"), value.into());
            Ok(())
        })
    }

    async fn desktop_list_schemas(&self) -> anyhow::Result<Vec<String>> {
        Ok(vec!["org.mock.desktop.interface".into()])
    }

    async fn backlight_list(&self) -> anyhow::Result<Vec<BacklightInfo>> {
        self.with_state(|state| Ok(vec![state.backlight.clone()]))
    }

    async fn backlight_get(&self, device: Option<&str>) -> anyhow::Result<BacklightInfo> {
        self.with_state(|state| {
            if device.is_some_and(|device| device != state.backlight.device) {
                anyhow::bail!("mock backlight device not found: {}", device.unwrap());
            }
            Ok(state.backlight.clone())
        })
    }

    async fn backlight_set(
        &self,
        device: Option<&str>,
        value: &str,
    ) -> anyhow::Result<BacklightInfo> {
        self.with_state_mut(|state| {
            if device.is_some_and(|device| device != state.backlight.device) {
                anyhow::bail!("mock backlight device not found: {}", device.unwrap());
            }
            let brightness = parse_brightness(value, state.backlight.max_brightness)?;
            state.backlight.brightness = brightness;
            state.backlight.percentage =
                ((brightness as f64 / state.backlight.max_brightness as f64) * 100.0).round() as u8;
            Ok(state.backlight.clone())
        })
    }

    async fn print_list(&self) -> anyhow::Result<Vec<PrintPrinter>> {
        Ok(vec![PrintPrinter {
            name: "Mock_Printer".into(),
            location: "Mock Lab".into(),
            status: "idle".into(),
            is_default: true,
            uri: Some("mock://printer".into()),
        }])
    }

    async fn print_default(&self, printer: Option<&str>) -> anyhow::Result<PrintPrinter> {
        let selected = printer.unwrap_or("Mock_Printer");
        Ok(PrintPrinter {
            name: selected.into(),
            location: "Mock Lab".into(),
            status: "idle".into(),
            is_default: true,
            uri: Some("mock://printer".into()),
        })
    }

    async fn print_jobs(&self) -> anyhow::Result<Vec<PrintJob>> {
        Ok(Vec::new())
    }

    async fn print_job_cancel(&self, _job_id: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn print_job_pause(&self, _job_id: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn print_job_resume(&self, _job_id: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn print_file(&self, printer: &str, path: &str) -> anyhow::Result<PrintJob> {
        Ok(PrintJob {
            id: "mock-job-1".into(),
            printer: printer.into(),
            user: "mock".into(),
            name: Path::new(path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("mock-document")
                .into(),
            size: Some("0".into()),
            status: "queued".into(),
            submitted: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_backend() -> MockBackend {
        MockBackend {
            state: Mutex::new(MockState::default()),
        }
    }

    #[tokio::test]
    async fn focus_updates_window_state() {
        let backend = test_backend();

        backend.window_focus("mock-browser").await.unwrap();
        let windows = backend.windows_list().await.unwrap();

        assert!(
            windows
                .iter()
                .any(|window| window.id == "mock-browser" && window.is_focused)
        );
        assert!(
            windows
                .iter()
                .any(|window| window.id == "mock-terminal" && !window.is_focused)
        );
    }

    #[tokio::test]
    async fn screenshot_writes_png() {
        let backend = test_backend();

        let result = backend
            .screenshot(
                Some(0),
                Some(Region {
                    x: 0,
                    y: 0,
                    width: 12,
                    height: 8,
                }),
                None,
            )
            .await
            .unwrap();

        assert_eq!(result.width, 12);
        assert_eq!(result.height, 8);
        assert!(Path::new(&result.path).exists());
        let _ = std::fs::remove_file(&result.path);
    }

    #[test]
    fn partial_scenario_keeps_default_desktop_shape() {
        let scenario: MockScenario =
            serde_json::from_str(r#"{"clipboard":"scenario clipboard"}"#).unwrap();
        let state = MockState::from_scenario(scenario);

        assert_eq!(state.clipboard, "scenario clipboard");
        assert!(!state.windows.is_empty());
        assert!(!state.workspaces.is_empty());
        assert!(!state.monitors.is_empty());
    }
}
