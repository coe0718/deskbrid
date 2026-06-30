use super::*;
use crate::cli::sessions::{SessionCmd, VarCmd};
use crate::protocol;

fn into_secrets_action(cmd: &Command) -> protocol::Action {
    match cmd {
        Command::Secrets {
            cmd: SecretsCmd::List,
        } => protocol::Action::SecretsListCollections,
        Command::Secrets {
            cmd: SecretsCmd::Lookup { attributes },
        } => {
            let mut attrs = std::collections::HashMap::new();
            for pair in attributes {
                if let Some((k, v)) = pair.split_once('=') {
                    attrs.insert(k.to_string(), v.to_string());
                }
            }
            protocol::Action::SecretsGetSecret { attributes: attrs }
        }
        Command::Secrets {
            cmd:
                SecretsCmd::Store {
                    attributes,
                    secret,
                    label,
                    collection,
                },
        } => {
            let mut attrs = std::collections::HashMap::new();
            for pair in attributes {
                if let Some((k, v)) = pair.split_once('=') {
                    attrs.insert(k.to_string(), v.to_string());
                }
            }
            protocol::Action::SecretsStoreSecret {
                attributes: attrs,
                secret: secret.clone(),
                label: label.clone(),
                collection: collection.clone(),
            }
        }
        _ => panic!("cli: unmatched command — this is a bug"),
    }
}

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
        | Command::Monitors { .. }
        | Command::Desktop { .. } => desktop::into_desktop_action(cmd),

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

        // Sessions
        Command::Session { cmd } => Ok(match cmd {
            SessionCmd::Create {
                name,
                clone_from,
                profile,
            } => protocol::Action::SessionCreate {
                name: name.clone(),
                clone_from: clone_from.clone(),
                profile: profile.clone(),
            },
            SessionCmd::Destroy { name } => protocol::Action::SessionDestroy { name: name.clone() },
            SessionCmd::List => protocol::Action::SessionList,
            SessionCmd::Switch { name } => protocol::Action::SessionSwitch { name: name.clone() },
            SessionCmd::Suspend { name, reason } => protocol::Action::SessionSuspend {
                name: name.clone(),
                reason: reason.clone(),
            },
            SessionCmd::Resume { name } => protocol::Action::SessionResume { name: name.clone() },
            SessionCmd::Var { cmd: var_cmd } => match var_cmd {
                VarCmd::Set { name, value } => protocol::Action::SessionVarSet {
                    name: name.clone(),
                    value: value.clone(),
                },
                VarCmd::Get { name } => protocol::Action::SessionVarGet { name: name.clone() },
                VarCmd::List => protocol::Action::SessionVarList,
            },
        }),

        Command::Secrets { .. } => Ok(into_secrets_action(&cmd)),

        _ => bail!(
            "unexpected command in client mode: {:?}",
            std::mem::discriminant(&cmd)
        ),
    }
}
