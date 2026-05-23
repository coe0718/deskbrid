//! Input parameter types for MCP tools, with JsonSchema derives for tool discovery.
//! Each type maps to one MCP tool's parameters.

use serde::Deserialize;

fn default_button() -> String {
    "left".into()
}

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

// ── Input ──────────────────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct TypeText {
    #[schemars(description = "Text to type")]
    pub text: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct PressKey {
    #[schemars(description = "Single key name (e.g. 'Return', 'Escape', 'Tab')")]
    pub key: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct PressKeys {
    #[schemars(description = "Keys to press (e.g. ['Control_L', 'c'])")]
    pub keys: Vec<String>,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct MouseMove {
    #[schemars(description = "X coordinate")]
    pub x: f64,
    #[schemars(description = "Y coordinate")]
    pub y: f64,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct MouseClick {
    #[schemars(description = "Button: 'left', 'middle', or 'right'")]
    #[serde(default = "default_button")]
    pub button: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct MouseScroll {
    #[schemars(description = "Horizontal scroll delta")]
    #[serde(default)]
    pub dx: f64,
    #[schemars(description = "Vertical scroll delta (negative = down)")]
    #[serde(default)]
    pub dy: f64,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct ClickCoord {
    #[schemars(description = "X coordinate")]
    pub x: f64,
    #[schemars(description = "Y coordinate")]
    pub y: f64,
    #[schemars(description = "Button")]
    #[serde(default = "default_button")]
    pub button: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct Drag {
    #[schemars(description = "Start X")]
    pub from_x: f64,
    #[schemars(description = "Start Y")]
    pub from_y: f64,
    #[schemars(description = "End X")]
    pub to_x: f64,
    #[schemars(description = "End Y")]
    pub to_y: f64,
    #[schemars(description = "Button")]
    #[serde(default = "default_button")]
    pub button: String,
}

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

// ── Audio ──────────────────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct SetVolume {
    #[schemars(description = "Sink ID from list_audio_sinks")]
    pub sink_id: u32,
    #[schemars(description = "Volume 0.0–1.0")]
    pub volume: f64,
}

// ── System ─────────────────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct ServiceName {
    #[schemars(description = "systemd unit name (e.g. 'nginx.service')")]
    pub name: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct JournalQuery {
    #[schemars(description = "Since timestamp (unix seconds)")]
    pub since: Option<u64>,
    #[schemars(description = "Until timestamp (unix seconds)")]
    pub until: Option<u64>,
    #[schemars(description = "Filter by unit name")]
    pub unit: Option<String>,
    #[schemars(description = "Max priority (0=emerg, 7=debug)")]
    pub priority: Option<u8>,
    #[schemars(description = "Number of recent entries")]
    pub tail: Option<u32>,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct BluetoothScan {
    #[schemars(description = "Scan duration in seconds (default: 10)")]
    pub duration: Option<u32>,
}

// ── Files ──────────────────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct FilePath {
    #[schemars(description = "File or directory path")]
    pub path: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct FileRead {
    #[schemars(description = "File path")]
    pub path: String,
    #[schemars(description = "Byte offset to start reading")]
    pub offset: Option<u64>,
    #[schemars(description = "Maximum bytes to read")]
    pub limit: Option<u64>,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct FileWrite {
    #[schemars(description = "File path")]
    pub path: String,
    #[schemars(description = "Content to write")]
    pub content: String,
    #[schemars(description = "Append instead of overwrite")]
    #[serde(default)]
    pub append: bool,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct FileSearch {
    #[schemars(description = "Search pattern (glob or regex)")]
    pub pattern: String,
    #[schemars(description = "Root directory to search (default: home)")]
    pub root: Option<String>,
    #[schemars(description = "Maximum results")]
    #[serde(default = "default_max_results")]
    pub max_results: u32,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct FileCopy {
    #[schemars(description = "Source path")]
    pub source: String,
    #[schemars(description = "Destination path")]
    pub destination: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct FileWatch {
    #[schemars(description = "Directory or file path to watch")]
    pub path: String,
    #[schemars(description = "Watch recursively")]
    #[serde(default)]
    pub recursive: bool,
    #[schemars(description = "File patterns to watch (e.g. ['*.rs'])")]
    pub patterns: Option<Vec<String>>,
}

fn default_max_results() -> u32 {
    50
}

// ── Terminal ───────────────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct TerminalCreate {
    #[schemars(description = "Shell to use (default: /bin/bash)")]
    pub shell: Option<String>,
    #[schemars(description = "Working directory")]
    pub cwd: Option<String>,
    #[schemars(description = "Terminal rows")]
    pub rows: Option<u16>,
    #[schemars(description = "Terminal columns")]
    pub cols: Option<u16>,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct TerminalWrite {
    #[schemars(description = "Terminal ID")]
    pub terminal_id: String,
    #[schemars(description = "Input to send (supports ANSI escape sequences)")]
    pub input: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct TerminalRead {
    #[schemars(description = "Terminal ID")]
    pub terminal_id: String,
    #[schemars(description = "Maximum bytes to read")]
    pub max_bytes: Option<u64>,
    #[schemars(description = "Flush output buffer before reading")]
    #[serde(default)]
    pub flush: bool,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct TerminalResize {
    #[schemars(description = "Terminal ID")]
    pub terminal_id: String,
    #[schemars(description = "Rows")]
    pub rows: u16,
    #[schemars(description = "Columns")]
    pub cols: u16,
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
