use crate::protocol::Action;
use serde_json::Value;

pub fn parse_agent(raw: &Value, _id: &str, s: &str) -> anyhow::Result<Action> {
    Ok(match s {
        "agent.message" => Action::AgentMessage {
            to_session: raw["to_session"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("agent.message requires to_session"))?
                .to_string(),
            subject: raw["subject"].as_str().unwrap_or("").to_string(),
            body: raw["body"].clone(),
            ttl_ms: raw["ttl_ms"].as_u64(),
            reply_to: raw["reply_to"].as_str().map(String::from),
        },
        "agent.broadcast" => Action::AgentBroadcast {
            subject: raw["subject"].as_str().unwrap_or("").to_string(),
            body: raw["body"].clone(),
            exclude_self: raw["exclude_self"].as_bool(),
        },
        "agent.mailbox" => Action::AgentMailbox,
        "agent.register" => {
            let name = raw["name"]
                .as_str()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| anyhow::anyhow!("agent.register requires non-empty name"))?
                .to_string();
            let capabilities = raw["capabilities"]
                .as_array()
                .map(|values| {
                    values
                        .iter()
                        .filter_map(|value| value.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            Action::AgentRegister {
                name,
                agent_type: raw["agent_type"].as_str().map(String::from),
                capabilities,
                metadata: raw.get("metadata").cloned(),
                heartbeat_interval_ms: raw["heartbeat_interval_ms"].as_u64(),
            }
        }
        "agent.list" => Action::AgentList,
        "agent.get" => Action::AgentGet {
            name: raw["name"]
                .as_str()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| anyhow::anyhow!("agent.get requires non-empty name"))?
                .to_string(),
        },
        "agent.heartbeat" => Action::AgentHeartbeat {
            name: raw["name"]
                .as_str()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| anyhow::anyhow!("agent.heartbeat requires non-empty name"))?
                .to_string(),
        },
        _ => anyhow::bail!("unknown agent action: {}", s),
    })
}
