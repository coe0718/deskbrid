use crate::DaemonState;
use crate::SessionData;
use crate::protocol::Action;
use serde_json::Value;
use tracing::info;

/// Execute named session actions (#31).
pub(crate) async fn execute_session_action(
    action: Action,
    state: &DaemonState,
    session_id: &str,
) -> anyhow::Result<Value> {
    match action {
        Action::SessionCreate { name, clone_from } => {
            let mut sessions = state.sessions.lock().await;

            if sessions.contains_key(&name) {
                anyhow::bail!("session '{}' already exists", name);
            }

            let data = if let Some(ref source_name) = clone_from {
                match sessions.get(source_name) {
                    Some(source) => {
                        let mut cloned = source.clone();
                        cloned.name = name.clone();
                        cloned
                    }
                    None => anyhow::bail!("source session '{}' not found for cloning", source_name),
                }
            } else {
                SessionData::new(name.clone())
            };

            // Persist to database
            {
                let db = state.database.lock().await;
                if let Err(e) = db.upsert_session(&data) {
                    tracing::warn!("Failed to persist session '{}' to DB: {}", name, e);
                }
            }

            sessions.insert(name.clone(), data);
            info!("Session '{}' created", name);
            Ok(serde_json::json!({"ok": true, "session": name}))
        }

        Action::SessionDestroy { name } => {
            let mut sessions = state.sessions.lock().await;
            if sessions.remove(&name).is_some() {
                // Remove from database
                let db = state.database.lock().await;
                let _ = db.delete_session(&name);

                info!("Session '{}' destroyed", name);
                Ok(serde_json::json!({"ok": true, "destroyed": name}))
            } else {
                Ok(
                    serde_json::json!({"ok": false, "error": format!("session '{}' not found", name)}),
                )
            }
        }

        Action::SessionList => {
            let sessions = state.sessions.lock().await;
            let mut list: Vec<Value> = Vec::new();
            for s in sessions.values() {
                list.push(serde_json::json!({
                    "name": s.name,
                    "var_count": s.vars.len(),
                    "created_at": s.created_at,
                    "last_active": s.last_active,
                    "active": s.name == session_id,
                }));
            }
            list.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
            Ok(serde_json::json!({"sessions": list}))
        }

        Action::SessionSwitch { name } => {
            let sessions = state.sessions.lock().await;
            if sessions.contains_key(&name) {
                Ok(serde_json::json!({"ok": true, "session": name}))
            } else {
                // Auto-create if doesn't exist
                anyhow::bail!(
                    "session '{}' does not exist — use session.create first or connect with session='{}'",
                    name,
                    name
                )
            }
        }

        Action::SessionVarSet { name, value } => {
            let mut sessions = state.sessions.lock().await;
            let session = sessions
                .get_mut(&name.to_string())
                .ok_or_else(|| anyhow::anyhow!("session '{}' not found", name))?;

            session.vars.insert(name.clone(), value.clone());
            session.touch();

            // Persist variable to DB
            {
                let db = state.database.lock().await;
                let _ = db.upsert_session(session);
            }

            Ok(serde_json::json!({"ok": true, "var": name, "value": value}))
        }

        Action::SessionVarGet { name } => {
            let sessions = state.sessions.lock().await;
            // Look up by session_id, var name
            let session = sessions
                .get(session_id)
                .ok_or_else(|| anyhow::anyhow!("session '{}' not found", session_id))?;

            match session.vars.get(&name) {
                Some(value) => Ok(serde_json::json!({"var": name, "value": value})),
                None => Ok(serde_json::json!({"var": name, "value": null, "found": false})),
            }
        }

        Action::SessionVarList => {
            let sessions = state.sessions.lock().await;
            let session = sessions
                .get(session_id)
                .ok_or_else(|| anyhow::anyhow!("session '{}' not found", session_id))?;

            let mut vars: Vec<Value> = session
                .vars
                .iter()
                .map(|(k, v)| serde_json::json!({"name": k, "value": v}))
                .collect();
            vars.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));

            Ok(serde_json::json!({
                "session": session_id,
                "vars": vars,
                "count": vars.len(),
            }))
        }

        _ => anyhow::bail!("unexpected action in session handler"),
    }
}

/// Check if an action is a session-management action.
pub(crate) fn is_session_action(action: &Action) -> bool {
    matches!(
        action,
        Action::SessionCreate { .. }
            | Action::SessionDestroy { .. }
            | Action::SessionList
            | Action::SessionSwitch { .. }
            | Action::SessionVarSet { .. }
            | Action::SessionVarGet { .. }
            | Action::SessionVarList
    )
}
