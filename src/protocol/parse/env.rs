use crate::protocol::Action;
use serde_json::Value;

pub(super) fn parse_env(raw: &Value, _id: &str, type_str: &str) -> anyhow::Result<Action> {
    Ok(match type_str {
        "env.get" => Action::EnvGet {
            name: raw["name"].as_str().map(String::from),
        },
        "env.set" => {
            let name = raw["name"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("env.set requires 'name'"))?
                .to_string();
            if name.is_empty() || name.contains('=') {
                anyhow::bail!("env.set: invalid name (empty or contains '='): {:?}", name);
            }
            let value = raw["value"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("env.set requires 'value'"))?
                .to_string();
            Action::EnvSet { name, value }
        }
        // Unknown env.* action
        other => anyhow::bail!("no env parser for {:?}", other),
    })
}
