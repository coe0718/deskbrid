use crate::protocol::Action;
use serde_json::Value;

pub fn parse_search(raw: &Value, _id: &str, s: &str) -> anyhow::Result<Action> {
    Ok(match s {
        "search.query" => Action::UnifiedSearch {
            query: raw["query"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("search.query requires query"))?
                .to_string(),
            categories: raw["categories"].as_array().map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            }),
            limit: raw["limit"].as_u64().map(|n| n as usize),
        },
        "search.index" => Action::UnifiedIndex,
        _ => anyhow::bail!("unknown search action: {}", s),
    })
}
