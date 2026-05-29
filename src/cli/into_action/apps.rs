use super::*;
use crate::protocol::Action;

pub fn into_apps_action(cmd: Command) -> anyhow::Result<Action> {
    Ok(match cmd {
        Command::Apps { cmd } => match cmd {
            AppCmd::List {
                categories,
                mime_types,
                include_hidden,
                limit,
            } => Action::AppList {
                categories,
                mime_types,
                include_hidden,
                limit,
            },
            AppCmd::Search { query, limit } => Action::AppSearch { query, limit },
            AppCmd::Get { app_id } => Action::AppGet { app_id },
        },

        Command::Mpris { cmd } => match cmd {
            MprisCmd::List => Action::MprisList,
            MprisCmd::Get { player } => Action::MprisGet { player },
            MprisCmd::Control { action, player } => Action::MprisControl { player, action },
        },

        Command::Audio { cmd } => match cmd {
            AudioCmd::Sinks => Action::AudioListSinks,
            AudioCmd::Volume { sink_id, volume } => Action::AudioSetSinkVolume { sink_id, volume },
            AudioCmd::Sources => Action::AudioListSources,
            AudioCmd::GetVolume { target, id } => Action::AudioGetVolume { target, id },
            AudioCmd::SetVolume { target, id, volume } => {
                Action::AudioSetVolume { target, id, volume }
            }
            AudioCmd::Mute { target, id, mute } => Action::AudioMute {
                target,
                id,
                mute: parse_bool_arg(&mute)?,
            },
            AudioCmd::SetDefault { target, name } => Action::AudioSetDefault { target, name },
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

        _ => bail!(
            "unexpected command in client mode: {:?}",
            std::mem::discriminant(&cmd)
        ),
    })
}

fn parse_bool_arg(value: &str) -> anyhow::Result<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" | "mute" => Ok(true),
        "false" | "no" | "off" | "0" | "unmute" => Ok(false),
        _ => bail!("expected boolean value (true/false, on/off, 1/0), got '{value}'"),
    }
}
