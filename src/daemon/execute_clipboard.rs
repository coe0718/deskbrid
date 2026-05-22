use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::Action;
use serde_json::Value;

pub(crate) async fn execute_clipboard(
    action: Action,
    backend: &dyn DesktopBackend,
    state: &DaemonState,
) -> anyhow::Result<Value> {
    use Action::*;
    Ok(match action {
        ClipboardRead => {
            let text = backend.clipboard_read().await?;
            super::record_clipboard_text(state, &text, "read").await;
            serde_json::json!({"text": text})
        }
        ClipboardWrite { ref text } => {
            backend.clipboard_write(text).await?;
            super::record_clipboard_text(state, text, "write").await;
            serde_json::json!({"written": true})
        }
        ClipboardHistoryList { .. } | ClipboardHistoryClear => {
            anyhow::bail!("clipboard history actions are handled by the daemon dispatcher")
        }
        _ => unreachable!("not a clipboard action"),
    })
}
