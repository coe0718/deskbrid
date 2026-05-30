use anyhow::bail;
use clap::{Parser, Subcommand};

mod apps;
mod audio;
mod audit;
mod bluetooth;
mod clipboard;
mod color;
mod files;
mod input;
mod monitor;
mod mpris;
mod network;
mod notify;
mod portal;
mod system;
mod terminal;
mod windows;
mod workspace;

use apps::AppCmd;
use audio::AudioCmd;
use audit::AuditCmd;
use bluetooth::BluetoothCmd;
use clipboard::ClipboardCmd;
use color::ColorCmd;
use files::FilesCmd;
use input::{InputCmd, MouseCmd};
use monitor::MonitorCmd;
use mpris::MprisCmd;
use network::{NetworkCmd, WifiCmd};
use notify::NotifyCmd;
use portal::PortalCmd;
use system::{JournalCmd, ServiceCmd, SystemCmd, TimerCmd};
use terminal::TerminalCmd;
use windows::WindowCmd;
use workspace::{ProfileCmd, WorkspaceCmd};

#[derive(Subcommand)]
pub enum ScreencastCmd {
    /// Start recording the desktop to an MP4 file (GNOME only)
    Start {
        /// Output file path for the MP4 recording
        output_path: String,
    },
    /// Stop the running screencast recording
    Stop,
}

#[derive(Parser)]
#[command(
    name = "deskbrid",
    about = "The HAL your Linux desktop agents are missing",
    version = env!("CARGO_PKG_VERSION")
)]
pub struct Args {
    /// Validate permissions and show what would happen without executing the action.
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Override the action timeout in milliseconds for this request.
    #[arg(long, global = true)]
    pub timeout_ms: Option<u64>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start the deskbrid daemon
    Daemon {
        #[arg(long)]
        verbose: bool,

        #[arg(long)]
        mcp_port: Option<u16>,

        /// Disable the built-in web dashboard (port 20129)
        #[arg(long)]
        no_dashboard: bool,
    },

    /// Check if daemon is running
    Status,

    /// One-command setup: install GNOME Shell extension, enable it
    Setup,

    /// Start MCP (Model Context Protocol) stdio server for AI coding tools
    Mcp,

    // ─── Windows ────────────────────────────────────────
    #[command(name = "windows")]
    Windows {
        #[command(subcommand)]
        cmd: WindowCmd,
    },

    // ─── Workspaces ─────────────────────────────────────
    #[command(name = "workspaces")]
    Workspaces {
        #[command(subcommand)]
        cmd: WorkspaceCmd,
    },

    // ─── Layout Profiles ───────────────────────────────
    #[command(name = "profiles")]
    Profiles {
        #[command(subcommand)]
        cmd: ProfileCmd,
    },

    // ─── Input ──────────────────────────────────────────
    #[command(name = "input")]
    Input {
        #[command(subcommand)]
        cmd: InputCmd,
    },

    #[command(name = "combo")]
    Combo {
        /// Keys to press, separated by + (e.g. Control_L+c)
        keys: String,
    },

    // ─── Mouse ──────────────────────────────────────────
    #[command(name = "mouse")]
    Mouse {
        #[command(subcommand)]
        cmd: MouseCmd,
    },

    // ─── Clipboard ──────────────────────────────────────
    #[command(name = "clipboard")]
    Clipboard {
        #[command(subcommand)]
        cmd: ClipboardCmd,
    },

    // ─── Apps ───────────────────────────────────────────
    #[command(name = "apps")]
    Apps {
        #[command(subcommand)]
        cmd: AppCmd,
    },

    // ─── Media ──────────────────────────────────────────
    #[command(name = "mpris")]
    Mpris {
        #[command(subcommand)]
        cmd: MprisCmd,
    },

    // ─── Color ──────────────────────────────────────────
    #[command(name = "color")]
    Color {
        #[command(subcommand)]
        cmd: ColorCmd,
    },

    // ─── Screenshot ─────────────────────────────────────
    #[command(name = "screenshot")]
    Screenshot {
        /// Output file path (default: /tmp/deskbrid/screenshot_<ts>.png)
        #[arg(long, short)]
        output: Option<String>,

        /// Capture specific monitor index
        #[arg(long)]
        monitor: Option<u32>,

        /// Capture region: x y width height
        #[arg(long, num_args = 4)]
        region: Option<Vec<u32>>,

        /// Capture specific window
        #[arg(long)]
        window: Option<String>,
    },

    // ─── Screencast ─────────────────────────────────────
    #[command(name = "screencast")]
    Screencast {
        #[command(subcommand)]
        cmd: ScreencastCmd,
    },

    // ─── Desktop Portal ────────────────────────────────
    #[command(name = "portal")]
    Portal {
        #[command(subcommand)]
        cmd: PortalCmd,
    },

    // ─── OCR ───────────────────────────────────────────
    #[command(name = "ocr")]
    Ocr {
        /// OCR an existing screenshot path. Omit to capture a fresh screenshot.
        #[arg(long)]
        path: Option<String>,
        /// Tesseract language, e.g. eng or eng+spa
        #[arg(long)]
        language: Option<String>,
        /// Tesseract page segmentation mode
        #[arg(long)]
        psm: Option<u32>,
        /// Include word-level bounding boxes
        #[arg(long)]
        boxes: bool,
        /// Capture specific monitor index when path is omitted
        #[arg(long)]
        monitor: Option<u32>,
        /// Capture region when path is omitted: x y width height
        #[arg(long, num_args = 4)]
        region: Option<Vec<u32>>,
        /// Capture specific window when path is omitted
        #[arg(long)]
        window: Option<String>,
    },

