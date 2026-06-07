use crate::protocol::Action;
use serde_json::Value;

pub fn parse_secrets(raw: &Value, _id: &str, s: &str) -> anyhow::Result<Action> {
    Ok(match s {
        "secrets.list_collections" => Action::SecretsListCollections,

        "secrets.get_secret" => {
            let attrs = raw
                .get("attributes")
                .ok_or_else(|| anyhow::anyhow!("secrets.get_secret requires attributes"))?;
            let map: std::collections::HashMap<String, String> =
                serde_json::from_value(attrs.clone())
                    .map_err(|e| anyhow::anyhow!("invalid attributes: {e}"))?;
            Action::SecretsGetSecret { attributes: map }
        }

        "secrets.store_secret" => {
            let attrs = raw
                .get("attributes")
                .ok_or_else(|| anyhow::anyhow!("secrets.store_secret requires attributes"))?;
            let attributes: std::collections::HashMap<String, String> =
                serde_json::from_value(attrs.clone())
                    .map_err(|e| anyhow::anyhow!("invalid attributes: {e}"))?;
            let secret = raw["secret"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("secrets.store_secret requires secret"))?
                .to_string();
            let label = raw["label"].as_str().map(|s| s.to_string());
            let collection = raw["collection"].as_str().map(|s| s.to_string());
            Action::SecretsStoreSecret {
                attributes,
                secret,
                label,
                collection,
            }
        }

        _ => anyhow::bail!("unknown secrets action: {s}"),
    })
}
