use crate::protocol::Action;
use serde_json::json;

pub(super) fn serialize_rules(action: &Action, id: &str) -> serde_json::Value {
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
            let mut obj = json!({
                "type": "rule.create",
                "id": id,
                "name": name,
                "trigger": serde_json::to_value(trigger).unwrap_or_default(),
                "action_type": action_type,
                "enabled": enabled,
            });
            if let Some(c) = condition {
                obj["condition"] = serde_json::to_value(c).unwrap_or_default();
            }
            if !action_params.is_null() {
                obj["action_params"] = action_params.clone();
            }
            if let Some(mf) = max_fires {
                obj["max_fires"] = json!(mf);
            }
            if let Some(cd) = cooldown_ms {
                obj["cooldown_ms"] = json!(cd);
            }
            obj
        }
        Action::RuleList => json!({"type": "rule.list", "id": id}),
        Action::RuleGet { rule_id } => json!({"type": "rule.get", "id": id, "rule_id": rule_id}),
        Action::RuleDelete { rule_id } => {
            json!({"type": "rule.delete", "id": id, "rule_id": rule_id})
        }
        Action::RulePause { rule_id } => {
            json!({"type": "rule.pause", "id": id, "rule_id": rule_id})
        }
        Action::RuleResume { rule_id } => {
            json!({"type": "rule.resume", "id": id, "rule_id": rule_id})
        }
        _ => serde_json::json!({"error": "not a rules action"}),
    }
}
