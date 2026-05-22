use super::*;
use crate::protocol::Action;

pub fn into_terminal_action(cmd: Command) -> anyhow::Result<Action> {
    Ok(match cmd {
        Command::Terminal { cmd } => match cmd {
            TerminalCmd::Create {
                shell,
                cwd,
                rows,
                cols,
            } => Action::TerminalCreate {
                shell,
                cwd,
                env: None,
                rows,
                cols,
            },
            TerminalCmd::Write { terminal_id, input } => {
                Action::TerminalWrite { terminal_id, input }
            }
            TerminalCmd::Read {
                terminal_id,
                max_bytes,
                flush,
            } => Action::TerminalRead {
                terminal_id,
                max_bytes,
                flush,
            },
            TerminalCmd::Resize {
                terminal_id,
                rows,
                cols,
            } => Action::TerminalResize {
                terminal_id,
                rows,
                cols,
            },
            TerminalCmd::List => Action::TerminalList,
            TerminalCmd::Kill {
                terminal_id,
                signal,
            } => Action::TerminalKill {
                terminal_id,
                signal,
            },
        },

        Command::Wait {
            condition,
            params,
            timeout_ms,
            interval_ms,
        } => Action::WaitFor {
            condition,
            params: super::helpers::wait_params(params)?,
            timeout_ms,
            interval_ms,
        },

        _ => bail!(
            "unexpected command in client mode: {:?}",
            std::mem::discriminant(&cmd)
        ),
    })
}
