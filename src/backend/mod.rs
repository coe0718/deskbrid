#[cfg(feature = "pipewire")]
pub mod audio;
pub mod detect;
pub mod gnome;
pub mod kde;
#[cfg(feature = "pipewire")]
pub mod screencast;
pub mod types;
pub mod wlroots;

use crate::events::EventBus;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::watch;
pub use types::{MonitorInfo, WindowInfo};

use self::detect::{detect_desktop, DesktopType};
use self::gnome::GnomeBackend;
use self::wlroots::WlrootsBackend;

#[cfg(not(feature = "pipewire"))]
pub mod audio {
    use crate::events::EventBus;
    use tokio::sync::watch;

    pub fn spawn_audio_monitor(_event_bus: EventBus, _shutdown: watch::Receiver<bool>) {}
}

#[cfg(not(feature = "pipewire"))]
pub mod screencast {
    use crate::capture;
    use anyhow::{anyhow, Result};
    use zbus::zvariant::OwnedObjectPath;

    #[derive(Clone, Debug)]
    pub struct ScreencastSession {
        pub session_path: OwnedObjectPath,
    }

    #[derive(Debug)]
    pub struct ScreenshotResult {
        pub path: String,
        pub width: u32,
        pub height: u32,
    }

    #[derive(Debug)]
    pub struct StartedScreencast {
        pub node_id: u32,
        pub width: Option<u32>,
        pub height: Option<u32>,
        pub session: ScreencastSession,
    }

    pub async fn screenshot(
        _conn: &zbus::Connection,
        _monitor_connector: &str,
    ) -> Result<ScreenshotResult> {
        let path = capture::fallback_screenshot(None).await?;
        Ok(ScreenshotResult {
            path,
            width: 0,
            height: 0,
        })
    }

    pub async fn start_screencast(
        _conn: &zbus::Connection,
        _monitor_connector: &str,
        _framerate: u32,
    ) -> Result<StartedScreencast> {
        Err(anyhow!(
            "not_supported: screencast capability unavailable; build with --features pipewire and install libpipewire-0.3 development files"
        ))
    }

    pub async fn stop_screencast(
        _conn: &zbus::Connection,
        _session: &ScreencastSession,
    ) -> Result<()> {
        Err(anyhow!(
            "not_supported: screencast capability unavailable; build with --features pipewire and install libpipewire-0.3 development files"
        ))
    }
}

#[async_trait]
pub trait DesktopBackend: Send + Sync {
    async fn list_windows(&self) -> Result<Vec<WindowInfo>>;
    async fn focus_window(
        &self,
        app_id: Option<&str>,
        title: Option<&str>,
        exact: bool,
    ) -> Result<()>;
    async fn focused_window(&self) -> Result<Option<WindowInfo>>;
    async fn list_displays(&self) -> Result<Vec<MonitorInfo>>;
    async fn create_input_session(&self) -> Result<Box<dyn InputBackend>>;
    async fn send_notification(&self, summary: &str, body: &str, urgency: &str) -> Result<u32>;
    async fn screenshot(&self, monitor: Option<u32>) -> Result<Value> {
        let _ = monitor;
        Err(anyhow!("not_supported: screenshot capability unavailable"))
    }
    async fn start_screencast(&self, monitor: u32, framerate: u32) -> Result<Value> {
        let _ = (monitor, framerate);
        Err(anyhow!("not_supported: screencast capability unavailable"))
    }
    async fn stop_screencast(&self, node_id: u32) -> Result<()> {
        let _ = node_id;
        Err(anyhow!("not_supported: screencast capability unavailable"))
    }
    fn desktop_name(&self) -> &'static str;
    fn capabilities(&self) -> &'static [&'static str];
}

#[async_trait]
pub trait InputBackend: Send + Sync {
    async fn type_text(&self, text: &str) -> Result<()>;
    async fn send_keys(&self, keys: &[String]) -> Result<()>;
    async fn mouse_action(&self, params: &Value) -> Result<()>;
}

pub async fn create_backend(
    event_bus: EventBus,
    shutdown: watch::Receiver<bool>,
) -> Result<Box<dyn DesktopBackend>> {
    match detect_desktop() {
        DesktopType::Gnome => Ok(Box::new(GnomeBackend::new(event_bus, shutdown).await?)),
        DesktopType::Kde => Err(anyhow!("KDE backend not yet implemented")),
        DesktopType::Wlroots => Ok(Box::new(WlrootsBackend::new(event_bus).await?)),
        DesktopType::Other => Err(anyhow!("unsupported desktop environment")),
    }
}
