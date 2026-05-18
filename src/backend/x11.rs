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

    async fn sh_owned(&self, cmd: &str, args: Vec<String>) -> anyhow::Result<String> {
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        self.sh(cmd, &refs).await
    }

    fn ensure_window_id(id: &str) -> anyhow::Result<()> {
        if id.trim().is_empty() {
            anyhow::bail!("window id must not be empty");
        }
        Ok(())
    }

    async fn xrandr_monitors(&self) -> anyhow::Result<Vec<protocol::MonitorInfo>> {
        let out = self.sh("xrandr", &["--query"]).await?;
        Ok(parse_xrandr_query(&out))
    }
}

#[async_trait]
impl super::DesktopBackend for X11Backend {
    async fn windows_list(&self) -> anyhow::Result<Vec<protocol::WindowInfo>> {
        Ok(Vec::new())
    }
    async fn window_focus(&self, id: &str) -> anyhow::Result<()> {
        Self::ensure_window_id(id)?;
        self.sh("xdotool", &["search", "--name", id, "windowactivate"])
            .await
            .map(|_| ())
    }
    async fn window_get(&self, id: &str) -> anyhow::Result<protocol::WindowInfo> {
        Self::ensure_window_id(id)?;
        // xdotool getwindowname verifies existence AND returns the real window title
        let title = self
            .sh("xdotool", &["getwindowname", id])
            .await
            .map_err(|_| anyhow::anyhow!("window not found: {}", id))?;

        Ok(protocol::WindowInfo {
            id: id.to_string(),
            title,
            app_id: String::new(),
            workspace_id: 0,
            is_focused: false,
            is_minimized: false,
            geometry: None,
            pid: None,
        })
    }
    async fn window_close(&self, id: &str) -> anyhow::Result<()> {
        Self::ensure_window_id(id)?;
        self.sh("xdotool", &["windowclose", id]).await.map(|_| ())
    }
    async fn window_minimize(&self, id: &str) -> anyhow::Result<()> {
        Self::ensure_window_id(id)?;
        self.sh("xdotool", &["windowminimize", id])
            .await
            .map(|_| ())
    }
    async fn window_maximize(&self, id: &str) -> anyhow::Result<()> {
        Self::ensure_window_id(id)?;
        self.sh(
            "wmctrl",
            &["-ir", id, "-b", "add,maximized_vert,maximized_horz"],
        )
        .await
        .map(|_| ())
    }
    async fn window_move_resize(
        &self,
        id: &str,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> anyhow::Result<()> {
        Self::ensure_window_id(id)?;
        self.sh(
            "xdotool",
            &["windowmove", id, &x.to_string(), &y.to_string()],
        )
        .await?;
        self.sh(
            "xdotool",
            &["windowsize", id, &width.to_string(), &height.to_string()],
        )
        .await
        .map(|_| ())
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
            // Read back real dimensions from the captured PNG
            let dims = self
                .sh("identify", &["-format", "%w %h", &path])
                .await
                .unwrap_or_else(|_| "0 0".into());
            let mut parts = dims.split_whitespace();
            let w: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            let h: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            Ok(protocol::ScreenshotResult {
                path,
                width: w,
                height: h,
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
        self.sh("notify-send", &["-a", app_name, "-u", urgency, title, body])
            .await?;
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
            monitors: self.xrandr_monitors().await.unwrap_or_else(|_| {
                vec![protocol::MonitorInfo {
                    id: 0,
                    name: "X11".into(),
                    width: 1920,
                    height: 1080,
                    scale: 1.0,
                    primary: true,
                    enabled: true,
                    x: 0,
                    y: 0,
                    refresh_rate: None,
                    rotation: "normal".into(),
                }]
            }),
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

    async fn monitor_set_primary(&self, output: &str) -> anyhow::Result<()> {
        self.sh("xrandr", &["--output", output, "--primary"])
            .await
            .map(|_| ())
    }

    async fn monitor_set_resolution(
        &self,
        output: &str,
        width: u32,
        height: u32,
        refresh_rate: Option<f64>,
    ) -> anyhow::Result<()> {
        let mut args = vec![
            "--output".to_string(),
            output.to_string(),
            "--mode".into(),
            format!("{}x{}", width, height),
        ];
        if let Some(refresh) = refresh_rate {
            args.push("--rate".into());
            args.push(format_monitor_float(refresh));
        }
        self.sh_owned("xrandr", args).await.map(|_| ())
    }

    async fn monitor_set_scale(&self, output: &str, scale: f64) -> anyhow::Result<()> {
        let scale_arg = format!("{0}x{0}", format_monitor_float(1.0 / scale));
        self.sh_owned(
            "xrandr",
            vec![
                "--output".into(),
                output.into(),
                "--scale".into(),
                scale_arg,
            ],
        )
        .await
        .map(|_| ())
    }

    async fn monitor_set_rotation(&self, output: &str, rotation: &str) -> anyhow::Result<()> {
        self.sh(
            "xrandr",
            &["--output", output, "--rotate", xrandr_rotation(rotation)?],
        )
        .await
        .map(|_| ())
    }

    async fn monitor_set_enabled(&self, output: &str, enabled: bool) -> anyhow::Result<()> {
        self.sh(
            "xrandr",
            &["--output", output, if enabled { "--auto" } else { "--off" }],
        )
        .await
        .map(|_| ())
    }
}

fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn parse_xrandr_query(raw: &str) -> Vec<protocol::MonitorInfo> {
    let mut monitors = Vec::new();
    let mut current: Option<protocol::MonitorInfo> = None;

    for line in raw.lines() {
        if !line.starts_with(' ') && line.contains(" connected") {
            if let Some(monitor) = current.take() {
                monitors.push(monitor);
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            let geometry = parts
                .iter()
                .find(|part| part.contains('+') && part.contains('x'));
            let mut monitor = protocol::MonitorInfo {
                id: monitors.len() as u32,
                name: parts.first().copied().unwrap_or("").to_string(),
                width: 0,
                height: 0,
                scale: 1.0,
                primary: parts.contains(&"primary"),
                enabled: geometry.is_some(),
                x: 0,
                y: 0,
                refresh_rate: None,
                rotation: parse_xrandr_rotation(line),
            };
            if let Some(geometry) = geometry {
                parse_xrandr_geometry(geometry, &mut monitor);
            }
            current = Some(monitor);
            continue;
        }

        let Some(ref mut monitor) = current else {
            continue;
        };
        let trimmed = line.trim();
        if trimmed.contains('*') {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if let Some(refresh) = parts.iter().find(|part| part.contains('*')) {
                monitor.refresh_rate = refresh.trim_end_matches(['*', '+']).parse().ok();
            }
        }
    }

    if let Some(monitor) = current.take() {
        monitors.push(monitor);
    }
    monitors
}

fn parse_xrandr_geometry(value: &str, monitor: &mut protocol::MonitorInfo) {
    // XRandR geometry: WIDTHxHEIGHT[+-]X[+-]Y — offsets can be negative
    // e.g. "1920x1080-1920+0" for a display left of origin
    if let Some(x_pos) = value.find('x') {
        monitor.width = value[..x_pos].parse().unwrap_or(0);

        // Find the start of the X offset (first + or - after 'x')
        let rest = &value[x_pos + 1..];
        let offset_start = rest.find(['+', '-']).unwrap_or(rest.len());
        monitor.height = rest[..offset_start].parse().unwrap_or(0);

        // Parse the offsets with their signs
        let offset_str = &rest[offset_start..];
        let mut parts = offset_str.split('+');
        let x_part = parts.next().unwrap_or("");
        // x_part starts with a sign, so parse() handles it directly
        monitor.x = x_part.parse().unwrap_or(0);
        // Remaining parts are positive offsets
        for (i, part) in parts.enumerate() {
            if i == 0 {
                monitor.y = part.parse().unwrap_or(0);
            }
        }
    }
}

fn parse_xrandr_rotation(line: &str) -> String {
    let rotations = ["normal", "left", "right", "inverted"];
    let active_segment = line.split('(').next().unwrap_or(line);
    let tokens: Vec<&str> = active_segment.split_whitespace().collect();

    if let Some(geometry_idx) = tokens
        .iter()
        .position(|part| part.contains('+') && part.contains('x'))
        && let Some(candidate) = tokens.get(geometry_idx + 1)
        && rotations.contains(candidate)
    {
        return (*candidate).to_string();
    }

    for token in tokens {
        if rotations.contains(&token) {
            return token.to_string();
        }
    }
    "normal".into()
}

fn xrandr_rotation(rotation: &str) -> anyhow::Result<&'static str> {
    match rotation {
        "normal" => Ok("normal"),
        "left" => Ok("left"),
        "right" => Ok("right"),
        "inverted" => Ok("inverted"),
        _ => anyhow::bail!("unsupported monitor rotation: {}", rotation),
    }
}

fn format_monitor_float(value: f64) -> String {
    let mut out = format!("{:.3}", value);
    while out.contains('.') && out.ends_with('0') {
        out.pop();
    }
    if out.ends_with('.') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_active_xrandr_rotation_before_capability_list() {
        assert_eq!(
            parse_xrandr_rotation(
                "DP-1 connected primary 2560x1440+0+0 right (normal left inverted right x axis y axis)"
            ),
            "right"
        );
        assert_eq!(
            parse_xrandr_rotation(
                "HDMI-1 connected 1080x1920+2560+0 inverted (normal left inverted right x axis y axis)"
            ),
            "inverted"
        );
        assert_eq!(
            parse_xrandr_rotation(
                "eDP-1 connected 1920x1080+0+0 (normal left inverted right x axis y axis)"
            ),
            "normal"
        );
    }
}
