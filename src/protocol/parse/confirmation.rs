use crate::protocol::Action;
use serde_json::Value;

pub fn parse_confirmation(raw: &Value, _id: &str, s: &str) -> anyhow::Result<Action> {
    Ok(match s {
        "confirmation.confirm" => Action::ConfirmAction {
            id: raw["id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("confirmation.confirm requires id"))?
                .to_string(),
        },
        "confirmation.deny" => Action::DenyAction {
            id: raw["id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("confirmation.deny requires id"))?
                .to_string(),
        },
        "confirmation.list" => Action::ConfirmationList,
        _ => anyhow::bail!("unknown confirmation action: {}", s),
    })
}
