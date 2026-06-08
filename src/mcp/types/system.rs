//! MCP parameter types for system operations: audio, services, bluetooth, process, backlight, print, desktop settings, notifications.

use serde::Deserialize;

// ── Audio ──────────────────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct SetVolume {
    #[schemars(description = "Sink ID from list_audio_sinks")]
    pub sink_id: u32,
    #[schemars(description = "Volume 0.0–1.0")]
    pub volume: f64,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct AudioTargetParams {
    #[schemars(description = "\"sink\" for output, \"source\" for input")]
    #[serde(default = "default_audio_target")]
    pub target: String,
    #[schemars(description = "Device ID")]
    #[serde(default)]
    pub id: u32,
}

fn default_audio_target() -> String {
    "sink".to_string()
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct AudioVolumeParams {
    #[schemars(description = "\"sink\" for output, \"source\" for input")]
    #[serde(default = "default_audio_target")]
    pub target: String,
    #[schemars(description = "Device ID")]
    #[serde(default)]
    pub id: u32,
    #[schemars(description = "Volume 0.0–1.0")]
    pub volume: f64,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct AudioMuteParams {
    #[schemars(description = "\"sink\" for output, \"source\" for input")]
    #[serde(default = "default_audio_target")]
    pub target: String,
    #[schemars(description = "Device ID")]
    #[serde(default)]
    pub id: u32,
    #[schemars(description = "true to mute, false to unmute")]
    #[serde(default = "default_true")]
    pub mute: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct AudioDefaultParams {
    #[schemars(description = "\"sink\" for output, \"source\" for input")]
    #[serde(default = "default_audio_target")]
    pub target: String,
    #[schemars(description = "Device name (from list_audio_sinks/list_audio_sources)")]
    pub name: String,
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

// ── Process ────────────────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct ProcessStart {
    #[schemars(description = "Command and arguments")]
    pub command: Vec<String>,
    #[schemars(description = "Working directory")]
    pub workdir: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct ProcessPid {
    #[schemars(description = "Process ID")]
    pub pid: u32,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct ProcessSignal {
    #[schemars(description = "Process ID")]
    pub pid: u32,
    #[schemars(description = "Signal name (e.g. 'SIGTERM', 'SIGKILL')")]
    #[serde(default = "default_signal")]
    pub signal: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct ProcessWait {
    #[schemars(description = "Process ID")]
    pub pid: u32,
    #[schemars(description = "Timeout in milliseconds")]
    pub timeout_ms: Option<u64>,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct DbusCallArgs {
    #[schemars(description = "D-Bus bus: 'session' (default) or 'system'")]
    pub bus: Option<String>,
    #[schemars(description = "D-Bus service name (e.g. 'org.freedesktop.portal.Desktop')")]
    pub service: String,
    #[schemars(description = "Object path (e.g. '/org/freedesktop/portal/desktop')")]
    pub path: String,
    #[schemars(description = "Interface name (e.g. 'org.freedesktop.portal.Settings')")]
    pub interface: String,
    #[schemars(description = "Method name (e.g. 'Read')")]
    pub method: String,
    #[schemars(description = "Method arguments as JSON array or object")]
    pub args: Option<serde_json::Value>,
}

fn default_signal() -> String {
    "SIGTERM".into()
}

// ── Backlight ──────────────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct BacklightDevice {
    #[schemars(description = "Backlight device name (e.g. 'intel_backlight'). Omit for default.")]
    pub device: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct BacklightSetArgs {
    #[schemars(description = "Backlight device name (e.g. 'intel_backlight'). Omit for default.")]
    pub device: Option<String>,
    #[schemars(description = "Brightness value: percentage ('50%') or raw integer ('469')")]
    pub value: String,
}

// ── Print ──────────────────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct PrintDefaultArgs {
    #[schemars(description = "Printer name to set as default. Omit to just read current default.")]
    pub printer: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct PrintJobAction {
    #[schemars(description = "Print job ID (e.g. '42')")]
    pub job_id: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct PrintFileArgs {
    #[schemars(description = "Printer name (e.g. 'Canon_TS3500')")]
    pub printer: String,
    #[schemars(description = "Absolute path to the file to print")]
    pub path: String,
}

// ── Desktop Settings ───────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct DesktopSettingKey {
    #[schemars(description = "GSettings schema (e.g. 'org.gnome.desktop.interface')")]
    pub schema: String,
    #[schemars(description = "Schema key (e.g. 'gtk-theme')")]
    pub key: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct DesktopSettingValue {
    #[schemars(description = "GSettings schema (e.g. 'org.gnome.desktop.interface')")]
    pub schema: String,
    #[schemars(description = "Schema key (e.g. 'gtk-theme')")]
    pub key: String,
    #[schemars(description = "Value to set (string, boolean, or number)")]
    pub value: String,
}

// ── Notifications ──────────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct NotificationSend {
    #[schemars(description = "App name shown in notification")]
    pub app_name: String,
    #[schemars(description = "Notification title")]
    pub title: String,
    #[schemars(description = "Notification body text")]
    pub body: String,
    #[schemars(description = "Urgency: 'low', 'normal', or 'critical'")]
    #[serde(default = "default_urgency")]
    pub urgency: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct NotificationClose {
    #[schemars(description = "Notification ID to close")]
    pub notification_id: u32,
}

fn default_urgency() -> String {
    "normal".into()
}
