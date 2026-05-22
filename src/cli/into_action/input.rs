use super::*;
use crate::protocol::Action;

pub fn into_input_action(cmd: Command) -> anyhow::Result<Action> {
    Ok(match cmd {
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
            ClipboardCmd::History { limit, query } => Action::ClipboardHistoryList { limit, query },
            ClipboardCmd::ClearHistory => Action::ClipboardHistoryClear,
        },

        _ => bail!(
            "unexpected command in client mode: {:?}",
            std::mem::discriminant(&cmd)
        ),
    })
}
