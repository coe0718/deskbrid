use super::Action;
use serde_json::json;

pub(super) fn serialize_apps(action: &Action, id: &str) -> serde_json::Value {
    match action {
        // Apps
        Action::AppList {
            categories,
            mime_types,
            include_hidden,
            limit,
        } => {
            let mut obj = json!({"type": "apps.list", "id": id, "categories": categories, "mime_types": mime_types, "include_hidden": include_hidden});
            if let Some(limit) = limit {
                obj["limit"] = json!(limit);
            }
            obj
        }
        Action::AppSearch { query, limit } => {
            let mut obj = json!({"type": "apps.search", "id": id, "query": query});
            if let Some(limit) = limit {
                obj["limit"] = json!(limit);
            }
            obj
        }
        Action::AppGet { app_id } => json!({"type": "apps.get", "id": id, "app_id": app_id}),

        // MPRIS media control
        Action::MprisList => json!({"type": "mpris.list", "id": id}),
        Action::MprisGet { player } => {
            let mut obj = json!({"type": "mpris.get", "id": id});
            if let Some(player) = player {
                obj["player"] = json!(player);
            }
            obj
        }
        Action::MprisControl { player, action } => {
            let mut obj = json!({"type": "mpris.control", "id": id, "action": action});
            if let Some(player) = player {
                obj["player"] = json!(player);
            }
            obj
        }
        _ => serde_json::json!({"error": "not a apps action"}),
    }
}
