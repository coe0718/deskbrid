use anyhow::bail;

pub fn wait_params(params: Vec<String>) -> anyhow::Result<serde_json::Value> {
    let mut obj = serde_json::Map::new();
    for param in params {
        let Some((key, value)) = param.split_once('=') else {
            bail!("wait params must use key=value syntax");
        };
        if key.trim().is_empty() {
            bail!("wait param key must not be empty");
        }
        obj.insert(key.trim().to_string(), parse_wait_value(value.trim()));
    }
    Ok(serde_json::Value::Object(obj))
}

fn parse_wait_value(value: &str) -> serde_json::Value {
    if let Ok(value) = value.parse::<u64>() {
        return serde_json::json!(value);
    }
    if let Ok(value) = value.parse::<f64>()
        && value.is_finite()
    {
        return serde_json::json!(value);
    }
    match value {
        "true" => serde_json::json!(true),
        "false" => serde_json::json!(false),
        _ => serde_json::json!(value),
    }
}
