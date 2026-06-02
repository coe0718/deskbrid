use crate::DaemonState;
use crate::daemon::macro_engine;
use crate::protocol::Action;

/// Check if an action is a macro-related command.
pub fn is_macro_action(action: &Action) -> bool {
    matches!(
        action,
        Action::MacroRecordStart { .. }
            | Action::MacroRecordStop
            | Action::MacroReplay { .. }
            | Action::MacroList
            | Action::MacroGet { .. }
            | Action::MacroDelete { .. }
            | Action::MacroExport { .. }
            | Action::MacroImport { .. }
    )
}

fn ok_data(id: &str, seq: u64, data: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "type": "response",
        "id": id,
        "seq": seq,
        "status": "ok",
        "data": data
    })
}

/// Handle macro actions. Returns Ok(true) if this was a macro action.
pub async fn execute_macro_action(
    action: &Action,
    state: &DaemonState,
    request_id: &str,
    seq: u64,
    peer_uid: u32,
) -> anyhow::Result<Option<serde_json::Value>> {
    match action {
        Action::MacroRecordStart { name, description } => {
            macro_engine::start_recording(state, name.clone(), description.clone())?;
            Ok(Some(ok_data(
                request_id,
                seq,
                serde_json::json!({"recording": true, "macro_name": name}),
            )))
        }
        Action::MacroRecordStop => {
            let summary = macro_engine::stop_recording(state).await?;
            Ok(Some(ok_data(
                request_id,
                seq,
                serde_json::json!({
                    "macro_name": summary.name,
                    "actions_recorded": summary.action_count,
                    "total_duration_ms": summary.total_duration_ms,
                }),
            )))
        }
        Action::MacroReplay {
            name,
            mode,
            loop_count,
            stop_on_error,
        } => {
            let mode = mode.as_deref().unwrap_or("fast");
            let loop_count = loop_count.unwrap_or(1).max(1);
            let stop_on_error = stop_on_error.unwrap_or(true);
            let results =
                macro_engine::replay_macro(state, name, mode, loop_count, stop_on_error, peer_uid)
                    .await?;
            Ok(Some(ok_data(
                request_id,
                seq,
                serde_json::json!({
                    "macro_name": name,
                    "mode": mode,
                    "loop_count": loop_count,
                    "actions_executed": results.len(),
                }),
            )))
        }
        Action::MacroList => {
            let summaries = macro_engine::list_macros().await?;
            Ok(Some(ok_data(
                request_id,
                seq,
                serde_json::json!({"macros": summaries}),
            )))
        }
        Action::MacroGet { name } => {
            let mf = macro_engine::get_macro(name).await?;
            Ok(Some(ok_data(request_id, seq, serde_json::to_value(mf)?)))
        }
        Action::MacroDelete { name } => {
            macro_engine::delete_macro(name).await?;
            Ok(Some(ok_data(
                request_id,
                seq,
                serde_json::json!({"deleted": name}),
            )))
        }
        Action::MacroExport { name } => {
            let exported = macro_engine::export_macro(name).await?;
            Ok(Some(ok_data(
                request_id,
                seq,
                serde_json::json!({"macro_name": name, "data": exported}),
            )))
        }
        Action::MacroImport { name, data } => {
            let summary = macro_engine::import_macro(name, data).await?;
            Ok(Some(ok_data(
                request_id,
                seq,
                serde_json::json!({
                    "macro_name": summary.name,
                    "actions_imported": summary.action_count,
                }),
            )))
        }
        _ => Ok(None),
    }
}
