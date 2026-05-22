use super::*;
use crate::backend::DesktopBackend;
use crate::protocol;
use async_trait::async_trait;

#[async_trait]
impl DesktopBackend for WayfireBackend {
    async fn windows_list(&self) -> anyhow::Result<Vec<protocol::WindowInfo>> {
        windows::windows_list(self).await
    }
    async fn window_focus(&self, id: &str) -> anyhow::Result<()> {
        windows::window_focus(self, id).await
    }
    async fn window_get(&self, id: &str) -> anyhow::Result<protocol::WindowInfo> {
        windows::window_get(self, id).await
    }
    async fn window_close(&self, id: &str) -> anyhow::Result<()> {
        windows::window_close(self, id).await
    }
    async fn window_minimize(&self, id: &str) -> anyhow::Result<()> {
        windows::window_minimize(self, id).await
    }
    async fn window_maximize(&self, id: &str) -> anyhow::Result<()> {
        windows::window_maximize(self, id).await
    }
    async fn window_move_resize(
        &self,
        _id: &str,
        _x: i32,
        _y: i32,
        _w: u32,
        _h: u32,
    ) -> anyhow::Result<()> {
        windows::window_move_resize(self, _id, _x, _y, _w, _h).await
    }
    async fn workspaces_list(&self) -> anyhow::Result<Vec<protocol::WorkspaceInfo>> {
        workspaces::workspaces_list(self).await
    }
    async fn workspace_switch(&self, id: u32) -> anyhow::Result<()> {
        workspaces::workspace_switch(self, id).await
    }
    async fn workspace_move_window(
        &self,
        window_id: &str,
        workspace_id: u32,
        _follow: bool,
    ) -> anyhow::Result<()> {
        workspaces::workspace_move_window(self, window_id, workspace_id, _follow).await
    }
    async fn keyboard_type(&self, text: &str) -> anyhow::Result<()> {
        workspaces::keyboard_type(self, text).await
    }
    async fn keyboard_key(&self, key: &str) -> anyhow::Result<()> {
        workspaces::keyboard_key(self, key).await
    }
    async fn keyboard_combo(&self, keys: &[String]) -> anyhow::Result<()> {
        workspaces::keyboard_combo(self, keys).await
    }
    async fn mouse_move(&self, x: f64, y: f64) -> anyhow::Result<()> {
        workspaces::mouse_move(self, x, y).await
    }
    async fn mouse_click(&self, button: &str) -> anyhow::Result<()> {
        workspaces::mouse_click(self, button).await
    }
    async fn mouse_scroll(&self, dx: f64, dy: f64) -> anyhow::Result<()> {
        workspaces::mouse_scroll(self, dx, dy).await
    }
    async fn clipboard_read(&self) -> anyhow::Result<String> {
        workspaces::clipboard_read(self).await
    }
    async fn clipboard_write(&self, text: &str) -> anyhow::Result<()> {
        workspaces::clipboard_write(self, text).await
    }
    async fn screenshot(
        &self,
        _monitor: Option<u32>,
        region: Option<protocol::Region>,
        _window_id: Option<String>,
    ) -> anyhow::Result<protocol::ScreenshotResult> {
        screenshot::screenshot(self, _monitor, region, _window_id).await
    }
    async fn notification_send(
        &self,
        app_name: &str,
        title: &str,
        body: &str,
        urgency: &str,
    ) -> anyhow::Result<u32> {
        notifications::notification_send(self, app_name, title, body, urgency).await
    }
    async fn notification_close(&self, id: u32) -> anyhow::Result<()> {
        notifications::notification_close(self, id).await
    }
    async fn system_info(&self) -> anyhow::Result<protocol::SystemInfo> {
        system_info::system_info(self).await
    }
    async fn idle_seconds(&self) -> anyhow::Result<u64> {
        system_info::idle_seconds(self).await
    }
    async fn power_action(&self, action: &str) -> anyhow::Result<()> {
        system_info::power_action(self, action).await
    }
    async fn battery_status(&self) -> anyhow::Result<Vec<protocol::BatteryInfo>> {
        system_info::battery_status(self).await
    }
    async fn network_status(&self) -> anyhow::Result<protocol::NetworkStatusInfo> {
        networking::network_status(self).await
    }
    async fn network_interfaces(&self) -> anyhow::Result<Vec<protocol::NetworkInterfaceInfo>> {
        networking::network_interfaces(self).await
    }
    async fn wifi_scan(&self) -> anyhow::Result<Vec<protocol::WifiNetworkInfo>> {
        networking::wifi_scan(self).await
    }
    async fn wifi_connect(&self, ssid: &str, pw: Option<&str>) -> anyhow::Result<()> {
        networking::wifi_connect(self, ssid, pw).await
    }
    async fn bluetooth_list(&self) -> anyhow::Result<Vec<protocol::BluetoothDeviceInfo>> {
        bluetooth::bluetooth_list(self).await
    }
    async fn bluetooth_scan(&self, _duration: Option<u32>) -> anyhow::Result<()> {
        bluetooth::bluetooth_scan(self, _duration).await
    }
    async fn bluetooth_stop_scan(&self) -> anyhow::Result<()> {
        bluetooth::bluetooth_stop_scan(self).await
    }
    async fn bluetooth_connect(&self, address: &str) -> anyhow::Result<()> {
        bluetooth::bluetooth_connect(self, address).await
    }
    async fn bluetooth_disconnect(&self, address: &str) -> anyhow::Result<()> {
        bluetooth::bluetooth_disconnect(self, address).await
    }
    async fn files_watch(
        &self,
        path: &str,
        recursive: bool,
        _patterns: Option<&[String]>,
    ) -> anyhow::Result<()> {
        files::files_watch(self, path, recursive, _patterns).await
    }
    async fn files_unwatch(&self, path: &str) -> anyhow::Result<()> {
        files::files_unwatch(self, path).await
    }
    async fn files_search(
        &self,
        pattern: &str,
        root: Option<&str>,
        max_results: u32,
    ) -> anyhow::Result<Vec<String>> {
        files::files_search(self, pattern, root, max_results).await
    }
    async fn audio_list_sinks(&self) -> anyhow::Result<Vec<protocol::AudioSinkInfo>> {
        audio::audio_list_sinks(self).await
    }
    async fn audio_set_sink_volume(&self, sink_id: u32, volume: f64) -> anyhow::Result<()> {
        audio::audio_set_sink_volume(self, sink_id, volume).await
    }
    async fn monitor_set_primary(&self, _output: &str) -> anyhow::Result<()> {
        monitor::monitor_set_primary(self, _output).await
    }
    async fn monitor_set_resolution(
        &self,
        _output: &str,
        _w: u32,
        _h: u32,
        _rr: Option<f64>,
    ) -> anyhow::Result<()> {
        monitor::monitor_set_resolution(self, _output, _w, _h, _rr).await
    }
    async fn monitor_set_scale(&self, _output: &str, _scale: f64) -> anyhow::Result<()> {
        monitor::monitor_set_scale(self, _output, _scale).await
    }
    async fn monitor_set_rotation(&self, _output: &str, _rotation: &str) -> anyhow::Result<()> {
        monitor::monitor_set_rotation(self, _output, _rotation).await
    }
    async fn monitor_set_enabled(&self, _output: &str, _enabled: bool) -> anyhow::Result<()> {
        monitor::monitor_set_enabled(self, _output, _enabled).await
    }
}
