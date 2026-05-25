use crate::protocol;
use serde_json::Value;

/// Parse `labwc-helper list-windows` JSON output (when labwc-helper is available).
pub(super) fn parse_labwc_windows_json(raw: &Value) -> Vec<protocol::WindowInfo> {
    raw.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|w| {
                    let id = w["window_id"].as_u64().map(|n| n.to_string())?;
                    let title = w["title"].as_str().unwrap_or("").to_string();
                    let app_id = w["app_id"].as_str().unwrap_or("").to_string();
                    let focused = w["focused"].as_bool().unwrap_or(false);
                    let minimized = w["minimized"].as_bool().unwrap_or(false);
                    Some(protocol::WindowInfo {
                        is_focused: focused,
                        id,
                        title,
                        app_id: app_id.to_ascii_lowercase(),
                        workspace_id: 0,
                        is_minimized: minimized,
                        geometry: None,
                        pid: None,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Parse `wlrctl toplevel list` output (fallback when labwc-helper is missing).
///
/// Output format (one window per line):
///   foot: jeremy@jeremy-hp15notebookpc:~
///   firefox: Mozilla Firefox
///
/// wlrctl identifies windows by app_id (the part before the colon+space).
/// The full line is `app_id: title` — we use app_id as the window ID for
/// focus/close/maximize operations and the full title for display.
pub(super) fn parse_wlrctl_windows(
    raw: &str,
    focused_id: Option<&str>,
) -> Vec<protocol::WindowInfo> {
    raw.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            // wlrctl format: "app_id: title" — app_id is before the first colon
            let (app_id, title) = line
                .split_once(':')
                .map(|(a, t)| (a.trim().to_string(), t.trim().to_string()))
                .unwrap_or((line.to_string(), String::new()));
            let title_clean = if let Some((t, _c)) = title.rsplit_once(" (") {
                t.to_string()
            } else {
                title
            };
            let is_focused = focused_id.is_some_and(|fid| fid == app_id);
            Some(protocol::WindowInfo {
                id: app_id.clone(),
                title: title_clean,
                app_id: app_id.to_ascii_lowercase(),
                workspace_id: 0,
                is_focused,
                is_minimized: false,
                geometry: None,
                pid: None,
            })
        })
        .collect()
}
