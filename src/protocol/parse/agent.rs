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
        _ => anyhow::bail!("unknown agent action: {}", s),
    })
}
