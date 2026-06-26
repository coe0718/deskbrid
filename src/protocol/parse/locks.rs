use crate::protocol::Action;
use serde_json::Value;

pub fn parse_lock(raw: &Value, _id: &str, s: &str) -> anyhow::Result<Action> {
    Ok(match s {
        "lock.acquire" => Action::LockAcquire {
            resource: raw["resource"]
                .as_str()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| anyhow::anyhow!("lock.acquire requires non-empty resource"))?
                .to_string(),
            holder: raw["holder"].as_str().map(String::from),
            ttl_ms: raw["ttl_ms"].as_u64(),
            wait_ms: raw["wait_ms"].as_u64(),
            force: raw["force"].as_bool().unwrap_or(false),
        },
        "lock.release" => Action::LockRelease {
            resource: raw["resource"]
                .as_str()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| anyhow::anyhow!("lock.release requires non-empty resource"))?
                .to_string(),
            token: raw["token"]
                .as_str()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| anyhow::anyhow!("lock.release requires non-empty token"))?
                .to_string(),
        },
        "lock.list" => Action::LockList,
        _ => anyhow::bail!("unknown lock action: {}", s),
    })
}
