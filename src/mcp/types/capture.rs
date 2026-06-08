//! MCP parameter types for capture, media, accessibility, and browser operations.

use serde::Deserialize;

// ── Screenshot ─────────────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct ScreenshotOptions {
    #[schemars(description = "Monitor index")]
    pub monitor: Option<u32>,
    #[schemars(description = "Window ID to capture")]
    pub window_id: Option<String>,
    #[schemars(description = "Region x")]
    pub region_x: Option<i32>,
    #[schemars(description = "Region y")]
    pub region_y: Option<i32>,
    #[schemars(description = "Region width")]
    pub region_w: Option<u32>,
    #[schemars(description = "Region height")]
    pub region_h: Option<u32>,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct ScreenshotDiff {
    #[schemars(description = "Path to before screenshot")]
    pub before_path: String,
    #[schemars(description = "Path to after screenshot (takes new screenshot if omitted)")]
    pub after_path: Option<String>,
    #[schemars(description = "Pixel tolerance 0-255 (default: 10)")]
    pub tolerance: Option<u8>,
    #[schemars(description = "Save diff image to this path")]
    pub diff_path: Option<String>,
    #[schemars(description = "Monitor index")]
    pub monitor: Option<u32>,
}

// ── Clipboard ──────────────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct ClipboardWrite {
    #[schemars(description = "Text to copy to clipboard")]
    pub text: String,
}

// ── AT-SPI ─────────────────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct A11yTree {
    #[schemars(description = "Filter by app name")]
    pub app_name: Option<String>,
    #[schemars(description = "Filter by process ID")]
    pub pid: Option<u32>,
    #[schemars(description = "Maximum nodes (default: 200)")]
    pub max_nodes: Option<usize>,
    #[schemars(description = "Maximum depth (default: 10)")]
    pub max_depth: Option<u32>,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct A11yAction {
    #[schemars(description = "AT-SPI object reference path")]
    pub object_ref: String,
    #[schemars(description = "Action name (e.g. 'click', 'activate')")]
    pub action_name: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct SetValue {
    #[schemars(description = "AT-SPI object reference path")]
    pub object_ref: String,
    #[schemars(description = "Value to set")]
    pub value: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct GetText {
    #[schemars(description = "AT-SPI object reference path")]
    pub object_ref: String,
    #[schemars(description = "Maximum characters to return")]
    pub max_chars: Option<i32>,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct ClickElement {
    #[schemars(description = "AT-SPI object reference path")]
    pub object_ref: String,
}

// ── Browser (CDP) ──────────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct TabIndex {
    #[schemars(description = "Tab index from list_browser_tabs")]
    pub tab_index: Option<u32>,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct BrowserNavigate {
    #[schemars(description = "Tab index")]
    pub tab_index: Option<u32>,
    #[schemars(description = "URL to navigate to")]
    pub url: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct BrowserEvaluate {
    #[schemars(description = "Tab index")]
    pub tab_index: Option<u32>,
    #[schemars(description = "JavaScript expression to evaluate")]
    pub expression: String,
    #[schemars(description = "Wait for returned promise to resolve")]
    #[serde(default)]
    pub await_promise: bool,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct BrowserClick {
    #[schemars(description = "Tab index")]
    pub tab_index: Option<u32>,
    #[schemars(description = "CSS selector to click")]
    pub selector: String,
}

// ── MPRIS ──────────────────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct MprisPlayer {
    #[schemars(description = "Player bus name (optional, uses first available if omitted)")]
    pub player: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct MprisControl {
    #[schemars(description = "Player bus name")]
    pub player: Option<String>,
    #[schemars(description = "Action: 'play', 'pause', 'play_pause', 'next', 'previous', 'stop'")]
    pub action: String,
}

// ── Screencast ────────────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct ScreencastStartParams {
    #[schemars(description = "Output file path for the recording (e.g. /tmp/recording.mp4)")]
    pub output_path: String,
}

// ── Portal ────────────────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct PortalScreenshotParams {
    #[schemars(description = "Show interactive picker to select area/window")]
    #[serde(default)]
    pub interactive: bool,
}
