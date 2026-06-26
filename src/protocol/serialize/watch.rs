use super::Action;
use serde_json::json;

pub(super) fn serialize_watch(action: &Action, id: &str) -> serde_json::Value {
    match action {
        Action::RegionWatchCreate {
            name,
            monitor,
            region,
            interval_ms,
            change_threshold_pct,
            notify_on_change,
            notify_on_stable,
            stable_duration_ms,
            auto_save,
            max_changes,
            tolerance,
        } => {
            let mut obj = json!({
                "type": "region_watch.create",
                "id": id,
                "name": name,
                "region": region,
                "notify_on_change": notify_on_change,
                "notify_on_stable": notify_on_stable
            });
            if let Some(value) = monitor {
                obj["monitor"] = json!(value);
            }
            if let Some(value) = interval_ms {
                obj["interval_ms"] = json!(value);
            }
            if let Some(value) = change_threshold_pct {
                obj["change_threshold_pct"] = json!(value);
            }
            if let Some(value) = stable_duration_ms {
                obj["stable_duration_ms"] = json!(value);
            }
            if let Some(value) = auto_save {
                obj["auto_save"] = json!(value);
            }
            if let Some(value) = max_changes {
                obj["max_changes"] = json!(value);
            }
            if let Some(value) = tolerance {
                obj["tolerance"] = json!(value);
            }
            obj
        }
        Action::RegionWatchUpdate {
            name,
            monitor,
            region,
            interval_ms,
            change_threshold_pct,
            notify_on_change,
            notify_on_stable,
            stable_duration_ms,
            auto_save,
            max_changes,
            tolerance,
        } => {
            let mut obj = json!({"type": "region_watch.update", "id": id, "name": name});
            if let Some(value) = monitor {
                obj["monitor"] = json!(value);
            }
            if let Some(value) = region {
                obj["region"] = json!(value);
            }
            if let Some(value) = interval_ms {
                obj["interval_ms"] = json!(value);
            }
            if let Some(value) = change_threshold_pct {
                obj["change_threshold_pct"] = json!(value);
            }
            if let Some(value) = notify_on_change {
                obj["notify_on_change"] = json!(value);
            }
            if let Some(value) = notify_on_stable {
                obj["notify_on_stable"] = json!(value);
            }
            if let Some(value) = stable_duration_ms {
                obj["stable_duration_ms"] = json!(value);
            }
            if let Some(value) = auto_save {
                obj["auto_save"] = json!(value);
            }
            if let Some(value) = max_changes {
                obj["max_changes"] = json!(value);
            }
            if let Some(value) = tolerance {
                obj["tolerance"] = json!(value);
            }
            obj
        }
        Action::RegionWatchRemove { name } => {
            json!({"type": "region_watch.remove", "id": id, "name": name})
        }
        Action::RegionWatchList => json!({"type": "region_watch.list", "id": id}),
        Action::TextWatchCreate {
            name,
            monitor,
            region,
            interval_ms,
            language,
            notify_on_change,
            notify_on_match,
            notify_on_mismatch,
            max_entries,
            psm,
        } => {
            let mut obj = json!({
                "type": "text_watch.create",
                "id": id,
                "name": name,
                "region": region,
                "notify_on_change": notify_on_change
            });
            if let Some(value) = monitor {
                obj["monitor"] = json!(value);
            }
            if let Some(value) = interval_ms {
                obj["interval_ms"] = json!(value);
            }
            if let Some(value) = language {
                obj["language"] = json!(value);
            }
            if let Some(value) = notify_on_match {
                obj["notify_on_match"] = json!(value);
            }
            if let Some(value) = notify_on_mismatch {
                obj["notify_on_mismatch"] = json!(value);
            }
            if let Some(value) = max_entries {
                obj["max_entries"] = json!(value);
            }
            if let Some(value) = psm {
                obj["psm"] = json!(value);
            }
            obj
        }
        Action::TextWatchRemove { name } => {
            json!({"type": "text_watch.remove", "id": id, "name": name})
        }
        Action::TextWatchList => json!({"type": "text_watch.list", "id": id}),
        _ => json!({"error": "not a watch action"}),
    }
}
