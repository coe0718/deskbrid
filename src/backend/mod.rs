pub mod gnome;
use crate::protocol;
use async_trait::async_trait;

/// Create the default backend for the current desktop environment.
pub async fn create_backend() -> anyhow::Result<Box<dyn DesktopBackend>> {
    gnome::GnomeBackend::new()
        .await
        .map(|b| Box::new(b) as Box<dyn DesktopBackend>)
}

/// The DesktopBackend trait defines all actions deskbrid can perform on a desktop
/// environment. Only GNOME 46+ is supported in v2.
#[async_trait]
pub trait DesktopBackend: Send + Sync {
    // ─── Windows ────────────────────────────────────────
    async fn windows_list(&self) -> anyhow::Result<Vec<protocol::WindowInfo>>;
    async fn window_focus(&self, id: &str) -> anyhow::Result<()>;
    async fn window_get(&self, id: &str) -> anyhow::Result<protocol::WindowInfo>;

    // ─── Workspaces ─────────────────────────────────────
    async fn workspaces_list(&self) -> anyhow::Result<Vec<protocol::WorkspaceInfo>>;
    async fn workspace_switch(&self, id: u32) -> anyhow::Result<()>;
    async fn workspace_move_window(
        &self,
        window_id: &str,
        workspace_id: u32,
        follow: bool,
    ) -> anyhow::Result<()>;

    // ─── Input ──────────────────────────────────────────
    async fn keyboard_type(&self, text: &str) -> anyhow::Result<()>;
    async fn keyboard_key(&self, key: &str) -> anyhow::Result<()>;
    async fn keyboard_combo(&self, keys: &[String]) -> anyhow::Result<()>;
    async fn mouse_move(&self, x: f64, y: f64) -> anyhow::Result<()>;
    async fn mouse_click(&self, button: &str) -> anyhow::Result<()>;
    async fn mouse_scroll(&self, dx: f64, dy: f64) -> anyhow::Result<()>;

    // ─── Clipboard ──────────────────────────────────────
    async fn clipboard_read(&self) -> anyhow::Result<String>;
    async fn clipboard_write(&self, text: &str) -> anyhow::Result<()>;

    // ─── Screenshot ─────────────────────────────────────
    async fn screenshot(
        &self,
        monitor: Option<u32>,
        region: Option<protocol::Region>,
        window_id: Option<String>,
    ) -> anyhow::Result<protocol::ScreenshotResult>;

    // ─── Notifications ──────────────────────────────────
    async fn notification_send(
        &self,
        app_name: &str,
        title: &str,
        body: &str,
        urgency: &str,
    ) -> anyhow::Result<u32>;
    async fn notification_close(&self, id: u32) -> anyhow::Result<()>;

    // ─── System ─────────────────────────────────────────
    async fn system_info(&self) -> anyhow::Result<protocol::SystemInfo>;
    async fn idle_seconds(&self) -> anyhow::Result<u64>;
    async fn power_action(&self, action: &str) -> anyhow::Result<()>;
    async fn battery_status(&self) -> anyhow::Result<Vec<protocol::BatteryInfo>>;

    // ─── Network ────────────────────────────────────────
    async fn network_status(&self) -> anyhow::Result<protocol::NetworkStatusInfo>;
    async fn network_interfaces(&self) -> anyhow::Result<Vec<protocol::NetworkInterfaceInfo>>;
    async fn wifi_scan(&self) -> anyhow::Result<Vec<protocol::WifiNetworkInfo>>;
    async fn wifi_connect(&self, ssid: &str, password: Option<&str>) -> anyhow::Result<()>;

    // ─── Bluetooth ──────────────────────────────────────
    async fn bluetooth_list(&self) -> anyhow::Result<Vec<protocol::BluetoothDeviceInfo>>;
    async fn bluetooth_scan(&self, duration: Option<u32>) -> anyhow::Result<()>;
    async fn bluetooth_stop_scan(&self) -> anyhow::Result<()>;
    async fn bluetooth_connect(&self, address: &str) -> anyhow::Result<()>;
    async fn bluetooth_disconnect(&self, address: &str) -> anyhow::Result<()>;

    // ─── Files ──────────────────────────────────────────
    async fn files_watch(
        &self,
        path: &str,
        recursive: bool,
        patterns: Option<&[String]>,
    ) -> anyhow::Result<()>;
    async fn files_unwatch(&self, path: &str) -> anyhow::Result<()>;
    async fn files_search(
        &self,
        pattern: &str,
        root: Option<&str>,
        max_results: u32,
    ) -> anyhow::Result<Vec<String>>;

    // ─── Audio ──────────────────────────────────────────
    async fn audio_list_sinks(&self) -> anyhow::Result<Vec<protocol::AudioSinkInfo>>;
    async fn audio_set_sink_volume(&self, sink_id: u32, volume: f64) -> anyhow::Result<()>;
}
