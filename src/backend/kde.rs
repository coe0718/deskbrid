use crate::backend::types::{MonitorInfo, WindowInfo};
use crate::backend::{DesktopBackend, InputBackend};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::Value;

pub struct KdeBackend;

impl KdeBackend {
    pub async fn new() -> Result<Self> {
        Ok(Self)
    }
}

#[async_trait]
impl DesktopBackend for KdeBackend {
    async fn list_windows(&self) -> Result<Vec<WindowInfo>> {
        Err(not_supported("window listing"))
    }

    async fn focus_window(
        &self,
        _app_id: Option<&str>,
        _title: Option<&str>,
        _exact: bool,
    ) -> Result<()> {
        Err(not_supported("window focusing"))
    }

    async fn focused_window(&self) -> Result<Option<WindowInfo>> {
        Err(not_supported("focused window lookup"))
    }

    async fn list_displays(&self) -> Result<Vec<MonitorInfo>> {
        Err(not_supported("display listing"))
    }

    async fn create_input_session(&self) -> Result<Box<dyn InputBackend>> {
        Err(not_supported("input injection"))
    }

    async fn send_notification(&self, _summary: &str, _body: &str, _urgency: &str) -> Result<u32> {
        Err(not_supported("notifications"))
    }

    fn desktop_name(&self) -> &'static str {
        "KDE"
    }

    fn capabilities(&self) -> &'static [&'static str] {
        &["screenshot"]
    }
}

pub struct KdeInputBackend;

#[async_trait]
impl InputBackend for KdeInputBackend {
    async fn type_text(&self, _text: &str) -> Result<()> {
        Err(not_supported("input injection"))
    }

    async fn send_keys(&self, _keys: &[String]) -> Result<()> {
        Err(not_supported("input injection"))
    }

    async fn mouse_action(&self, _params: &Value) -> Result<()> {
        Err(not_supported("input injection"))
    }
}

fn not_supported(capability: &str) -> anyhow::Error {
    anyhow!(
        "not_supported: KDE backend not yet implemented for {capability}; TODO use org.kde.KWin window APIs and org.freedesktop.portal.RemoteDesktop for input"
    )
}
