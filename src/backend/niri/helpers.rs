use serde_json::Value;

use crate::protocol;

/// Parse `niri msg --json windows` into window list.
pub(super) fn parse_niri_windows(raw: &Value) -> Vec<protocol::WindowInfo> {
    raw.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|w| {
                    let id = w["id"].as_u64().map(|i| i.to_string())?;
                    let title = w["title"].as_str().unwrap_or("").to_string();
                    let app_id = w["app_id"].as_str().unwrap_or("").to_string();
                    let pid = w["pid"].as_u64().map(|p| p as u32);
                    let focused = w["is_focused"].as_bool().unwrap_or(false);
                    let workspace_id = w["workspace_id"].as_u64().unwrap_or(0) as u32;

                    let geometry = w["geometry"].as_object().map(|g| protocol::Geometry {
                        x: g.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                        y: g.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                        width: g.get("w").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                        height: g.get("h").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                    });

                    Some(protocol::WindowInfo {
                        is_focused: focused,
                        id,
                        title,
                        app_id: app_id.to_ascii_lowercase(),
                        workspace_id,
                        is_minimized: false,
                        geometry,
                        pid,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Parse `niri msg --json workspaces` into workspace list.
pub(super) fn parse_niri_workspaces(raw: &Value) -> Vec<protocol::WorkspaceInfo> {
    raw.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|ws| {
                    let id = ws["id"].as_u64()? as u32;
                    let name = ws["name"].as_str().unwrap_or("").to_string();
                    let active = ws["is_active"].as_bool().unwrap_or(false);
                    Some(protocol::WorkspaceInfo {
                        id,
                        name,
                        is_active: active,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Parse `niri msg --json outputs` into monitor list.
pub(super) fn parse_niri_outputs(raw: &Value) -> Vec<protocol::MonitorInfo> {
    raw.as_array()
        .map(|arr| {
            arr.iter()
                .enumerate()
                .map(|(i, out)| {
                    let name = out["name"].as_str().unwrap_or("").to_string();
                    let (width, height) = out["current_mode"]
                        .as_object()
                        .map(|m| {
                            (
                                m.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                                m.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                            )
                        })
                        .unwrap_or((0, 0));
                    let refresh_rate = out["current_mode"]
                        .as_object()
                        .and_then(|m| m.get("refresh_rate"))
                        .and_then(|r| r.as_f64());

                    protocol::MonitorInfo {
                        id: i as u32,
                        name,
                        width,
                        height,
                        scale: 1.0,
                        primary: i == 0,
                        enabled: true,
                        x: 0,
                        y: 0,
                        refresh_rate,
                        rotation: "normal".to_string(),
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_niri_windows() {
        let raw = json!([{
            "id": 1, "title": "Firefox", "app_id": "firefox",
            "pid": 1234, "is_focused": true, "workspace_id": 1,
            "geometry": {"x": 0, "y": 0, "w": 1920, "h": 1080}
        }]);

        let windows = parse_niri_windows(&raw);
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].title, "Firefox");
        assert_eq!(windows[0].app_id, "firefox");
    }

    #[test]
    fn test_parse_niri_workspaces() {
        let raw = json!([
            {"id": 1, "name": "1", "is_active": true},
            {"id": 2, "name": "2", "is_active": false}
        ]);
        let ws = parse_niri_workspaces(&raw);
        assert_eq!(ws.len(), 2);
        assert_eq!(ws[0].id, 1);
        assert!(ws[0].is_active);
    }
}
