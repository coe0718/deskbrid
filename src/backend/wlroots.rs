use crate::backend::types::{MonitorInfo, WindowInfo};
use crate::backend::{DesktopBackend, InputBackend};
use crate::events::EventBus;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::Value;

pub struct WlrootsBackend;

impl WlrootsBackend {
    pub async fn new(_event_bus: EventBus) -> Result<Self> {
        // TODO: window tracking via wlr-foreign-toplevel-management-unstable-v1.
        // TODO: input injection via libei Emulated Input protocol.
        // TODO: clipboard integration via wlr-data-control-unstable-v1.
        // TODO: screenshots via wlr-screencopy-unstable-v1.
        // TODO: display listing via wlr-output-management-unstable-v1.
        Ok(Self)
    }
}

#[async_trait]
impl DesktopBackend for WlrootsBackend {
    async fn list_windows(&self) -> Result<Vec<WindowInfo>> {
        Err(not_supported(
            "window tracking",
            "TODO implement wlr-foreign-toplevel-management-unstable-v1",
        ))
    }

    async fn focus_window(
        &self,
        _app_id: Option<&str>,
        _title: Option<&str>,
        _exact: bool,
    ) -> Result<()> {
        Err(not_supported(
            "window focusing",
            "TODO implement wlr-foreign-toplevel-management-unstable-v1",
        ))
    }

    async fn focused_window(&self) -> Result<Option<WindowInfo>> {
        Err(not_supported(
            "focused window lookup",
            "TODO implement wlr-foreign-toplevel-management-unstable-v1",
        ))
    }

    async fn list_displays(&self) -> Result<Vec<MonitorInfo>> {
        Err(not_supported(
            "display listing",
            "TODO implement wlr-output-management-unstable-v1",
        ))
    }

    async fn create_input_session(&self) -> Result<Box<dyn InputBackend>> {
        Err(not_supported(
            "input injection",
            "TODO implement libei Emulated Input protocol",
        ))
    }

    async fn send_notification(&self, _summary: &str, _body: &str, _urgency: &str) -> Result<u32> {
        Err(not_supported(
            "notifications",
            "wlroots compositors do not expose a common notification API here",
        ))
    }

    fn desktop_name(&self) -> &'static str {
        "wlroots"
    }

    fn capabilities(&self) -> &'static [&'static str] {
        &["screenshot"]
    }
}

pub struct WlrootsInputBackend;

#[async_trait]
impl InputBackend for WlrootsInputBackend {
    async fn type_text(&self, _text: &str) -> Result<()> {
        Err(not_supported(
            "input injection",
            "TODO implement libei Emulated Input protocol",
        ))
    }

    async fn send_keys(&self, _keys: &[String]) -> Result<()> {
        Err(not_supported(
            "input injection",
            "TODO implement libei Emulated Input protocol",
        ))
    }

    async fn mouse_action(&self, _params: &Value) -> Result<()> {
        Err(not_supported(
            "input injection",
            "TODO implement libei Emulated Input protocol",
        ))
    }
}

fn not_supported(capability: &str, todo: &str) -> anyhow::Error {
    anyhow!(
        "not_supported: wlroots backend currently supports clipboard via wlr-data-control-unstable-v1 and screenshots via wlr-screencopy-unstable-v1 only; {capability} unavailable; {todo}"
    )
}
