use crate::protocol;
use serde_json::Value;

pub(super) fn parse_wayfire_views(raw: &Value) -> Vec<protocol::WindowInfo> {
    raw.as_array()
        .map(|arr| {
            arr.iter()
                .map(|v| {
                    let id = v["id"].as_u64().map(|n| n.to_string()).unwrap_or_default();
                    let title = v["title"].as_str().unwrap_or("").to_string();
                    let app_id = v["app-id"].as_str().unwrap_or("").to_string();
                    let pid = v["pid"].as_u64().map(|p| p as u32);
                    let workspace_id = v["workspace"]
                        .as_object()
                        .and_then(|ws| ws["x"].as_u64())
                        .unwrap_or(0) as u32;
                    let focused = v["state"]
                        .as_object()
                        .and_then(|s| s["activated"].as_bool())
                        .unwrap_or(false);
                    let minimized = v["state"]
                        .as_object()
                        .and_then(|s| s["minimized"].as_bool())
                        .unwrap_or(false);
                    let geometry = v["geometry"].as_object().map(|g| protocol::Geometry {
                        x: g.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                        y: g.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                        width: g.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                        height: g.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                    });
                    protocol::WindowInfo {
                        is_focused: focused,
                        id,
                        title,
                        app_id: app_id.to_ascii_lowercase(),
                        workspace_id,
                        is_minimized: minimized,
                        geometry,
                        pid,
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn parse_wayfire_workspaces(_raw: &Value) -> Vec<protocol::WorkspaceInfo> {
    vec![protocol::WorkspaceInfo {
        id: 1,
        name: "workspace-1".into(),
        is_active: true,
    }]
}

pub(super) fn parse_wayfire_outputs(raw: &Value) -> Vec<protocol::MonitorInfo> {
    raw.as_array()
        .map(|arr| {
            arr.iter()
                .enumerate()
                .map(|(i, out)| {
                    let name = out["name"].as_str().unwrap_or("").to_string();
                    let (width, height) = out["mode"]
                        .as_object()
                        .map(|m| {
                            (
                                m.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                                m.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                            )
                        })
                        .unwrap_or((0, 0));
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
                        refresh_rate: out["mode"]
                            .as_object()
                            .and_then(|m| m.get("refresh").and_then(|v| v.as_f64())),
                        rotation: "normal".into(),
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}
