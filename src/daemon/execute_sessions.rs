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
        Action::SessionCreate {
            name,
            clone_from,
            profile,
        } => {
            if state.sessions.contains_key(&name) {
                anyhow::bail!("session '{}' already exists", name);
            }
            if let Some(profile_name) = &profile
                && state.permissions.profile(profile_name).is_none()
            {
                anyhow::bail!(
                    "profile '{}' is not defined in permissions.toml",
                    profile_name
                );
            }

            let mut data = if let Some(ref source_name) = clone_from {
                match state.sessions.get(source_name) {
                    Some(source) => {
                        let mut cloned = source.value().clone();
                        cloned.name = name.clone();
                        cloned
                    }
                    None => anyhow::bail!("source session '{}' not found for cloning", source_name),
                }
            } else {
                SessionData::new(name.clone())
            };
            if profile.is_some() {
                data.profile = profile.clone();
            }

            // Persist to database
            {
                let db = state.database.lock().await;
                if let Err(e) = db.upsert_session(&data) {
                    tracing::warn!("Failed to persist session '{}' to DB: {}", name, e);
                }
            }

            state.sessions.insert(name.clone(), data);
            info!("Session '{}' created", name);
            Ok(serde_json::json!({"ok": true, "session": name}))
        }

        Action::SessionDestroy { name } => {
            if state.sessions.remove(&name).is_some() {
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
            let mut list: Vec<Value> = Vec::new();
            for entry in state.sessions.iter() {
                let (name, var_count, profile, created_at, last_active) = {
                    let s = entry.value();
                    (
                        s.name.clone(),
                        s.vars.len(),
                        s.profile.clone(),
                        s.created_at,
                        s.last_active,
                    )
                };
                let suspension = state.auto_suspend.is_suspended(&name).await;
                list.push(serde_json::json!({
                    "name": name.clone(),
                    "var_count": var_count,
                    "profile": profile,
                    "created_at": created_at,
                    "last_active": last_active,
                    "active": name == session_id,
                    "suspended": suspension.is_some(),
                    "suspension": suspension,
                }));
            }
            list.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
            Ok(serde_json::json!({"sessions": list}))
        }

        Action::SessionSwitch { name } => {
            if state.sessions.contains_key(&name) {
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
        Action::SessionSuspend { name, reason } => {
            if !state.sessions.contains_key(&name) {
                anyhow::bail!("session '{}' not found", name);
            }
            let event = state
                .auto_suspend
                .suspend_session(
                    &name,
                    reason.unwrap_or_else(|| "manual suspension".to_string()),
                    "manual",
                    None,
                )
                .await;
            if let Some(event) = event {
                let _ = state.event_tx.send(event);
            }
            Ok(serde_json::json!({"ok": true, "session": name, "suspended": true}))
        }
        Action::SessionResume { name } => {
            let event = state.auto_suspend.resume_session(&name).await;
            if let Some(event) = event {
                let _ = state.event_tx.send(event);
            }
            Ok(serde_json::json!({"ok": true, "session": name, "suspended": false}))
        }

        Action::SessionVarSet { name, value } => {
            {
                let mut session_ref = state
                    .sessions
                    .get_mut(session_id)
                    .ok_or_else(|| anyhow::anyhow!("session '{}' not found", session_id))?;

                session_ref.vars.insert(name.clone(), value.clone());
                session_ref.touch();
            }

            // Persist variable to DB
            {
                let session_ref = state.sessions.get(session_id).unwrap();
                let db = state.database.lock().await;
                let _ = db.upsert_session(session_ref.value());
            }

            Ok(serde_json::json!({"ok": true, "var": name, "value": value}))
        }

        Action::SessionVarGet { name } => {
            let session = state
                .sessions
                .get(session_id)
                .ok_or_else(|| anyhow::anyhow!("session '{}' not found", session_id))?;
            let session = session.value();
            match session.vars.get(&name) {
                Some(value) => Ok(serde_json::json!({"var": name, "value": value})),
                None => Ok(serde_json::json!({"var": name, "value": null, "found": false})),
            }
        }

        Action::SessionVarList => {
            let session = state
                .sessions
                .get(session_id)
                .ok_or_else(|| anyhow::anyhow!("session '{}' not found", session_id))?;
            let session = session.value();

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
            | Action::SessionSuspend { .. }
            | Action::SessionResume { .. }
            | Action::SessionVarSet { .. }
            | Action::SessionVarGet { .. }
            | Action::SessionVarList
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SessionData;

    #[test]
    fn session_data_new_has_empty_vars() {
        let s = SessionData::new("test".into());
        assert_eq!(s.name, "test");
        assert!(s.vars.is_empty());
    }

    #[test]
    fn session_data_vars_set_and_get() {
        let mut s = SessionData::new("agent".into());
        s.vars.insert("key1".into(), "val1".into());
        s.vars.insert("key2".into(), "val2".into());
        assert_eq!(s.vars.get("key1").unwrap(), "val1");
        assert_eq!(s.vars.len(), 2);
    }

    #[test]
    fn is_session_action_recognizes_all_variants() {
        assert!(is_session_action(&Action::SessionList));
        assert!(is_session_action(&Action::SessionCreate {
            name: "x".into(),
            clone_from: None,
            profile: None,
        }));
        assert!(is_session_action(&Action::SessionDestroy {
            name: "x".into()
        }));
        assert!(is_session_action(&Action::SessionSwitch {
            name: "x".into()
        }));
        assert!(is_session_action(&Action::SessionSuspend {
            name: "x".into(),
            reason: None,
        }));
        assert!(is_session_action(&Action::SessionResume {
            name: "x".into()
        }));
        assert!(is_session_action(&Action::SessionVarSet {
            name: "k".into(),
            value: "v".into()
        }));
        assert!(is_session_action(&Action::SessionVarGet {
            name: "k".into()
        }));
        assert!(is_session_action(&Action::SessionVarList));
    }

    #[test]
    fn is_session_action_rejects_other_actions() {
        assert!(!is_session_action(&Action::Ping));
        assert!(!is_session_action(&Action::WindowsList));
        assert!(!is_session_action(&Action::ClipboardRead));
    }
}
