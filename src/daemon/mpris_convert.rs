use std::collections::HashMap;
use zbus::zvariant;

pub(crate) fn get_string(
    props: &HashMap<String, zvariant::OwnedValue>,
    key: &str,
) -> Option<String> {
    props
        .get(key)
        .and_then(|value| value.downcast_ref::<zvariant::Str>().ok())
        .map(|value| value.to_string())
}

pub(crate) fn get_bool(props: &HashMap<String, zvariant::OwnedValue>, key: &str) -> bool {
    props
        .get(key)
        .and_then(|value| value.downcast_ref::<bool>().ok())
        .unwrap_or(false)
}

pub(crate) fn mpris_method(action: &str) -> anyhow::Result<&'static str> {
    match action {
        "play_pause" | "toggle" => Ok("PlayPause"),
        "play" => Ok("Play"),
        "pause" => Ok("Pause"),
        "stop" => Ok("Stop"),
        "next" => Ok("Next"),
        "previous" | "prev" => Ok("Previous"),
        _ => anyhow::bail!("unsupported MPRIS action: {}", action),
    }
}

pub(crate) fn owned_value_to_json(value: &zvariant::OwnedValue) -> serde_json::Value {
    if let Ok(value) = value.downcast_ref::<zvariant::Str>() {
        return serde_json::json!(value.to_string());
    }
    if let Ok(value) = value.downcast_ref::<bool>() {
        return serde_json::json!(value);
    }
    if let Ok(value) = value.downcast_ref::<i64>() {
        return serde_json::json!(value);
    }
    if let Ok(value) = value.downcast_ref::<u64>() {
        return serde_json::json!(value);
    }
    if let Ok(value) = value.downcast_ref::<i32>() {
        return serde_json::json!(value);
    }
    if let Ok(value) = value.downcast_ref::<u32>() {
        return serde_json::json!(value);
    }
    if let Ok(value) = value.downcast_ref::<f64>() {
        return serde_json::json!(value);
    }
    if let Ok(array) = value.downcast_ref::<zvariant::Array>() {
        return serde_json::Value::Array(array.iter().map(value_to_json).collect());
    }
    if let Ok(dict) = value.downcast_ref::<zvariant::Dict>() {
        return dict_to_json(dict);
    }
    serde_json::json!(format!("{:?}", value))
}

pub(crate) fn value_to_json(value: &zvariant::Value<'_>) -> serde_json::Value {
    if let Ok(value) = value.downcast_ref::<zvariant::Str>() {
        return serde_json::json!(value.to_string());
    }
    if let Ok(value) = value.downcast_ref::<bool>() {
        return serde_json::json!(value);
    }
    if let Ok(value) = value.downcast_ref::<i64>() {
        return serde_json::json!(value);
    }
    if let Ok(value) = value.downcast_ref::<u64>() {
        return serde_json::json!(value);
    }
    if let Ok(value) = value.downcast_ref::<i32>() {
        return serde_json::json!(value);
    }
    if let Ok(value) = value.downcast_ref::<u32>() {
        return serde_json::json!(value);
    }
    if let Ok(value) = value.downcast_ref::<f64>() {
        return serde_json::json!(value);
    }
    if let Ok(array) = value.downcast_ref::<zvariant::Array>() {
        return serde_json::Value::Array(array.iter().map(value_to_json).collect());
    }
    if let Ok(dict) = value.downcast_ref::<zvariant::Dict>() {
        return dict_to_json(dict);
    }
    serde_json::json!(format!("{:?}", value))
}

pub(crate) fn dict_to_json(dict: zvariant::Dict<'_, '_>) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    for (key, value) in dict.iter() {
        let key = if let Ok(key) = key.downcast_ref::<zvariant::Str>() {
            key.to_string()
        } else {
            format!("{:?}", key)
        };
        obj.insert(key, value_to_json(value));
    }
    serde_json::Value::Object(obj)
}