    // ─── Screenshot diffing ────────────────────────────
    #[command(name = "screenshot-diff")]
    ScreenshotDiff {
        /// Baseline screenshot path
        before_path: String,
        /// Screenshot path to compare. Omit to capture a fresh screenshot.
        #[arg(long)]
        after_path: Option<String>,
        /// Per-channel pixel tolerance
        #[arg(long)]
        tolerance: Option<u8>,
        /// Save a red-highlight diff image to this path
        #[arg(long)]
        diff_path: Option<String>,
        /// Save a diff image to a generated /tmp/deskbrid path
        #[arg(long)]
        save_diff: bool,
        /// Capture specific monitor index when after_path is omitted
        #[arg(long)]
        monitor: Option<u32>,
        /// Capture region when after_path is omitted: x y width height
        #[arg(long, num_args = 4)]
        region: Option<Vec<u32>>,
        /// Capture specific window when after_path is omitted
        #[arg(long)]
        window: Option<String>,
    },

    // ─── Notifications ──────────────────────────────────
    #[command(name = "notify")]
    Notify {
        #[command(subcommand)]
        cmd: NotifyCmd,
    },

    // ─── System ─────────────────────────────────────────
    #[command(name = "system")]
    System {
        #[command(subcommand)]
        cmd: SystemCmd,
    },

    // ─── systemd services ──────────────────────────────
    #[command(name = "service")]
    Service {
        #[command(subcommand)]
        cmd: ServiceCmd,
    },

    // ─── journald ──────────────────────────────────────
    #[command(name = "journal")]
    Journal {
        #[command(subcommand)]
        cmd: JournalCmd,
    },

    // ─── systemd timers ────────────────────────────────
    #[command(name = "timer")]
    Timer {
        #[command(subcommand)]
        cmd: TimerCmd,
    },

    // ─── Terminal / PTY ────────────────────────────────
    #[command(name = "terminal")]
    Terminal {
        #[command(subcommand)]
        cmd: TerminalCmd,
    },

    // ─── Network ────────────────────────────────────────
    #[command(name = "network")]
    Network {
        #[command(subcommand)]
        cmd: NetworkCmd,
    },

    #[command(name = "wifi")]
    Wifi {
        #[command(subcommand)]
        cmd: WifiCmd,
    },

    // ─── Bluetooth ──────────────────────────────────────
    #[command(name = "bluetooth")]
    Bluetooth {
        #[command(subcommand)]
        cmd: BluetoothCmd,
    },

    // ─── Files ──────────────────────────────────────────
    #[command(name = "files")]
    Files {
        #[command(subcommand)]
        cmd: FilesCmd,
    },

    // ─── Audio ──────────────────────────────────────────
    #[command(name = "audio")]
    Audio {
        #[command(subcommand)]
        cmd: AudioCmd,
    },

    // ─── Monitors ───────────────────────────────────────
    #[command(name = "monitors")]
    Monitors {
        #[command(subcommand)]
        cmd: MonitorCmd,
    },

    // ─── Wait ───────────────────────────────────────────
    #[command(name = "wait")]
    Wait {
        /// Condition to wait for: window_exists, window_title, clipboard_contains, process_exits, file_exists, file_content, idle_seconds, screenshot_stable
        condition: String,
        /// Condition parameter as key=value. Repeat for multiple params.
        #[arg(long = "param")]
        params: Vec<String>,
        /// Timeout in milliseconds
        #[arg(long, default_value = "30000")]
        timeout_ms: u64,
        /// Poll interval in milliseconds
        #[arg(long)]
        interval_ms: Option<u64>,
    },

    // ─── Audit ──────────────────────────────────────────
    #[command(name = "audit")]
    Audit {
        #[command(subcommand)]
        cmd: AuditCmd,
    },

    // ─── Clients ────────────────────────────────────────
    #[command(name = "clients")]
    Clients,

    /// Check for updates and self-update the deskbrid binary
    #[command(name = "update")]
    Update {
        /// Only check for updates, don't download
        #[arg(long)]
        check: bool,
        /// Re-download even if same version
        #[arg(long)]
        force: bool,
    },

    /// Start the system tray icon (requires a running daemon)
    #[command(name = "tray")]
    Tray,

    /// Make a raw D-Bus method call (escape hatch for power users)
    #[command(name = "dbus-call")]
    DbusCall {
        /// D-Bus bus: 'session' (default) or 'system'
        #[arg(long, default_value = "session")]
        bus: String,
        /// D-Bus service name
        #[arg(long)]
        service: String,
        /// Object path
        #[arg(long)]
        path: String,
        /// Interface name
        #[arg(long)]
        interface: String,
        /// Method name
        #[arg(long)]
        method: String,
        /// Method arguments as JSON string
        #[arg(long)]
        args: Option<String>,
    },
}

pub fn parse() -> Args {
    Args::parse()
}

/// Translate CLI commands into protocol actions
pub(crate) mod into_action;

pub use into_action::into_action;
