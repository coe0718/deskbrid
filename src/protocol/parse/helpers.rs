// Helper functions for JSON validation

pub(super) fn required_non_empty_string(
    raw: &serde_json::Value,
    field: &str,
) -> anyhow::Result<String> {
    let value = raw[field]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing or invalid '{}' field", field))?;
    if value.trim().is_empty() {
        anyhow::bail!("'{}' must not be empty", field);
    }
    Ok(value.to_string())
}

pub(super) fn optional_non_empty_string(
    raw: &serde_json::Value,
    field: &str,
) -> anyhow::Result<Option<String>> {
    let Some(value) = raw.get(field) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let value = value
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("invalid '{}' field", field))?;
    if value.trim().is_empty() {
        anyhow::bail!("'{}' must not be empty", field);
    }
    Ok(Some(value.to_string()))
}

pub(super) fn required_non_empty_string_alias(
    raw: &serde_json::Value,
    primary: &str,
    alias: &str,
) -> anyhow::Result<String> {
    match optional_non_empty_string(raw, primary)? {
        Some(value) => Ok(value),
        None => required_non_empty_string(raw, alias),
    }
}

pub(super) fn optional_non_empty_string_alias(
    raw: &serde_json::Value,
    primary: &str,
    alias: &str,
) -> anyhow::Result<Option<String>> {
    match optional_non_empty_string(raw, primary)? {
        Some(value) => Ok(Some(value)),
        None => optional_non_empty_string(raw, alias),
    }
}

pub(super) fn required_positive_u32(raw: &serde_json::Value, field: &str) -> anyhow::Result<u32> {
    let value = raw[field]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("missing or invalid '{}' field", field))?;
    if value == 0 || value > u32::MAX as u64 {
        anyhow::bail!("'{}' must be a positive 32-bit integer", field);
    }
    Ok(value as u32)
}

pub(super) fn optional_u32(raw: &serde_json::Value, field: &str) -> anyhow::Result<Option<u32>> {
    let Some(value) = raw.get(field) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let value = value
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("invalid '{}' field", field))?;
    if value > u32::MAX as u64 {
        anyhow::bail!("'{}' must fit in a 32-bit integer", field);
    }
    Ok(Some(value as u32))
}

pub(super) fn required_positive_u16(raw: &serde_json::Value, field: &str) -> anyhow::Result<u16> {
    let value = raw[field]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("missing or invalid '{}' field", field))?;
    if value == 0 || value > u16::MAX as u64 {
        anyhow::bail!("'{}' must be a positive 16-bit integer", field);
    }
    Ok(value as u16)
}

pub(super) fn optional_positive_u16(
    raw: &serde_json::Value,
    field: &str,
) -> anyhow::Result<Option<u16>> {
    let Some(value) = raw.get(field) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let value = value
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("invalid '{}' field", field))?;
    if value == 0 || value > u16::MAX as u64 {
        anyhow::bail!("'{}' must be a positive 16-bit integer", field);
    }
    Ok(Some(value as u16))
}

pub(super) fn optional_priority(
    raw: &serde_json::Value,
    field: &str,
) -> anyhow::Result<Option<u8>> {
    let Some(value) = raw.get(field) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let value = value
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("invalid '{}' field", field))?;
    if value > 7 {
        anyhow::bail!("'{}' must be 0-7", field);
    }
    Ok(Some(value as u8))
}

pub(super) fn optional_u8(raw: &serde_json::Value, field: &str) -> anyhow::Result<Option<u8>> {
    let Some(value) = raw.get(field) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let value = value
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("invalid '{}' field", field))?;
    if value > u8::MAX as u64 {
        anyhow::bail!("'{}' must fit in an 8-bit integer", field);
    }
    Ok(Some(value as u8))
}

pub(super) fn optional_positive_u64(
    raw: &serde_json::Value,
    field: &str,
) -> anyhow::Result<Option<u64>> {
    let Some(value) = raw.get(field) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let value = value
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("invalid '{}' field", field))?;
    if value == 0 {
        anyhow::bail!("'{}' must be a positive integer", field);
    }
    Ok(Some(value))
}

pub(super) fn required_positive_f64(raw: &serde_json::Value, field: &str) -> anyhow::Result<f64> {
    let value = raw[field]
        .as_f64()
        .ok_or_else(|| anyhow::anyhow!("missing or invalid '{}' field", field))?;
    if !value.is_finite() || value <= 0.0 {
        anyhow::bail!("'{}' must be a positive finite number", field);
    }
    Ok(value)
}

pub(super) fn required_number(raw: &serde_json::Value, field: &str) -> anyhow::Result<f64> {
    let value = raw[field]
        .as_f64()
        .ok_or_else(|| anyhow::anyhow!("missing or invalid '{}' field", field))?;
    if !value.is_finite() {
        anyhow::bail!("'{}' must be a finite number", field);
    }
    Ok(value)
}

pub(super) fn optional_positive_f64(
    raw: &serde_json::Value,
    field: &str,
) -> anyhow::Result<Option<f64>> {
    let Some(value) = raw.get(field) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let value = value
        .as_f64()
        .ok_or_else(|| anyhow::anyhow!("invalid '{}' field", field))?;
    if !value.is_finite() || value <= 0.0 {
        anyhow::bail!("'{}' must be a positive finite number", field);
    }
    Ok(Some(value))
}

pub(super) fn required_rotation(raw: &serde_json::Value, field: &str) -> anyhow::Result<String> {
    let value = required_non_empty_string(raw, field)?;
    match value.as_str() {
        "normal" | "left" | "right" | "inverted" => Ok(value),
        _ => anyhow::bail!("'{}' must be one of: normal, left, right, inverted", field),
    }
}

pub(super) fn optional_string_array(
    raw: &serde_json::Value,
    field: &str,
) -> anyhow::Result<Vec<String>> {
    let Some(value) = raw.get(field) else {
        return Ok(Vec::new());
    };
    let Some(values) = value.as_array() else {
        anyhow::bail!("'{}' must be an array of strings", field);
    };

    let mut items = Vec::with_capacity(values.len());
    for value in values {
        let Some(item) = value.as_str() else {
            anyhow::bail!("'{}' must be an array of strings", field);
        };
        if item.trim().is_empty() {
            anyhow::bail!("'{}' entries must not be empty", field);
        }
        items.push(item.to_string());
    }
    Ok(items)
}

/// An array of strings with at least one non-empty element
pub(super) fn required_string_array(
    raw: &serde_json::Value,
    field: &str,
) -> anyhow::Result<Vec<String>> {
    let items = optional_string_array(raw, field)?;
    if items.is_empty() {
        anyhow::bail!("'{}' must contain at least one entry", field);
    }
    Ok(items)
}

/// Required path field — must be a non-empty string
pub(super) fn required_path(raw: &serde_json::Value, field: &str) -> anyhow::Result<String> {
    let value = required_non_empty_string(raw, field)?;
    Ok(value)
}
