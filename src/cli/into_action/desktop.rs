use super::*;
use crate::protocol::Action;

pub fn into_desktop_action(cmd: Command) -> anyhow::Result<Action> {
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
            WindowCmd::Tile {
                window_id,
                preset,
                monitor,
                padding,
            } => Action::WindowsTile {
                window_id,
                preset,
                monitor,
                padding,
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

        _ => bail!(
            "unexpected command in client mode: {:?}",
            std::mem::discriminant(&cmd)
        ),
    })
}
