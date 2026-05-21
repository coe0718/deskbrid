use super::*;
use crate::protocol;

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
            SystemCmd::Inhibit {
                what,
                who,
                why,
                mode,
            } => Action::SystemInhibit {
                what,
                who,
                why,
                mode,
            },
            SystemCmd::ReleaseInhibit { inhibitor_id } => {
                Action::SystemReleaseInhibit { inhibitor_id }
            }
            SystemCmd::Sessions => Action::SystemListSessions,
            SystemCmd::LockSession { session_id } => Action::SystemLockSession { session_id },
            SystemCmd::SwitchUser { username } => Action::SystemSwitchUser { username },
            SystemCmd::CheckAuth { action_id } => Action::SystemCheckAuth { action_id },
            SystemCmd::Elevate { action_id, reason } => Action::SystemElevate { action_id, reason },
        },

        Command::Service { cmd } => match cmd {
            ServiceCmd::Status { name } => Action::ServiceStatus { name },
            ServiceCmd::Start { name } => Action::ServiceStart { name },
            ServiceCmd::Stop { name } => Action::ServiceStop { name },
            ServiceCmd::Restart { name } => Action::ServiceRestart { name },
            ServiceCmd::Enable { name, runtime } => Action::ServiceEnable { name, runtime },
            ServiceCmd::Disable { name, runtime } => Action::ServiceDisable { name, runtime },
            ServiceCmd::List { unit_type } => Action::ServiceList { unit_type },
        },

        Command::Journal { cmd } => match cmd {
            JournalCmd::Query {
                since,
                until,
                unit,
                priority,
                tail,
            } => Action::JournalQuery {
                since,
                until,
                unit,
                priority,
                tail,
            },
        },

        Command::Timer { cmd } => match cmd {
            TimerCmd::List => Action::TimerList,
            TimerCmd::Start { name } => Action::TimerStart { name },
            TimerCmd::Stop { name } => Action::TimerStop { name },
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

        Command::Monitors { cmd } => match cmd {
            MonitorCmd::List => Action::MonitorList,
            MonitorCmd::Primary { output } => Action::MonitorSetPrimary { output },
            MonitorCmd::Resolution {
                output,
                width,
                height,
                refresh,
            } => Action::MonitorSetResolution {
                output,
                width,
                height,
                refresh_rate: refresh,
            },
            MonitorCmd::Scale { output, scale } => Action::MonitorSetScale { output, scale },
            MonitorCmd::Rotate { output, rotation } => {
                Action::MonitorSetRotation { output, rotation }
            }
            MonitorCmd::Enable { output } => Action::MonitorEnable { output },
            MonitorCmd::Disable { output } => Action::MonitorDisable { output },
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
