use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::Action;
use serde_json::Value;

pub(crate) async fn execute_hotkeys(
    action: Action,
    _backend: &dyn DesktopBackend,
    _state: &DaemonState,
) -> anyhow::Result<Value> {
    use Action::*;
    Ok(match action {
        HotkeysRegister {
            ref hotkey_id,
            ref keys,
        } => serde_json::json!({"registered": hotkey_id, "keys": keys}),
        HotkeysUnregister { ref hotkey_id } => serde_json::json!({"unregistered": hotkey_id}),

        _ => unreachable!("not a hotkeys action"),
    })
}
