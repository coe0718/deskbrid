use crate::protocol::Region;
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn process_exists(pid: u32) -> anyhow::Result<bool> {
    let rc = unsafe { libc::kill(pid as i32, 0) };
    if rc == 0 {
        return Ok(true);
    }
    let errno = std::io::Error::last_os_error()
        .raw_os_error()
        .unwrap_or_default();
    if errno == libc::ESRCH {
        Ok(false)
    } else {
        anyhow::bail!(
            "failed to check pid {}: {}",
            pid,
            std::io::Error::last_os_error()
        )
    }
}

pub(crate) fn region_param(params: &Value) -> anyhow::Result<Option<Region>> {
    let Some(region) = params.get("region") else {
        return Ok(None);
    };
    if region.is_null() {
        return Ok(None);
    }
    Ok(Some(Region {
        x: region["x"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("region.x is required"))? as u32,
        y: region["y"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("region.y is required"))? as u32,
        width: region["width"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("region.width is required"))? as u32,
        height: region["height"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("region.height is required"))? as u32,
    }))
}

pub(crate) fn param_string(params: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        params
            .get(*key)
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned)
    })
}

pub(crate) fn param_u32(params: &Value, keys: &[&str]) -> anyhow::Result<u32> {
    let value = param_u64(params, keys)?;
    if value == 0 || value > u32::MAX as u64 {
        anyhow::bail!("parameter '{}' must be a positive u32", keys[0]);
    }
    Ok(value as u32)
}

pub(crate) fn param_u64(params: &Value, keys: &[&str]) -> anyhow::Result<u64> {
    param_u64_optional(params, keys)
        .ok_or_else(|| anyhow::anyhow!("missing numeric parameter '{}'", keys[0]))
}

pub(crate) fn param_u64_optional(params: &Value, keys: &[&str]) -> Option<u64> {
    keys.iter().find_map(|key| {
        params.get(*key).and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_str().and_then(|s| s.parse::<u64>().ok()))
        })
    })
}

pub(crate) fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
