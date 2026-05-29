use super::*;
use crate::protocol;

mod apps;
mod desktop;
mod helpers;
mod input;
mod screenshot;
mod system;
mod terminal;

pub fn into_action(cmd: Command) -> anyhow::Result<protocol::Action> {
    match &cmd {
        Command::Windows { .. }
        | Command::Workspaces { .. }
        | Command::Profiles { .. }
        | Command::Monitors { .. } => desktop::into_desktop_action(cmd),

        Command::Combo { .. }
        | Command::Input { .. }
        | Command::Mouse { .. }
        | Command::Clipboard { .. } => input::into_input_action(cmd),

        Command::Color { .. }
        | Command::Screenshot { .. }
        | Command::Ocr { .. }
        | Command::ScreenshotDiff { .. }
        | Command::Screencast { .. }
        | Command::Portal { .. } => screenshot::into_screenshot_action(cmd),

        Command::Notify { .. }
        | Command::System { .. }
        | Command::Service { .. }
        | Command::Journal { .. }
        | Command::Timer { .. }
        | Command::Audit { .. } => system::into_system_action(cmd),

        Command::Apps { .. }
        | Command::Mpris { .. }
        | Command::Audio { .. }
        | Command::Network { .. }
        | Command::Wifi { .. }
        | Command::Bluetooth { .. }
        | Command::Files { .. } => apps::into_apps_action(cmd),

        Command::Clients => Ok(protocol::Action::ClientsList),

        Command::Terminal { .. } | Command::Wait { .. } => terminal::into_terminal_action(cmd),

        _ => bail!(
            "unexpected command in client mode: {:?}",
            std::mem::discriminant(&cmd)
        ),
    }
}
