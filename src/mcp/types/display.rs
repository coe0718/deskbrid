//! MCP parameter types for display operations: windows, workspaces, monitors, layouts.

use serde::Deserialize;

// ── Window ──────────────────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema)]
pub struct WindowId {
    #[schemars(description = "Window ID from list_windows")]
    pub window_id: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct MoveResize {
    #[schemars(description = "Window ID")]
    pub window_id: String,
    #[schemars(description = "X position")]
    pub x: i32,
    #[schemars(description = "Y position")]
    pub y: i32,
    #[schemars(description = "Width in pixels")]
    pub width: u32,
    #[schemars(description = "Height in pixels")]
    pub height: u32,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct TileWindow {
    #[schemars(description = "Window ID")]
    pub window_id: String,
    #[schemars(description = "Preset: 'left', 'right', 'maximize', or 'fullscreen'")]
    pub preset: String,
    #[schemars(description = "Monitor index")]
    pub monitor: Option<u32>,
    #[schemars(description = "Padding in pixels")]
    pub padding: Option<u32>,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct ActivateOrLaunch {
    #[schemars(description = "Application ID (e.g. 'firefox.desktop', 'code')")]
    pub app_id: String,
    #[schemars(description = "Launch command if app not running")]
    #[serde(default)]
    pub command: Vec<String>,
    #[schemars(description = "Working directory for launch")]
    pub workdir: Option<String>,
}

// ── Workspace ───────────────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct SwitchWorkspace {
    #[schemars(description = "Workspace index (0-based)")]
    pub workspace_id: u32,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct MoveWindowToWorkspace {
    #[schemars(description = "Window ID")]
    pub window_id: String,
    #[schemars(description = "Target workspace index (0-based)")]
    pub workspace_id: u32,
    #[schemars(description = "Follow window to target workspace")]
    #[serde(default)]
    pub follow: bool,
}

// ── Layout ─────────────────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct LayoutSave {
    #[schemars(description = "Layout profile name")]
    pub name: String,
    #[schemars(description = "Overwrite existing profile")]
    #[serde(default)]
    pub overwrite: bool,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct LayoutName {
    #[schemars(description = "Layout profile name")]
    pub name: String,
}

// ── Monitor ─────────────────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct MonitorOutput {
    #[schemars(description = "Monitor output name (e.g. 'DP-1', 'HDMI-1')")]
    pub output: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct SetResolution {
    #[schemars(description = "Monitor output name")]
    pub output: String,
    #[schemars(description = "Width in pixels")]
    pub width: u32,
    #[schemars(description = "Height in pixels")]
    pub height: u32,
    #[schemars(description = "Refresh rate in Hz")]
    pub refresh_rate: Option<f64>,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct SetScale {
    #[schemars(description = "Monitor output name")]
    pub output: String,
    #[schemars(description = "Scale factor (e.g. 1.0, 1.5, 2.0)")]
    pub scale: f64,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct SetRotation {
    #[schemars(description = "Monitor output name")]
    pub output: String,
    #[schemars(description = "Rotation: 'normal', 'left', 'right', 'inverted'")]
    pub rotation: String,
}
