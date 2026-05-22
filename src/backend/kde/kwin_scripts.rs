use super::*;

impl KdeBackend {
    pub(super) fn kwin_find_window_js(id: &str) -> String {
        let id_json = serde_json::to_string(id).unwrap_or_else(|_| "\"\"".to_string());
        format!(
            r#"
var windows = workspace.windowList();
var deskbridNeedle = {id_json};
var deskbridNeedleLower = String(deskbridNeedle).toLowerCase();

function deskbridContainsFold(haystack, needle) {{
    if (!haystack) return false;
    return String(haystack).toLowerCase().indexOf(needle) !== -1;
}}

function deskbridFindWindow() {{
    for (var i = 0; i < windows.length; i++) {{
        var w = windows[i];
        if (String(w.internalId) === deskbridNeedle) return w;
    }}
    for (var i = 0; i < windows.length; i++) {{
        var w = windows[i];
        if (String(w.resourceClass) === deskbridNeedle) return w;
    }}
    for (var i = 0; i < windows.length; i++) {{
        var w = windows[i];
        if (w.caption && String(w.caption) === deskbridNeedle) return w;
    }}
    for (var i = 0; i < windows.length; i++) {{
        var w = windows[i];
        if (deskbridContainsFold(w.resourceClass, deskbridNeedleLower)
            || deskbridContainsFold(w.caption, deskbridNeedleLower)) return w;
    }}
    return null;
}}

var target = deskbridFindWindow();
"#
        )
    }

    pub(super) fn ensure_window_id(id: &str) -> anyhow::Result<()> {
        if id.trim().is_empty() {
            anyhow::bail!("window id must not be empty");
        }
        Ok(())
    }

    pub(super) async fn kwin_expect_marker(
        &self,
        js: &str,
        marker: &str,
        missing_message: &str,
    ) -> anyhow::Result<()> {
        let lines = self.kwin_js(js).await?;
        if lines.iter().any(|l| l.starts_with(marker)) {
            return Ok(());
        }
        if let Some(err) = lines.iter().find(|l| l.starts_with("ERROR:")) {
            anyhow::bail!("{}", err.trim_start_matches("ERROR:"));
        }
        anyhow::bail!("{}", missing_message)
    }

    pub(super) async fn kwin_windows_json(&self) -> anyhow::Result<Vec<serde_json::Value>> {
        let js = r#"
var windows = workspace.windowList();
for (var i = 0; i < windows.length; i++) {
    var w = windows[i];
    var desks = w.desktops || [];
    var ws_id = desks.length > 0 ? Number(desks[0].x11DesktopNumber || 1) : 0;
    print(JSON.stringify({
        id: String(w.internalId),
        title: String(w.caption || ""),
        app_id: String(w.resourceClass || ""),
        x: w.x, y: w.y, width: w.width, height: w.height,
        active: Boolean(w.active),
        minimized: Boolean(w.minimized),
        pid: Number(w.pid),
        ws: ws_id
    }));
}
"#;
        let lines = self.kwin_js(js).await?;
        let mut windows = Vec::new();
        for line in &lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                windows.push(val);
            }
        }
        Ok(windows)
    }
}
