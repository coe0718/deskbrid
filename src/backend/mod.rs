pub(crate) mod backlight;
pub mod cosmic;
pub mod gnome;
pub(crate) mod gsettings_desktop;
pub mod hyprland;
pub mod kde;
pub mod labwc;
pub mod niri;
pub mod sway;
pub mod wayfire;
pub(crate) mod wlr_randr;
pub mod x11;

use crate::color::{rgba_to_hex, sample_pixel};
use crate::protocol;
use async_trait::async_trait;

/// Auto-detect the current desktop environment and create the matching backend.
pub async fn create_backend(
    event_tx: tokio::sync::broadcast::Sender<crate::protocol::DeskbridEvent>,
) -> anyhow::Result<Box<dyn DesktopBackend>> {
    let desktop = detect_desktop().await;

    match desktop {
        DesktopEnv::Cosmic => cosmic::CosmicBackend::new(event_tx)
            .await
            .map(|b| Box::new(b) as Box<dyn DesktopBackend>),
        DesktopEnv::Hyprland => hyprland::HyprBackend::new(event_tx)
            .await
            .map(|b| Box::new(b) as Box<dyn DesktopBackend>),
        DesktopEnv::Kde => kde::KdeBackend::new(event_tx)
            .await
            .map(|b| Box::new(b) as Box<dyn DesktopBackend>),
        DesktopEnv::X11 => {
            let xdg = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "X11".into());
            x11::X11Backend::new(event_tx, xdg)
                .await
                .map(|b| Box::new(b) as Box<dyn DesktopBackend>)
        }
        DesktopEnv::Sway => sway::SwayBackend::new(event_tx)
            .await
            .map(|b| Box::new(b) as Box<dyn DesktopBackend>),
        DesktopEnv::Niri => niri::NiriBackend::new(event_tx)
            .await
            .map(|b| Box::new(b) as Box<dyn DesktopBackend>),
        DesktopEnv::Labwc => labwc::LabwcBackend::new(event_tx)
            .await
            .map(|b| Box::new(b) as Box<dyn DesktopBackend>),
        DesktopEnv::Wayfire => wayfire::WayfireBackend::new(event_tx)
            .await
            .map(|b| Box::new(b) as Box<dyn DesktopBackend>),
        // GNOME is the fallback/default
        _ => gnome::GnomeBackend::new(event_tx)
            .await
            .map(|b| Box::new(b) as Box<dyn DesktopBackend>),
    }
}

/// Detect which desktop environment is running.
async fn detect_desktop() -> DesktopEnv {
    // 1. Check XDG_CURRENT_DESKTOP env var
    if let Ok(desktop) = std::env::var("XDG_CURRENT_DESKTOP") {
        let lower = desktop.to_lowercase();
        if lower.contains("hyprland") {
            return DesktopEnv::Hyprland;
        }
        if lower.contains("sway") {
            return DesktopEnv::Sway;
        }
        if lower.contains("niri") {
            return DesktopEnv::Niri;
        }
        if lower.contains("wayfire") {
            return DesktopEnv::Wayfire;
        }
        if lower.contains("labwc") {
            return DesktopEnv::Labwc;
        }
        if lower.contains("cosmic") {
            return DesktopEnv::Cosmic;
        }
        if lower.contains("kde") || lower.contains("plasma") {
            return DesktopEnv::Kde;
        }
        if lower.contains("gnome") {
            return DesktopEnv::Gnome;
        }
        if lower.contains("x11")
            || lower.contains("xfce")
            || lower.contains("mate")
            || lower.contains("cinnamon")
            || lower.contains("enlightenment")
        {
            return DesktopEnv::X11;
        }
    }

    // 2. Check running compositor processes
    for (process, desktop_env) in [
        ("Hyprland", DesktopEnv::Hyprland),
        ("sway", DesktopEnv::Sway),
        ("niri", DesktopEnv::Niri),
        ("wayfire", DesktopEnv::Wayfire),
        ("labwc", DesktopEnv::Labwc),
        ("kwin_wayland", DesktopEnv::Kde),
        ("cosmic-comp", DesktopEnv::Cosmic),
        ("enlightenment", DesktopEnv::X11),
    ] {
        if process_running(process).await {
            return desktop_env;
        }
    }

    // Default to GNOME
    if std::env::var("DISPLAY").is_ok() && std::env::var("WAYLAND_DISPLAY").is_err() {
        return DesktopEnv::X11;
    }
    DesktopEnv::Gnome
}

