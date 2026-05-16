use crate::protocol;
use crate::protocol::{DeskbridEvent, Region};
use async_trait::async_trait;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::process::Command;
use tokio::sync::broadcast;

pub struct X11Backend {
    #[allow(dead_code)]
    event_tx: broadcast::Sender<DeskbridEvent>,
    #[allow(dead_code)]
    watchers: Arc<Mutex<HashMap<String, notify::RecommendedWatcher>>>,
}

impl X11Backend {
    pub async fn new(event_tx: broadcast::Sender<DeskbridEvent>) -> anyhow::Result<Self> {
        Ok(Self {
            event_tx,
            watchers: Arc::new(Mutex::new(HashMap::new())),
        })
    }
    async fn sh(&self, cmd: &str, args: &[&str]) -> anyhow::Result<String> {
        let out = Command::new(cmd)
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await?;
        if !out.status.success() {
            anyhow::bail!(
                "{} failed: {}",
                cmd,
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(String::from_utf8(out.stdout)?.trim().to_string())
    }
}

#[async_trait]
impl super::DesktopBackend for X11Backend {
    async fn windows_list(&self) -> anyhow::Result<Vec<protocol::WindowInfo>> {
        Ok(Vec::new())
    }
    async fn window_focus(&self, id: &str) -> anyhow::Result<()> {
        self.sh("xdotool", &["search", "--name", id, "windowactivate"])
            .await
            .map(|_| ())
    }
    async fn window_get(&self, id: &str) -> anyhow::Result<protocol::WindowInfo> {
        Ok(protocol::WindowInfo {
            id: id.to_string(),
            title: id.to_string(),
            app_id: String::new(),
            workspace_id: 0,
            is_focused: false,
            is_minimized: false,
            geometry: None,
            pid: None,
        })
    }
    async fn workspaces_list(&self) -> anyhow::Result<Vec<protocol::WorkspaceInfo>> {
        Ok(vec![protocol::WorkspaceInfo {
            id: 0,
            name: "Desktop 1".into(),
            is_active: true,
        }])
    }
    async fn workspace_switch(&self, id: u32) -> anyhow::Result<()> {
        self.sh("xdotool", &["set_desktop", &id.to_string()])
            .await
            .map(|_| ())
    }
    async fn workspace_move_window(
        &self,
        _window_id: &str,
        _workspace_id: u32,
        _follow: bool,
    ) -> anyhow::Result<()> {
        anyhow::bail!("not implemented on x11 backend")
    }
    async fn keyboard_type(&self, text: &str) -> anyhow::Result<()> {
        self.sh("xdotool", &["type", text]).await.map(|_| ())
    }
    async fn keyboard_key(&self, key: &str) -> anyhow::Result<()> {
        self.sh("xdotool", &["key", key]).await.map(|_| ())
    }
    async fn keyboard_combo(&self, keys: &[String]) -> anyhow::Result<()> {
        self.sh("xdotool", &["key", &keys.join("+")])
            .await
            .map(|_| ())
    }
    async fn mouse_move(&self, x: f64, y: f64) -> anyhow::Result<()> {
        self.sh(
            "xdotool",
            &[
                "mousemove",
                &(x as i32).to_string(),
                &(y as i32).to_string(),
            ],
        )
        .await
        .map(|_| ())
    }
    async fn mouse_click(&self, button: &str) -> anyhow::Result<()> {
        let b = match button {
            "left" => "1",
            "middle" => "2",
            "right" => "3",
            _ => "1",
        };
        self.sh("xdotool", &["click", b]).await.map(|_| ())
    }
    async fn mouse_scroll(&self, _dx: f64, dy: f64) -> anyhow::Result<()> {
        let b = if dy >= 0.0 { "4" } else { "5" };
        self.sh("xdotool", &["click", b]).await.map(|_| ())
    }
    async fn clipboard_read(&self) -> anyhow::Result<String> {
        self.sh("xclip", &["-o", "-selection", "clipboard"]).await
    }
    async fn clipboard_write(&self, text: &str) -> anyhow::Result<()> {
        self.sh(
            "sh",
            &[
                "-c",
                &format!(
                    "printf %s {} | xclip -selection clipboard",
                    shell_escape(text)
                ),
            ],
        )
        .await
        .map(|_| ())
    }
    async fn screenshot(
        &self,
        _monitor: Option<u32>,
        region: Option<Region>,
        _window_id: Option<String>,
    ) -> anyhow::Result<protocol::ScreenshotResult> {
        let path = format!(
            "/tmp/deskbrid_x11_{}.png",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs()
        );
        if let Some(r) = region {
            let geo = format!("{}x{}+{}+{}", r.width, r.height, r.x, r.y);
            self.sh("import", &["-window", "root", "-crop", &geo, &path])
                .await?;
            Ok(protocol::ScreenshotResult {
                path,
                width: r.width,
                height: r.height,
                format: "png".into(),
            })
        } else {
            self.sh("import", &["-window", "root", &path]).await?;
            Ok(protocol::ScreenshotResult {
                path,
                width: 0,
                height: 0,
                format: "png".into(),
            })
        }
    }
    async fn notification_send(
        &self,
        app_name: &str,
        title: &str,
        body: &str,
        urgency: &str,
    ) -> anyhow::Result<u32> {
        let _ = self
            .sh("notify-send", &["-a", app_name, "-u", urgency, title, body])
            .await;
        Ok(0)
    }
    async fn notification_close(&self, _id: u32) -> anyhow::Result<()> {
        Ok(())
    }
    async fn system_info(&self) -> anyhow::Result<protocol::SystemInfo> {
        Ok(protocol::SystemInfo {
            desktop: "X11".into(),
            desktop_version: "unknown".into(),
            compositor: "x11".into(),
            session_type: "x11".into(),
            monitors: vec![protocol::MonitorInfo {
                id: 0,
                name: "X11".into(),
                width: 1920,
                height: 1080,
                scale: 1.0,
                primary: true,
            }],
            workspace_count: 1,
            current_workspace: 0,
            idle_seconds: 0,
        })
    }
    async fn idle_seconds(&self) -> anyhow::Result<u64> {
        Ok(0)
    }
    async fn power_action(&self, _action: &str) -> anyhow::Result<()> {
        anyhow::bail!("not implemented on x11 backend")
    }
    async fn battery_status(&self) -> anyhow::Result<Vec<protocol::BatteryInfo>> {
        Ok(Vec::new())
    }
    async fn network_status(&self) -> anyhow::Result<protocol::NetworkStatusInfo> {
        Ok(protocol::NetworkStatusInfo {
            online: false,
            net_type: "unknown".into(),
        })
    }
    async fn network_interfaces(&self) -> anyhow::Result<Vec<protocol::NetworkInterfaceInfo>> {
        Ok(Vec::new())
    }
    async fn wifi_scan(&self) -> anyhow::Result<Vec<protocol::WifiNetworkInfo>> {
        Ok(Vec::new())
    }
    async fn wifi_connect(&self, _ssid: &str, _password: Option<&str>) -> anyhow::Result<()> {
        anyhow::bail!("not implemented on x11 backend")
    }
    async fn bluetooth_list(&self) -> anyhow::Result<Vec<protocol::BluetoothDeviceInfo>> {
        Ok(Vec::new())
    }
    async fn bluetooth_scan(&self, _duration: Option<u32>) -> anyhow::Result<()> {
        anyhow::bail!("not implemented on x11 backend")
    }
    async fn bluetooth_stop_scan(&self) -> anyhow::Result<()> {
        anyhow::bail!("not implemented on x11 backend")
    }
    async fn bluetooth_connect(&self, _address: &str) -> anyhow::Result<()> {
        anyhow::bail!("not implemented on x11 backend")
    }
    async fn bluetooth_disconnect(&self, _address: &str) -> anyhow::Result<()> {
        anyhow::bail!("not implemented on x11 backend")
    }
    async fn files_watch(
        &self,
        _path: &str,
        _recursive: bool,
        _patterns: Option<&[String]>,
    ) -> anyhow::Result<()> {
        anyhow::bail!("not implemented on x11 backend")
    }
    async fn files_unwatch(&self, _path: &str) -> anyhow::Result<()> {
        anyhow::bail!("not implemented on x11 backend")
    }
    async fn files_search(
        &self,
        _pattern: &str,
        _root: Option<&str>,
        _max_results: u32,
    ) -> anyhow::Result<Vec<String>> {
        Ok(Vec::new())
    }
    async fn audio_list_sinks(&self) -> anyhow::Result<Vec<protocol::AudioSinkInfo>> {
        Ok(Vec::new())
    }
    async fn audio_set_sink_volume(&self, _sink_id: u32, _volume: f64) -> anyhow::Result<()> {
        anyhow::bail!("not implemented on x11 backend")
    }
}

fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
