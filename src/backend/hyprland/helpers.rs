use super::HyprBackend;
use crate::backend::DesktopBackend;
use crate::protocol;

impl HyprBackend {
    pub(super) async fn monitors_inner(&self) -> anyhow::Result<Vec<protocol::MonitorInfo>> {
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
                enabled: !m.get("disabled").and_then(|v| v.as_bool()).unwrap_or(false),
                x: m.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                y: m.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                refresh_rate: m.get("refreshRate").and_then(|v| v.as_f64()),
                rotation: super::free_functions::hypr_transform_to_rotation(
                    m.get("transform").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                )
                .to_string(),
            });
        }
        Ok(monitors)
    }

    pub(super) async fn monitor_config(
        &self,
        output: &str,
    ) -> anyhow::Result<super::free_functions::HyprMonitorConfig> {
        let json = match self.hyprctl_json(&["monitors", "all"]).await {
            Ok(json) => json,
            Err(_) => self.hyprctl_json(&["monitors"]).await?,
        };
        let arr = json
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("expected array"))?;
        let monitor = arr
            .iter()
            .find(|m| m.get("name").and_then(|v| v.as_str()) == Some(output))
            .ok_or_else(|| anyhow::anyhow!("monitor output not found: {}", output))?;

        Ok(super::free_functions::HyprMonitorConfig {
            name: output.to_string(),
            width: monitor
                .get("width")
                .and_then(|v| v.as_u64())
                .unwrap_or(1920) as u32,
            height: monitor
                .get("height")
                .and_then(|v| v.as_u64())
                .unwrap_or(1080) as u32,
            refresh_rate: monitor
                .get("refreshRate")
                .and_then(|v| v.as_f64())
                .unwrap_or(60.0),
            x: monitor.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            y: monitor.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            scale: monitor.get("scale").and_then(|v| v.as_f64()).unwrap_or(1.0),
            transform: monitor
                .get("transform")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32,
        })
    }

    pub(super) async fn apply_monitor_config(
        &self,
        config: &super::free_functions::HyprMonitorConfig,
    ) -> anyhow::Result<()> {
        let value = format!(
            "{},{}x{}@{},{}x{},{},transform,{}",
            config.name,
            config.width,
            config.height,
            super::free_functions::format_monitor_float(config.refresh_rate),
            config.x,
            config.y,
            super::free_functions::format_monitor_float(config.scale),
            config.transform
        );
        self.hyprctl_keyword("monitor", &value).await?;
        self.refresh_monitors_cache().await;
        Ok(())
    }

    pub(super) async fn refresh_monitors_cache(&self) {
        if let Ok(monitors) = self.monitors_inner().await
            && let Ok(mut m) = self.monitors.lock()
        {
            *m = monitors;
        }
    }

    pub(super) fn hyprctl_client_to_window(c: &serde_json::Value) -> protocol::WindowInfo {
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

    pub(super) async fn resolve_window(&self, id: &str) -> anyhow::Result<protocol::WindowInfo> {
        if id.trim().is_empty() {
            anyhow::bail!("window id must not be empty");
        }
        let windows = self.windows_list().await?;
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
            .ok_or_else(|| anyhow::anyhow!("no window matched id: {}", id))
    }

    pub(super) async fn window_is_fullscreen(&self, id: &str) -> anyhow::Result<bool> {
        let window = self.hyprctl_json(&["activewindow"]).await?;
        let active_id = window
            .get("address")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if !active_id.eq_ignore_ascii_case(id) {
            return Ok(false);
        }
        Ok(super::free_functions::json_truthy(window.get("fullscreen"))
            || super::free_functions::json_truthy(window.get("fullscreenClient"))
            || super::free_functions::json_truthy(window.get("fullscreenMode")))
    }

    pub(super) async fn idle_seconds_inner(&self) -> anyhow::Result<u64> {
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
