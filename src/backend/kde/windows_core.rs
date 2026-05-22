use super::*;
use crate::protocol;

pub(super) async fn windows_list(
    backend: &KdeBackend,
) -> anyhow::Result<Vec<protocol::WindowInfo>> {
    let windows = backend.kwin_windows_json().await?;
    Ok(windows
        .into_iter()
        .map(|w| protocol::WindowInfo {
            id: w["id"].as_str().unwrap_or("").to_string(),
            title: w["title"].as_str().unwrap_or("").to_string(),
            app_id: w["app_id"].as_str().unwrap_or("").to_string(),
            workspace_id: w["ws"].as_i64().unwrap_or(0) as u32,
            is_focused: w["active"].as_bool().unwrap_or(false),
            is_minimized: w["minimized"].as_bool().unwrap_or(false),
            geometry: Some(protocol::Geometry {
                x: w["x"].as_i64().unwrap_or(0) as i32,
                y: w["y"].as_i64().unwrap_or(0) as i32,
                width: w["width"].as_i64().unwrap_or(0) as u32,
                height: w["height"].as_i64().unwrap_or(0) as u32,
            }),
            pid: w["pid"].as_i64().map(|p| p as u32),
        })
        .collect())
}

pub(super) async fn window_focus(backend: &KdeBackend, id: &str) -> anyhow::Result<()> {
    KdeBackend::ensure_window_id(id)?;
    let id_escaped = id.replace('\\', "\\\\").replace('\'', "\\'");
    let js = format!(
        r#"
var windows = workspace.windowList();
var idLower = "{}".toLowerCase();

function containsFold(haystack, needle) {{
    if (!haystack) return false;
    return String(haystack).toLowerCase().indexOf(needle) !== -1;
}}

var target = null;
for (var i = 0; i < windows.length; i++) {{
    var w = windows[i];
    if (String(w.internalId) === "{}") {{ target = w; break; }}
}}
if (!target) {{
    for (var i = 0; i < windows.length; i++) {{
        var w = windows[i];
        if (String(w.resourceClass) === "{}") {{ target = w; break; }}
    }}
}}
if (!target) {{
    for (var i = 0; i < windows.length; i++) {{
        var w = windows[i];
        if (w.caption && String(w.caption) === "{}") {{ target = w; break; }}
    }}
}}
if (!target) {{
for (var i = 0; i < windows.length; i++) {{
    var w = windows[i];
    if (containsFold(w.resourceClass, idLower) || containsFold(w.caption, idLower)) {{
        target = w;
        break;
    }}
}}
}}
if (target) {{
    if (target.minimized) target.minimized = false;
    workspace.activeClient = target;
    print("FOCUSED:" + String(target.internalId));
}}
"#,
        id_escaped, id_escaped, id_escaped, id_escaped
    );
    let lines = backend.kwin_js(&js).await?;
    if !lines.iter().any(|l| l.starts_with("FOCUSED:")) {
        anyhow::bail!("no window matched id: {}", id);
    }
    Ok(())
}

pub(super) async fn window_get(
    backend: &KdeBackend,
    id: &str,
) -> anyhow::Result<protocol::WindowInfo> {
    KdeBackend::ensure_window_id(id)?;
    let id_escaped = id.replace('\\', "\\\\").replace('\'', "\\'");
    let js = format!(
        r#"
var windows = workspace.windowList();
for (var i = 0; i < windows.length; i++) {{
    var w = windows[i];
    if (String(w.internalId) === "{}" || String(w.resourceClass) === "{}") {{
        var desks = w.desktops || [];
        var ws_id = desks.length > 0 ? Number(desks[0].x11DesktopNumber || 1) : 0;
        print(JSON.stringify({{
            id: String(w.internalId),
            title: String(w.caption || ""),
            app_id: String(w.resourceClass || ""),
            x: w.x, y: w.y, width: w.width, height: w.height,
            active: Boolean(w.active),
            minimized: Boolean(w.minimized),
            pid: Number(w.pid),
            ws: ws_id
        }}));
        break;
    }}
}}
"#,
        id_escaped, id_escaped
    );
    let lines = backend.kwin_js(&js).await?;
    for line in &lines {
        let trimmed = line.trim();
        if trimmed.starts_with('{')
            && let Ok(w) = serde_json::from_str::<serde_json::Value>(trimmed)
        {
            return Ok(protocol::WindowInfo {
                id: w["id"].as_str().unwrap_or("").to_string(),
                title: w["title"].as_str().unwrap_or("").to_string(),
                app_id: w["app_id"].as_str().unwrap_or("").to_string(),
                workspace_id: w["ws"].as_i64().unwrap_or(0) as u32,
                is_focused: w["active"].as_bool().unwrap_or(false),
                is_minimized: w["minimized"].as_bool().unwrap_or(false),
                geometry: Some(protocol::Geometry {
                    x: w["x"].as_i64().unwrap_or(0) as i32,
                    y: w["y"].as_i64().unwrap_or(0) as i32,
                    width: w["width"].as_i64().unwrap_or(0) as u32,
                    height: w["height"].as_i64().unwrap_or(0) as u32,
                }),
                pid: w["pid"].as_i64().map(|p| p as u32),
            });
        }
    }
    anyhow::bail!("window not found: {}", id)
}

pub(super) async fn window_close(backend: &KdeBackend, id: &str) -> anyhow::Result<()> {
    KdeBackend::ensure_window_id(id)?;
    let js = format!(
        r#"
{}
if (target) {{
    try {{
        if (typeof target.closeWindow === "function") {{
            target.closeWindow();
            print("CLOSED:" + String(target.internalId));
        }} else if (typeof target.close === "function") {{
            target.close();
            print("CLOSED:" + String(target.internalId));
        }} else {{
            print("ERROR:no close method available");
        }}
    }} catch (e) {{
        print("ERROR:" + e);
    }}
}}
"#,
        KdeBackend::kwin_find_window_js(id)
    );
    backend
        .kwin_expect_marker(&js, "CLOSED:", &format!("window not found: {}", id))
        .await
}
