use crate::DaemonState;
use crate::protocol::Action;
use serde_json::Value;
use tracing::debug;

pub(crate) async fn execute_blackboard_action(
    action: Action,
    state: &DaemonState,
) -> anyhow::Result<Value> {
    let db = state.database.lock().await;
    let default_ns = "default".to_string();

    match action {
        Action::BlackboardSet {
            key,
            value,
            namespace,
        } => {
            let ns = namespace.unwrap_or(default_ns);
            db.upsert_blackboard(&key, &value, &ns, None)?;
            debug!("blackboard: {}/{} = {}", ns, key, value);
            Ok(serde_json::json!({"ok": true, "key": key, "namespace": ns}))
        }
        Action::BlackboardGet { key, namespace } => {
            let ns = namespace.unwrap_or(default_ns);
            match db.get_blackboard(&key, &ns)? {
                Some(value) => Ok(serde_json::json!({"key": key, "namespace": ns, "value": value})),
                None => Ok(
                    serde_json::json!({"key": key, "namespace": ns, "value": null, "found": false}),
                ),
            }
        }
        Action::BlackboardDelete { key, namespace } => {
            let ns = namespace.unwrap_or(default_ns);
            let deleted = db.delete_blackboard(&key, &ns)?;
            Ok(serde_json::json!({"ok": true, "key": key, "namespace": ns, "deleted": deleted}))
        }
        Action::BlackboardList { namespace } => {
            let ns = namespace.unwrap_or(default_ns);
            let keys = db.blackboard_keys(&ns)?;
            let mut entries: Vec<Value> = Vec::new();
            for k in &keys {
                if let Some(v) = db.get_blackboard(k, &ns)? {
                    entries.push(serde_json::json!({"key": k, "value": v}));
                }
            }
            entries.sort_by(|a, b| a["key"].as_str().cmp(&b["key"].as_str()));
            Ok(serde_json::json!({"namespace": ns, "entries": entries, "count": entries.len()}))
        }
        _ => anyhow::bail!("not a blackboard action"),
    }
}

pub(crate) fn is_blackboard_action(action: &Action) -> bool {
    matches!(
        action,
        Action::BlackboardSet { .. }
            | Action::BlackboardGet { .. }
            | Action::BlackboardDelete { .. }
            | Action::BlackboardList { .. }
    )
}
