use serde::{Deserialize, Serialize};

// ─── Common Types ───────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct WindowInfo {
    pub id: String,
    pub title: String,
    pub app_id: String,
    pub workspace_id: u32,
    pub is_focused: bool,
    pub is_minimized: bool,
    pub geometry: Option<Geometry>,
    pub pid: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Geometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct WorkspaceInfo {
    pub id: u32,
    pub name: String,
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemInfo {
    pub desktop: String,
    pub desktop_version: String,
    pub compositor: String,
    pub session_type: String,
    pub monitors: Vec<MonitorInfo>,
    pub workspace_count: u32,
    pub current_workspace: u32,
    pub idle_seconds: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonitorInfo {
    pub id: u32,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
    pub primary: bool,
    #[serde(default = "default_monitor_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
    #[serde(default)]
    pub refresh_rate: Option<f64>,
    #[serde(default = "default_monitor_rotation")]
    pub rotation: String,
}

fn default_monitor_enabled() -> bool {
    true
}

fn default_monitor_rotation() -> String {
    "normal".into()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatteryInfo {
    pub source: String,
    pub percentage: f64,
    pub state: String,
    pub time_remaining_minutes: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct LayoutProfile {
    pub schema_version: u32,
    pub name: String,
    pub saved_at: u64,
    pub desktop: String,
    pub session_type: String,
    pub current_workspace: u32,
    pub monitors: Vec<MonitorInfo>,
    pub workspaces: Vec<WorkspaceInfo>,
    pub windows: Vec<WindowInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct LayoutProfileSummary {
    pub name: String,
    pub saved_at: u64,
    pub desktop: String,
    pub session_type: String,
    pub current_workspace: u32,
    pub monitor_count: usize,
    pub workspace_count: usize,
    pub window_count: usize,
}

// ─── Envelope ───────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Envelope {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorBody>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}
