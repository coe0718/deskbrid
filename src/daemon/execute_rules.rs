use crate::DaemonState;
use crate::protocol::Action;
use serde_json::Value;
use tracing::{info, warn};

/// Execute rule-management actions.
pub(crate) async fn execute_rules_action(
    action: Action,
    state: &DaemonState,
) -> anyhow::Result<Value> {
    match action {
        Action::RuleCreate {
            name,
            trigger,
            condition,
            action_type,
            action_params,
            enabled,
            max_fires,
            cooldown_ms,
        } => {
            let rule = crate::protocol::Rule {
                id: uuid::Uuid::new_v4().to_string(),
                name,
                trigger,
                condition,
                action_type,
                action_params,
                enabled,
                max_fires,
                cooldown_ms,
            };

            {
                let db = state.database.lock().unwrap();
                if let Err(e) = db.upsert_rule(&rule) {
                    warn!("Failed to persist rule '{}' to DB: {}", rule.name, e);
                }
            }

            // Register in the in-memory engine
            {
                let mut engine = state.rules.lock().await;
                engine.register(rule.clone());
            }

            info!("Rule '{}' created (id={})", rule.name, rule.id);
            Ok(serde_json::json!({
                "ok": true,
                "rule": {
                    "id": rule.id,
                    "name": rule.name,
                    "enabled": rule.enabled,
                }
            }))
        }

        Action::RuleList => {
            let engine = state.rules.lock().await;
            let rules: Vec<Value> = engine
                .list()
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.id,
                        "name": r.name,
                        "trigger": serde_json::to_value(&r.trigger).unwrap_or_default(),
                        "action_type": r.action_type,
                        "enabled": r.enabled,
                        "max_fires": r.max_fires,
                        "cooldown_ms": r.cooldown_ms,
                    })
                })
                .collect();
            Ok(serde_json::json!({"rules": rules, "count": rules.len()}))
        }

        Action::RuleGet { rule_id } => {
            let engine = state.rules.lock().await;
            match engine.get(&rule_id) {
                Some(r) => Ok(serde_json::json!({
                    "id": r.id,
                    "name": r.name,
                    "trigger": serde_json::to_value(&r.trigger).unwrap_or_default(),
                    "action_type": r.action_type,
                    "action_params": r.action_params,
                    "enabled": r.enabled,
                    "max_fires": r.max_fires,
                    "cooldown_ms": r.cooldown_ms,
                })),
                None => Ok(serde_json::json!({"found": false, "rule_id": rule_id})),
            }
        }

        Action::RuleDelete { rule_id } => {
            let removed = {
                let mut engine = state.rules.lock().await;
                engine.remove(&rule_id)
            };

            if removed.is_some() {
                let db = state.database.lock().unwrap();
                let _ = db.delete_rule(&rule_id);
                info!("Rule '{}' deleted", rule_id);
                Ok(serde_json::json!({"ok": true, "deleted": rule_id}))
            } else {
                Ok(
                    serde_json::json!({"ok": false, "error": format!("rule '{}' not found", rule_id)}),
                )
            }
        }

        Action::RulePause { rule_id } => {
            let found = {
                let mut engine = state.rules.lock().await;
                engine.set_enabled(&rule_id, false)
            };

            if found {
                let db = state.database.lock().unwrap();
                let _ = db.set_rule_enabled(&rule_id, false);
                info!("Rule '{}' paused", rule_id);
                Ok(serde_json::json!({"ok": true, "paused": rule_id}))
            } else {
                Ok(
                    serde_json::json!({"ok": false, "error": format!("rule '{}' not found", rule_id)}),
                )
            }
        }

        Action::RuleResume { rule_id } => {
            let found = {
                let mut engine = state.rules.lock().await;
                engine.set_enabled(&rule_id, true)
            };

            if found {
                let db = state.database.lock().unwrap();
                let _ = db.set_rule_enabled(&rule_id, true);
                info!("Rule '{}' resumed", rule_id);
                Ok(serde_json::json!({"ok": true, "resumed": rule_id}))
            } else {
                Ok(
                    serde_json::json!({"ok": false, "error": format!("rule '{}' not found", rule_id)}),
                )
            }
        }

        _ => anyhow::bail!("unexpected action in rules handler"),
    }
}

/// Check if an action is a rules-management action.
pub(crate) fn is_rules_action(action: &Action) -> bool {
    matches!(
        action,
        Action::RuleCreate { .. }
            | Action::RuleList
            | Action::RuleGet { .. }
            | Action::RuleDelete { .. }
            | Action::RulePause { .. }
            | Action::RuleResume { .. }
    )
}
