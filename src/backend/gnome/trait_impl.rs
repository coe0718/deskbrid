use super::*;
use crate::protocol;
use crate::protocol::Region;
use async_trait::async_trait;

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
        w: u32,
        h: u32,
    ) -> anyhow::Result<()> {
        self.window_move_resize_inner(id, x, y, w, h).await
    }

    async fn workspaces_list(&self) -> anyhow::Result<Vec<protocol::WorkspaceInfo>> {
        self.workspaces_list_inner().await
    }
    async fn workspace_switch(&self, id: u32) -> anyhow::Result<()> {
        self.workspace_switch_inner(id).await
    }
    async fn workspace_move_window(
        &self,
        wid: &str,
        wsid: u32,
        _follow: bool,
    ) -> anyhow::Result<()> {
        self.workspace_move_window_inner(wid, wsid).await
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
    async fn mouse_drag(
        &self,
        from_x: f64,
        from_y: f64,
        to_x: f64,
        to_y: f64,
        button: &str,
        duration_ms: Option<u64>,
    ) -> anyhow::Result<()> {
        self.mouse_drag_inner(from_x, from_y, to_x, to_y, button, duration_ms)
            .await
    }

    async fn clipboard_read(&self) -> anyhow::Result<String> {
        self.clipboard_read_inner().await
    }
    async fn clipboard_write(&self, text: &str) -> anyhow::Result<()> {
        self.clipboard_write_inner(text).await
    }

    async fn screenshot(
        &self,
        m: Option<u32>,
        r: Option<Region>,
        w: Option<String>,
    ) -> anyhow::Result<protocol::ScreenshotResult> {
        self.screenshot_inner(m, r, w).await
    }

    async fn notification_send(
        &self,
        app: &str,
        title: &str,
        body: &str,
        urgency: &str,
    ) -> anyhow::Result<u32> {
        self.notification_send_inner(app, title, body, urgency)
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
    async fn wifi_connect(&self, ssid: &str, pw: Option<&str>) -> anyhow::Result<()> {
        self.wifi_connect_inner(ssid, pw).await
    }

    async fn bluetooth_list(&self) -> anyhow::Result<Vec<protocol::BluetoothDeviceInfo>> {
        self.bluetooth_list_inner().await
    }
    async fn bluetooth_scan(&self, d: Option<u32>) -> anyhow::Result<()> {
        self.bluetooth_scan_inner(d).await
    }
    async fn bluetooth_stop_scan(&self) -> anyhow::Result<()> {
        self.bluetooth_stop_scan_inner().await
    }
    async fn bluetooth_connect(&self, addr: &str) -> anyhow::Result<()> {
        self.bluetooth_connect_inner(addr).await
    }
    async fn bluetooth_disconnect(&self, addr: &str) -> anyhow::Result<()> {
        self.bluetooth_disconnect_inner(addr).await
    }

    async fn files_watch(&self, p: &str, r: bool, pat: Option<&[String]>) -> anyhow::Result<()> {
        self.files_watch_inner(p, r, pat).await
    }
    async fn files_unwatch(&self, p: &str) -> anyhow::Result<()> {
        self.files_unwatch_inner(p).await
    }
    async fn files_search(
        &self,
        pat: &str,
        root: Option<&str>,
        max: u32,
    ) -> anyhow::Result<Vec<String>> {
        self.files_search_inner(pat, root, max).await
    }

    async fn audio_list_sinks(&self) -> anyhow::Result<Vec<protocol::AudioSinkInfo>> {
        self.audio_list_sinks_inner().await
    }
    async fn audio_set_sink_volume(&self, id: u32, vol: f64) -> anyhow::Result<()> {
        self.audio_set_sink_volume_inner(id, vol).await
    }

    async fn monitor_set_primary(&self, o: &str) -> anyhow::Result<()> {
        self.monitor_set_primary_inner(o).await
    }
    async fn monitor_set_resolution(
        &self,
        o: &str,
        w: u32,
        h: u32,
        r: Option<f64>,
    ) -> anyhow::Result<()> {
        self.monitor_set_resolution_inner(o, w, h, r).await
    }
    async fn monitor_set_scale(&self, o: &str, s: f64) -> anyhow::Result<()> {
        self.monitor_set_scale_inner(o, s).await
    }
    async fn monitor_set_rotation(&self, o: &str, r: &str) -> anyhow::Result<()> {
        self.monitor_set_rotation_inner(o, r).await
    }
    async fn monitor_set_enabled(&self, o: &str, e: bool) -> anyhow::Result<()> {
        self.monitor_set_enabled_inner(o, e).await
    }
}
