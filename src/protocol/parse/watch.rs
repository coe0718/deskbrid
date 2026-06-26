use super::helpers::*;
use crate::protocol::Action;
use crate::protocol::types::Region;
use serde_json::Value;

pub(super) fn parse_watch(raw: &Value, _id: &str, type_str: &str) -> anyhow::Result<Action> {
    Ok(match type_str {
        "region_watch.create" => Action::RegionWatchCreate {
            name: required_non_empty_string(raw, "name")?,
            monitor: optional_u32(raw, "monitor")?,
            region: required_region(raw)?,
            interval_ms: optional_positive_u64(raw, "interval_ms")?,
            change_threshold_pct: optional_positive_f64(raw, "change_threshold_pct")?,
            notify_on_change: raw["notify_on_change"].as_bool().unwrap_or(true),
            notify_on_stable: raw["notify_on_stable"].as_bool().unwrap_or(false),
            stable_duration_ms: optional_positive_u64(raw, "stable_duration_ms")?,
            auto_save: optional_non_empty_string(raw, "auto_save")?,
            max_changes: optional_u32(raw, "max_changes")?,
            tolerance: optional_u8(raw, "tolerance")?,
        },
        "region_watch.update" => Action::RegionWatchUpdate {
            name: required_non_empty_string(raw, "name")?,
            monitor: optional_u32(raw, "monitor")?,
            region: optional_region(raw)?,
            interval_ms: optional_positive_u64(raw, "interval_ms")?,
            change_threshold_pct: optional_positive_f64(raw, "change_threshold_pct")?,
            notify_on_change: raw.get("notify_on_change").and_then(Value::as_bool),
            notify_on_stable: raw.get("notify_on_stable").and_then(Value::as_bool),
            stable_duration_ms: optional_positive_u64(raw, "stable_duration_ms")?,
            auto_save: optional_non_empty_string(raw, "auto_save")?,
            max_changes: optional_u32(raw, "max_changes")?,
            tolerance: optional_u8(raw, "tolerance")?,
        },
        "region_watch.remove" => Action::RegionWatchRemove {
            name: required_non_empty_string(raw, "name")?,
        },
        "region_watch.list" => Action::RegionWatchList,
        "text_watch.create" => Action::TextWatchCreate {
            name: required_non_empty_string(raw, "name")?,
            monitor: optional_u32(raw, "monitor")?,
            region: required_region(raw)?,
            interval_ms: optional_positive_u64(raw, "interval_ms")?,
            language: optional_non_empty_string(raw, "language")?,
            notify_on_change: raw["notify_on_change"].as_bool().unwrap_or(true),
            notify_on_match: optional_non_empty_string(raw, "notify_on_match")?,
            notify_on_mismatch: optional_non_empty_string(raw, "notify_on_mismatch")?,
            max_entries: optional_u32(raw, "max_entries")?,
            psm: optional_u32(raw, "psm")?,
        },
        "text_watch.remove" => Action::TextWatchRemove {
            name: required_non_empty_string(raw, "name")?,
        },
        "text_watch.list" => Action::TextWatchList,
        _ => anyhow::bail!("unknown watch action: {type_str}"),
    })
}

fn required_region(raw: &Value) -> anyhow::Result<Region> {
    optional_region(raw)?.ok_or_else(|| anyhow::anyhow!("region is required"))
}

fn optional_region(raw: &Value) -> anyhow::Result<Option<Region>> {
    let Some(region) = raw.get("region") else {
        return Ok(None);
    };
    if region.is_null() {
        return Ok(None);
    }
    let width = region["width"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("region.width is required"))?;
    let height = region["height"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("region.height is required"))?;
    if width == 0 || height == 0 || width > u32::MAX as u64 || height > u32::MAX as u64 {
        anyhow::bail!("region.width and region.height must be positive 32-bit integers");
    }
    Ok(Some(Region {
        x: region["x"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("region.x is required"))? as u32,
        y: region["y"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("region.y is required"))? as u32,
        width: width as u32,
        height: height as u32,
    }))
}
