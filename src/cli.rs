use crate::protocol;
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

    // ─── Wait ───────────────────────────────────────────
    #[command(name = "wait")]
    Wait {
        /// Event to wait for (e.g. window.focus_changed)
        event: String,
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

pub fn parse() -> Args {
    Args::parse()
}

/// Translate CLI commands into protocol actions
pub fn into_action(cmd: Command) -> anyhow::Result<protocol::Action> {
    use protocol::Action;

    Ok(match cmd {
        Command::Windows { cmd } => match cmd {
            WindowCmd::List => Action::WindowsList,
            WindowCmd::Focus { window_id } => Action::WindowsFocus(window_id),
            WindowCmd::Get { window_id } => Action::WindowsGet(window_id),
            WindowCmd::Close { window_id } => Action::WindowsClose(window_id),
            WindowCmd::Minimize { window_id } => Action::WindowsMinimize(window_id),
            WindowCmd::Maximize { window_id } => Action::WindowsMaximize(window_id),
            WindowCmd::MoveResize {
                window_id,
                x,
                y,
                width,
                height,
            } => Action::WindowsMoveResize {
                window_id,
                x,
                y,
                width,
                height,
            },
            WindowCmd::ActivateOrLaunch { app_id, command } => Action::WindowsActivateOrLaunch {
                app_id,
                command,
                workdir: None,
                env: None,
            },
        },

        Command::Workspaces { cmd } => match cmd {
            WorkspaceCmd::List => Action::WorkspacesList,
            WorkspaceCmd::Switch { workspace_id } => Action::WorkspaceSwitch(workspace_id),
            WorkspaceCmd::Move {
                window_id,
                workspace_id,
                follow,
            } => Action::WorkspaceMoveWindow {
                window_id,
                workspace_id,
                follow,
            },
        },

        Command::Profiles { cmd } => match cmd {
            ProfileCmd::List => Action::LayoutProfilesList,
            ProfileCmd::Save { name, overwrite } => Action::LayoutProfileSave { name, overwrite },
            ProfileCmd::Get { name } => Action::LayoutProfileGet { name },
            ProfileCmd::Delete { name } => Action::LayoutProfileDelete { name },
            ProfileCmd::Restore { name } => Action::LayoutProfileRestore { name },
        },

        Command::Combo { keys } => {
            let keys: Vec<String> = keys.split('+').map(|s| s.trim().to_string()).collect();
            Action::InputKeyboardCombo { keys }
        }

        Command::Input { cmd } => match cmd {
            InputCmd::Type { text } => Action::InputKeyboardType { text },
            InputCmd::Key { key } => Action::InputKeyboardKey { key },
        },

        Command::Mouse { cmd } => match cmd {
            MouseCmd::Move { x, y } => Action::InputMouse {
                action: "move".into(),
                x: Some(x),
                y: Some(y),
                button: None,
                dx: None,
                dy: None,
            },
            MouseCmd::Click { button } => Action::InputMouse {
                action: "click".into(),
                x: None,
                y: None,
                button: Some(button),
                dx: None,
                dy: None,
            },
            MouseCmd::Scroll { dx, dy } => Action::InputMouse {
                action: "scroll".into(),
                x: None,
                y: None,
                button: None,
                dx: Some(dx),
                dy: Some(dy),
            },
        },

        Command::Clipboard { cmd } => match cmd {
            ClipboardCmd::Read => Action::ClipboardRead,
            ClipboardCmd::Write { text } => Action::ClipboardWrite { text },
        },

        Command::Screenshot {
            output: _,
            monitor,
            region,
            window,
        } => Action::Screenshot {
            monitor,
            region: region.map(|v| protocol::Region {
                x: v[0],
                y: v[1],
                width: v[2],
                height: v[3],
            }),
            window_id: window,
        },

        Command::Notify { cmd } => match cmd {
            NotifyCmd::Send {
                title,
                body,
                urgency,
            } => Action::NotificationSend {
                app_name: "deskbrid-cli".into(),
                title,
                body,
                urgency,
            },
            NotifyCmd::Close { notification_id } => Action::NotificationClose { notification_id },
        },

        Command::System { cmd } => match cmd {
            SystemCmd::Info => Action::SystemInfo,
            SystemCmd::Idle => Action::SystemIdle,
            SystemCmd::Power { action } => Action::SystemPower { action },
            SystemCmd::Battery => Action::SystemBattery,
        },

        Command::Network { cmd } => match cmd {
            NetworkCmd::Status => Action::NetworkStatus,
            NetworkCmd::Interfaces => Action::NetworkInterfaces,
        },

        Command::Wifi { cmd } => match cmd {
            WifiCmd::Scan => Action::NetworkWifiScan,
            WifiCmd::Connect { ssid } => Action::NetworkWifiConnect {
                ssid,
                password: None,
            },
        },

        Command::Bluetooth { cmd } => match cmd {
            BluetoothCmd::List => Action::BluetoothList,
            BluetoothCmd::Scan => Action::BluetoothScan { duration: Some(10) },
            BluetoothCmd::Connect { address } => Action::BluetoothConnect { address },
            BluetoothCmd::Disconnect { address } => Action::BluetoothDisconnect { address },
        },

        Command::Files { cmd } => match cmd {
            FilesCmd::Search {
                pattern,
                root,
                max_results,
            } => Action::FilesSearch {
                pattern,
                root,
                max_results,
            },
            FilesCmd::Watch { path } => Action::FilesWatch {
                path,
                recursive: true,
                patterns: None,
            },
            FilesCmd::Unwatch { path } => Action::FilesUnwatch { path },
        },

        Command::Audio { cmd } => match cmd {
            AudioCmd::Sinks => Action::AudioListSinks,
            AudioCmd::Volume { sink_id, volume } => Action::AudioSetSinkVolume { sink_id, volume },
        },

        Command::Wait { event } => Action::Subscribe {
            events: vec![event],
        },

        _ => bail!(
            "unexpected command in client mode: {:?}",
            std::mem::discriminant(&cmd)
        ),
    })
}
