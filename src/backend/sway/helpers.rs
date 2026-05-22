use serde_json::Value;
use std::collections::VecDeque;

use crate::protocol;

/// Parse `swaymsg -t get_tree` into a flat window list.
pub(super) fn parse_sway_tree_windows(tree: &Value) -> Vec<protocol::WindowInfo> {
    let mut windows = Vec::new();
    let mut queue: VecDeque<(&Value, u32, &str)> = VecDeque::new();
    queue.push_back((tree, 0, ""));

    while let Some((node, workspace_id, output_name)) = queue.pop_front() {
        let node_type = node["type"].as_str().unwrap_or("");

        // Track workspace ID and output name as we descend
        let (ws, out) = match node_type {
            "workspace" => {
                let ws_id = node["name"]
                    .as_str()
                    .and_then(|n| n.parse::<u32>().ok())
                    .unwrap_or(workspace_id);
                let out_name = node["output"].as_str().unwrap_or(output_name);
                (ws_id, out_name)
            }
            "output" => {
                let out_name = node["name"].as_str().unwrap_or(output_name);
                (workspace_id, out_name)
            }
            _ => (workspace_id, output_name),
        };

        if (node_type == "con" || node_type == "floating_con")
            && let Some(window) = parse_sway_con_node(node, ws)
        {
            windows.push(window);
        }

        // Recurse into children
        if let Some(nodes) = node["nodes"].as_array() {
            for child in nodes {
                queue.push_back((child, ws, out));
            }
        }
        if let Some(floating) = node["floating_nodes"].as_array() {
            for child in floating {
                queue.push_back((child, ws, out));
            }
        }
    }

    windows
}

fn parse_sway_con_node(node: &Value, workspace_id: u32) -> Option<protocol::WindowInfo> {
    let id = node["id"].as_u64().map(|i| i.to_string())?;
    let title = node["name"].as_str().unwrap_or("").to_string();
    let app_id = node["app_id"].as_str().unwrap_or("").to_string();
    if title.is_empty() && app_id.is_empty() {
        return None;
    }
    let pid = node["pid"].as_u64().map(|p| p as u32);
    let focused = node["focused"].as_bool().unwrap_or(false);

    let geometry = node["rect"].as_object().map(|r| protocol::Geometry {
        x: r.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
        y: r.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
        width: r.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        height: r.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
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
}

/// Parse `swaymsg -t get_workspaces` into workspace list.
pub(super) fn parse_sway_workspaces(raw: &Value) -> Vec<protocol::WorkspaceInfo> {
    raw.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|ws| {
                    let id = ws["num"].as_u64()? as u32;
                    let name = ws["name"].as_str().unwrap_or("").to_string();
                    let focused = ws["focused"].as_bool().unwrap_or(false);
                    Some(protocol::WorkspaceInfo {
                        id,
                        name,
                        is_active: focused,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Parse `swaymsg -t get_outputs` into monitor list.
pub(super) fn parse_sway_outputs(raw: &Value) -> Vec<protocol::MonitorInfo> {
    raw.as_array()
        .map(|arr| {
            arr.iter()
                .enumerate()
                .map(|(i, out)| {
                    let name = out["name"].as_str().unwrap_or("").to_string();
                    let focused = out["focused"].as_bool().unwrap_or(false);
                    let scale = out["scale"].as_f64().unwrap_or(1.0);
                    let enabled = out["active"].as_bool().unwrap_or(false);

                    let (width, height) = if let Some(mode) = out["current_mode"].as_object() {
                        (
                            mode.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                            mode.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                        )
                    } else if let Some(rect) = out["rect"].as_object() {
                        (
                            rect.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                            rect.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                        )
                    } else {
                        (0, 0)
                    };

                    let refresh_rate = out["current_mode"]
                        .as_object()
                        .and_then(|m| m.get("refresh"))
                        .and_then(|r| r.as_f64())
                        .map(|r| r / 1000.0);

                    protocol::MonitorInfo {
                        id: i as u32,
                        name,
                        width,
                        height,
                        scale,
                        primary: focused,
                        enabled,
                        x: out["rect"]["x"].as_i64().unwrap_or(0) as i32,
                        y: out["rect"]["y"].as_i64().unwrap_or(0) as i32,
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
    fn test_parse_sway_tree_single_window() {
        let tree = json!({
            "id": 1, "type": "root", "nodes": [{
                "id": 2, "type": "output", "name": "eDP-1", "nodes": [{
                    "id": 3, "type": "workspace", "name": "1", "nodes": [{
                        "id": 42, "type": "con", "name": "Firefox",
                        "app_id": "firefox", "pid": 1234, "focused": true,
                        "rect": {"x": 0, "y": 0, "width": 1920, "height": 1080}
                    }]
                }]
            }]
        });

        let windows = parse_sway_tree_windows(&tree);
        assert_eq!(windows.len(), 1);
        let w = &windows[0];
        assert_eq!(w.id, "42");
        assert_eq!(w.title, "Firefox");
        assert_eq!(w.app_id, "firefox");
        assert_eq!(w.pid, Some(1234));
        assert!(w.is_focused);
        assert_eq!(w.workspace_id, 1);
    }

    #[test]
    fn test_parse_sway_workspaces() {
        let raw = json!([
            {"num": 1, "name": "1: web", "focused": true, "output": "eDP-1"},
            {"num": 2, "name": "2: code", "focused": false, "output": "DP-2"}
        ]);

        let ws = parse_sway_workspaces(&raw);
        assert_eq!(ws.len(), 2);
        assert_eq!(ws[0].id, 1);
        assert_eq!(ws[0].name, "1: web");
        assert!(ws[0].is_active);
        assert_eq!(ws[1].id, 2);
    }

    #[test]
    fn test_parse_sway_outputs() {
        let raw = json!([
            {"name": "eDP-1", "focused": true, "active": true, "scale": 1.5,
             "rect": {"x": 0, "y": 0, "width": 1920, "height": 1080},
             "current_mode": {"width": 1920, "height": 1080, "refresh": 60000}}
        ]);

        let monitors = parse_sway_outputs(&raw);
        assert_eq!(monitors.len(), 1);
        assert_eq!(monitors[0].name, "eDP-1");
        assert_eq!(monitors[0].width, 1920);
        assert_eq!(monitors[0].height, 1080);
        assert!((monitors[0].scale - 1.5).abs() < 0.01);
        assert!(monitors[0].enabled);
    }
}
