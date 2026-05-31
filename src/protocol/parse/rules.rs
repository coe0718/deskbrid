use crate::protocol::Action;
use crate::protocol::rules_types::RuleCondition;
use serde_json::Value;

pub(super) fn parse_rules(raw: &Value, _id: &str, type_str: &str) -> anyhow::Result<Action> {
    Ok(match type_str {
        "rule.create" => Action::RuleCreate {
            name: raw["name"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("rule.create requires 'name'"))?
                .to_string(),
            trigger: serde_json::from_value(raw["trigger"].clone())
                .map_err(|e| anyhow::anyhow!("invalid trigger: {e}"))?,
            condition: if raw.get("condition").is_some() {
                Some(
                    serde_json::from_value(raw["condition"].clone())
                        .map_err(|e| anyhow::anyhow!("invalid condition: {e}"))?,
                )
            } else {
                None
            },
            action_type: raw["action_type"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("rule.create requires 'action_type'"))?
                .to_string(),
            action_params: raw.get("action_params").cloned().unwrap_or(Value::Null),
            enabled: raw["enabled"].as_bool().unwrap_or(true),
            max_fires: raw["max_fires"].as_u64().map(|v| v as u32),
            cooldown_ms: raw["cooldown_ms"].as_u64(),
        },
        "rule.list" => Action::RuleList,
        "rule.get" => Action::RuleGet {
            rule_id: raw["rule_id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("rule.get requires 'rule_id'"))?
                .to_string(),
        },
        "rule.delete" => Action::RuleDelete {
            rule_id: raw["rule_id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("rule.delete requires 'rule_id'"))?
                .to_string(),
        },
        "rule.pause" => Action::RulePause {
            rule_id: raw["rule_id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("rule.pause requires 'rule_id'"))?
                .to_string(),
        },
        "rule.resume" => Action::RuleResume {
            rule_id: raw["rule_id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("rule.resume requires 'rule_id'"))?
                .to_string(),
        },
        _ => anyhow::bail!("unknown rules action: {type_str}"),
    })
}
