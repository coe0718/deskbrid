use crate::protocol::Action;
use serde_json::Value;

pub(super) fn parse_session(raw: &Value, _id: &str, type_str: &str) -> anyhow::Result<Action> {
    Ok(match type_str {
        "session.create" => Action::SessionCreate {
            name: raw["name"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("session.create requires 'name'"))?
                .to_string(),
            clone_from: raw["clone_from"].as_str().map(String::from),
            profile: raw["profile"].as_str().map(String::from),
        },
        "session.destroy" => Action::SessionDestroy {
            name: raw["name"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("session.destroy requires 'name'"))?
                .to_string(),
        },
        "session.list" => Action::SessionList,
        "session.switch" => Action::SessionSwitch {
            name: raw["name"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("session.switch requires 'name'"))?
                .to_string(),
        },
        "session.suspend" => Action::SessionSuspend {
            name: raw["name"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("session.suspend requires 'name'"))?
                .to_string(),
            reason: raw["reason"].as_str().map(String::from),
        },
        "session.resume" => Action::SessionResume {
            name: raw["name"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("session.resume requires 'name'"))?
                .to_string(),
        },
        "session.var.set" => Action::SessionVarSet {
            name: raw["name"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("session.var.set requires 'name'"))?
                .to_string(),
            value: raw["value"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("session.var.set requires 'value'"))?
                .to_string(),
        },
        "session.var.get" => Action::SessionVarGet {
            name: raw["name"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("session.var.get requires 'name'"))?
                .to_string(),
        },
        "session.var.list" => Action::SessionVarList,
        _ => anyhow::bail!("unknown session action: {type_str}"),
    })
}
