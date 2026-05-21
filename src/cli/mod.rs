use anyhow::bail;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "deskbrid",
    about = "The HAL your Linux desktop agents are missing",
    version = "0.4.1"
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start the deskbrid daemon
    Daemon {
        #[arg(long)]
        verbose: bool,
    },

    /// Check if daemon is running
    Status,

    /// One-command setup: install GNOME Shell extension, enable it
    Setup,

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

    // ─── Clients ────────────────────────────────────────
    #[command(name = "clients")]
    Clients,
}

#[derive(Subcommand)]
pub enum WindowCmd {
    /// List all windows
    List,
    /// Focus a window
    Focus { window_id: String },
    /// Get window details
    Get { window_id: String },
    /// Close a window
    Close { window_id: String },
    /// Minimize a window
    Minimize { window_id: String },
    /// Maximize a window
    Maximize { window_id: String },
    /// Move and resize a window
    MoveResize {
        window_id: String,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    },
    /// Focus an app if open, launch it if not
    ActivateOrLaunch {
        app_id: String,
        /// Command to launch when no matching window exists. Defaults to app_id.
        #[arg(last = true)]
        command: Vec<String>,
    },
}

#[derive(Subcommand)]
pub enum WorkspaceCmd {
    /// List workspaces
    List,
    /// Switch to a workspace
    Switch { workspace_id: u32 },
    /// Move window to workspace
    Move {
        window_id: String,
        workspace_id: u32,
        #[arg(long)]
        follow: bool,
    },
}

#[derive(Subcommand)]
pub enum ProfileCmd {
    /// List saved layout profiles
    List,
    /// Save the current window/workspace layout
    Save {
        name: String,
        #[arg(long)]
        overwrite: bool,
    },
    /// Show one saved layout profile
    Get { name: String },
    /// Delete a saved layout profile
    Delete { name: String },
    /// Restore a saved layout profile
    Restore { name: String },
}

#[derive(Subcommand)]
pub enum InputCmd {
    /// Type a string
    Type { text: String },
    /// Press a single key
    Key { key: String },
}

#[derive(Subcommand)]
pub enum MouseCmd {
    /// Move cursor to position
    Move { x: f64, y: f64 },
    /// Click: left, middle, right
    Click { button: String },
    /// Scroll: dx dy
    Scroll { dx: f64, dy: f64 },
}

#[derive(Subcommand)]
pub enum ClipboardCmd {
    /// Read clipboard contents
    Read,
    /// Write to clipboard
    Write { text: String },
}

#[derive(Subcommand)]
pub enum NotifyCmd {
    /// Send a notification
    Send {
        #[arg(long)]
        title: String,
        #[arg(long)]
        body: String,
        #[arg(long, default_value = "normal")]
        urgency: String,
    },
    /// Close a notification
    Close { notification_id: u32 },
}

#[derive(Subcommand)]
pub enum SystemCmd {
    /// Show system info
    Info,
    /// Get idle seconds
    Idle,
    /// Power action
    Power { action: String },
    /// Battery status
    Battery,
    /// Inhibit sleep/shutdown/idle while work is active
    Inhibit {
        what: String,
        #[arg(long, default_value = "deskbrid")]
        who: String,
        #[arg(long)]
        why: Option<String>,
        #[arg(long)]
        mode: Option<String>,
    },
    /// Release a Deskbrid-created inhibitor
    ReleaseInhibit { inhibitor_id: u32 },
    /// List logind sessions
    Sessions,
    /// Lock the current or specified logind session
    LockSession { session_id: Option<String> },
    /// Switch to another display-manager user
    SwitchUser { username: String },
    /// Check a polkit action without prompting
    CheckAuth { action_id: String },
    /// Request polkit authorization with user interaction
    Elevate {
        action_id: String,
        #[arg(long)]
        reason: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ServiceCmd {
    /// Show one unit's status
    Status { name: String },
    /// Start a unit
    Start { name: String },
    /// Stop a unit
    Stop { name: String },
    /// Restart a unit
    Restart { name: String },
    /// Enable a unit
    Enable {
        name: String,
        #[arg(long)]
        runtime: bool,
    },
    /// Disable a unit
    Disable {
        name: String,
        #[arg(long)]
        runtime: bool,
    },
    /// List units by type
    List { unit_type: Option<String> },
}

#[derive(Subcommand)]
pub enum JournalCmd {
    /// Query journald lines
    Query {
        #[arg(long)]
        since: Option<u64>,
        #[arg(long)]
        until: Option<u64>,
        #[arg(long)]
        unit: Option<String>,
        #[arg(long)]
        priority: Option<u8>,
        #[arg(long)]
        tail: Option<u32>,
    },
}

#[derive(Subcommand)]
pub enum TimerCmd {
    /// List systemd timers
    List,
    /// Start a timer
    Start { name: String },
    /// Stop a timer
    Stop { name: String },
}

#[derive(Subcommand)]
pub enum TerminalCmd {
    /// Create an interactive PTY session
    Create {
        #[arg(long)]
        shell: Option<String>,
        #[arg(long)]
        cwd: Option<String>,
        #[arg(long)]
        rows: Option<u16>,
        #[arg(long)]
        cols: Option<u16>,
    },
    /// Write text to a terminal session
    Write { terminal_id: String, input: String },
    /// Read buffered terminal output
    Read {
        terminal_id: String,
        #[arg(long)]
        max_bytes: Option<u64>,
        #[arg(long, default_value_t = true)]
        flush: bool,
    },
    /// Resize a terminal session
    Resize {
        terminal_id: String,
        rows: u16,
        cols: u16,
    },
    /// List active terminal sessions
    List,
    /// Kill a terminal session
    Kill {
        terminal_id: String,
        #[arg(long)]
        signal: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum NetworkCmd {
    /// Connection status
    Status,
    /// List interfaces
    Interfaces,
}

#[derive(Subcommand)]
pub enum WifiCmd {
    /// Scan for networks
    Scan,
    /// Connect to a network
    Connect { ssid: String },
}

#[derive(Subcommand)]
pub enum BluetoothCmd {
    /// List known devices
    List,
    /// Scan for devices
    Scan,
    /// Connect to device
    Connect { address: String },
    /// Disconnect device
    Disconnect { address: String },
}

#[derive(Subcommand)]
pub enum FilesCmd {
    /// Search for files
    Search {
        pattern: String,
        #[arg(long)]
        root: Option<String>,
        #[arg(long, default_value = "50")]
        max_results: u32,
    },
    /// Watch a path for changes
    Watch { path: String },
    /// Stop watching a path
    Unwatch { path: String },
}

#[derive(Subcommand)]
pub enum AudioCmd {
    /// List audio sinks
    Sinks,
    /// Set sink volume
    Volume { sink_id: u32, volume: f64 },
}

#[derive(Subcommand)]
pub enum MonitorCmd {
    /// List monitors and outputs
    List,
    /// Set the primary monitor/output
    Primary { output: String },
    /// Set output resolution
    Resolution {
        output: String,
        width: u32,
        height: u32,
        #[arg(long)]
        refresh: Option<f64>,
    },
    /// Set output scale
    Scale { output: String, scale: f64 },
    /// Set output rotation: normal, left, right, inverted
    Rotate { output: String, rotation: String },
    /// Enable an output
    Enable { output: String },
    /// Disable an output
    Disable { output: String },
}

pub fn parse() -> Args {
    Args::parse()
}

/// Translate CLI commands into protocol actions
pub(crate) mod into_action;

pub use into_action::into_action;