async fn process_running(name: &str) -> bool {
    tokio::process::Command::new("pgrep")
        .args(["-x", name])
        .output()
        .await
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[derive(Clone, Copy)]
enum DesktopEnv {
    Cosmic,
    #[allow(dead_code)]
    Enlightenment,
    Gnome,
    Hyprland,
    Kde,
    Niri,
    Sway,
    Wayfire,
    Labwc,
    X11,
}

/// The DesktopBackend trait defines all actions deskbrid can perform on a desktop
/// environment. Supported backends: GNOME (Mutter DBus), Hyprland (hyprctl).
#[async_trait]
pub trait DesktopBackend: Send + Sync {
    // ─── Color ───────────────────────────────────────────
    /// Pick a pixel color at screen coordinates (x, y).
    /// Default: screenshot a 1x1 region and sample the pixel.
    /// Backends with native color pickers (e.g. hyprpicker) should override this.
    async fn pick_color(&self, x: u32, y: u32) -> anyhow::Result<serde_json::Value> {
        let screenshot = self
            .screenshot(
                None,
                Some(protocol::Region {
                    x,
                    y,
                    width: 1,
                    height: 1,
                }),
                None,
            )
            .await?;
        let pixel = tokio::task::spawn_blocking({
            let path = screenshot.path.clone();
            move || sample_pixel(&path, 0, 0)
        })
        .await??;
        Ok(serde_json::json!({
            "x": x,
            "y": y,
            "source_path": screenshot.path,
            "red": pixel[0],
            "green": pixel[1],
            "blue": pixel[2],
            "alpha": pixel[3],
            "hex": rgba_to_hex(pixel)
        }))
    }

    // ─── Windows ────────────────────────────────────────
    async fn windows_list(&self) -> anyhow::Result<Vec<protocol::WindowInfo>>;
    async fn window_focus(&self, id: &str) -> anyhow::Result<()>;
    async fn window_get(&self, id: &str) -> anyhow::Result<protocol::WindowInfo>;
    async fn window_close(&self, id: &str) -> anyhow::Result<()>;
    async fn window_minimize(&self, id: &str) -> anyhow::Result<()>;
    async fn window_maximize(&self, id: &str) -> anyhow::Result<()>;
    async fn window_move_resize(
        &self,
        id: &str,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> anyhow::Result<()>;

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
    // ─── Keyboard Layout ────────────────────────────────
    async fn keyboard_layout_list(&self) -> anyhow::Result<Vec<protocol::KeyboardLayout>> {
        anyhow::bail!("keyboard layout list not supported by this backend")
    }
    async fn keyboard_layout_get(&self) -> anyhow::Result<protocol::KeyboardLayout> {
        anyhow::bail!("keyboard layout get not supported by this backend")
    }
    async fn keyboard_layout_set(
        &self,
        _index: Option<u32>,
        _name: Option<&str>,
        _variant: Option<&str>,
    ) -> anyhow::Result<()> {
        anyhow::bail!("keyboard layout set not supported by this backend")
    }
    async fn keyboard_layout_add(&self, _name: &str, _variant: Option<&str>) -> anyhow::Result<()> {
        anyhow::bail!("keyboard layout add not supported by this backend")
    }
    async fn keyboard_layout_remove(&self, _index: u32) -> anyhow::Result<()> {
        anyhow::bail!("keyboard layout remove not supported by this backend")
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
        let _ = (from_x, from_y, to_x, to_y, button, duration_ms);
        anyhow::bail!("mouse drag is not supported by this backend")
    }

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

    // ─── Monitor ────────────────────────────────────────
    async fn monitor_set_primary(&self, output: &str) -> anyhow::Result<()>;
    async fn monitor_set_resolution(
        &self,
        output: &str,
        width: u32,
        height: u32,
        refresh_rate: Option<f64>,
    ) -> anyhow::Result<()>;
    async fn monitor_set_scale(&self, output: &str, scale: f64) -> anyhow::Result<()>;
    async fn monitor_set_rotation(&self, output: &str, rotation: &str) -> anyhow::Result<()>;
    async fn monitor_set_enabled(&self, output: &str, enabled: bool) -> anyhow::Result<()>;

    // ─── Screencast ─────────────────────────────────────
    /// Start recording the desktop to an MP4 file (GNOME only).
    async fn start_screencast(&self, output_path: &str) -> anyhow::Result<()> {
        let _ = output_path;
        anyhow::bail!("screencast is not supported by this backend")
    }
    /// Stop the running screencast recording (GNOME only).
    async fn stop_screencast(&self) -> anyhow::Result<()> {
        anyhow::bail!("screencast is not supported by this backend")
    }

    // ─── Desktop Settings ──────────────────────────────
    /// Read a desktop environment setting by schema and key.
    async fn desktop_get_setting(&self, _schema: &str, _key: &str) -> anyhow::Result<String> {
        anyhow::bail!("desktop settings not supported by this backend")
    }
    /// Write a desktop environment setting.
    async fn desktop_set_setting(
        &self,
        _schema: &str,
        _key: &str,
        _value: &str,
    ) -> anyhow::Result<()> {
        anyhow::bail!("desktop settings not supported by this backend")
    }
    /// List available setting schemas.
    async fn desktop_list_schemas(&self) -> anyhow::Result<Vec<String>> {
        anyhow::bail!("desktop settings not supported by this backend")
    }

    /// List backlight devices (sysfs — works everywhere).
    async fn backlight_list(&self) -> anyhow::Result<Vec<protocol::BacklightInfo>> {
        backlight::backlight_list()
    }
    /// Get brightness for a device.
    async fn backlight_get(&self, device: Option<&str>) -> anyhow::Result<protocol::BacklightInfo> {
        backlight::backlight_get(device)
    }
    /// Set brightness (absolute or "50%").
    async fn backlight_set(
        &self,
        device: Option<&str>,
        value: &str,
    ) -> anyhow::Result<protocol::BacklightInfo> {
        backlight::backlight_set(device, value)
    }
}
