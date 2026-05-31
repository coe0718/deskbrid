use crate::protocol::Action;
use serde_json::Value;

pub(super) fn parse_blackboard(raw: &Value, _id: &str, type_str: &str) -> anyhow::Result<Action> {
    let namespace = raw["namespace"].as_str().map(String::from);
    Ok(match type_str {
        "blackboard.set" => Action::BlackboardSet {
            key: raw["key"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("blackboard.set requires 'key'"))?
                .to_string(),
            value: raw["value"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("blackboard.set requires 'value'"))?
                .to_string(),
            namespace,
        },
        "blackboard.get" => Action::BlackboardGet {
            key: raw["key"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("blackboard.get requires 'key'"))?
                .to_string(),
            namespace,
        },
        "blackboard.delete" => Action::BlackboardDelete {
            key: raw["key"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("blackboard.delete requires 'key'"))?
                .to_string(),
            namespace,
        },
        "blackboard.list" => Action::BlackboardList { namespace },
        _ => anyhow::bail!("unknown blackboard action: {type_str}"),
    })
}
