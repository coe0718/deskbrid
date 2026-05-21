use crate::protocol;
use crate::protocol::DeskbridEvent;
use async_trait::async_trait;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::process::Command;
use tokio::sync::broadcast;

pub(crate) mod helpers;
#[cfg(test)]
mod tests;

use crate::backend::DesktopBackend;

use helpers::*;

pub struct KdeBackend {
    event_tx: broadcast::Sender<DeskbridEvent>,
    watchers: Arc<Mutex<HashMap<String, notify::RecommendedWatcher>>>,
    xdg_runtime: String,
    wl_socket: Option<String>,
}

impl KdeBackend {
    pub async fn new(event_tx: broadcast::Sender<DeskbridEvent>) -> anyhow::Result<Self> {
        let xdg_runtime =
            std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/run/user/1000".to_string());
        let wl_socket = std::env::var("WAYLAND_DISPLAY").ok();
        eprintln!("[deskbrid] KDE backend initialized (xdg={xdg_runtime})");
        Ok(Self {
            event_tx,
            watchers: Arc::new(Mutex::new(HashMap::new())),
            xdg_runtime,
            wl_socket,
        })
    }

    async fn sh(&self, cmd: &str, args: &[&str]) -> anyhow::Result<String> {
        let mut command = Command::new(cmd);
        command
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::piped());
        command.env("XDG_RUNTIME_DIR", &self.xdg_runtime);
        if let Some(sock) = &self.wl_socket {
            command.env("WAYLAND_DISPLAY", sock);
        }
        let output = command.output().await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("{} failed: {}", cmd, stderr.trim());
        }
        Ok(String::from_utf8(output.stdout)?.trim().to_string())
    }

    async fn sh_owned(&self, cmd: &str, args: Vec<String>) -> anyhow::Result<String> {
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        self.sh(cmd, &refs).await
    }

    async fn qdbus(
        &self,
        service: &str,
        path: &str,
        method: &str,
        args: &[&str],
    ) -> anyhow::Result<String> {
        let mut all_args = vec![service, path, method];
        all_args.extend_from_slice(args);
        self.sh("qdbus6", &all_args).await
    }

    async fn kwin_js(&self, js: &str) -> anyhow::Result<Vec<String>> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap();
        let marker = format!("KWIN_DESKBRID_{}", now.as_nanos());
        let wrapped = format!("print(\"{}\");\n{js}\nprint(\"{}\");", marker, marker);
        let tmp = format!("/tmp/deskbrid_kwin_{}.js", std::process::id());
        tokio::fs::write(&tmp, wrapped.as_bytes()).await?;

        let resp = self
            .sh(
                "dbus-send",
                &[
                    "--print-reply",
                    "--dest=org.kde.KWin",
                    "/Scripting",
                    "org.kde.kwin.Scripting.loadScript",
                    &format!("string:{}", tmp),
                ],
            )
            .await?;

        let num = resp
            .split_whitespace()
            .filter_map(|w| w.parse::<u32>().ok())
            .next()
            .ok_or_else(|| anyhow::anyhow!("could not parse script number: {}", resp))?;

        self.sh(
            "dbus-send",
            &[
                "--print-reply",
                "--dest=org.kde.KWin",
                &format!("/Scripting/Script{}", num),
                "org.kde.kwin.Script.run",
            ],
        )
        .await
        .ok();

        // Poll journalctl in a loop — but only the latest KWin logs to reduce stale matches.
        let mut out = String::new();
        for _ in 0..10 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let resp = self
                .sh(
                    "journalctl",
                    &[
                        "--since",
                        "30 seconds ago",
                        "_COMM=kwin_wayland",
                        "-o",
                        "cat",
                        "-n",
                        "300",
                    ],
                )
                .await
                .unwrap_or_default();
            if resp.contains(&marker) {
                out = resp;
                break;
            }
        }

        self.sh(
            "dbus-send",
            &[
                "--dest=org.kde.KWin",
                &format!("/Scripting/Script{}", num),
                "org.kde.kwin.Script.stop",
            ],
        )
        .await
        .ok();

        let _ = tokio::fs::remove_file(&tmp).await;

        let mut in_block = false;
        let mut results = Vec::new();
        for line in out.lines() {
            let trimmed = line.trim();
            if trimmed == marker {
                in_block = !in_block;
                continue;
            }
            if in_block {
                results.push(trimmed.strip_prefix("js: ").unwrap_or(trimmed).to_string());
            }
        }
        Ok(results)
    }

    fn kwin_find_window_js(id: &str) -> String {
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

    fn ensure_window_id(id: &str) -> anyhow::Result<()> {
        if id.trim().is_empty() {
            anyhow::bail!("window id must not be empty");
        }
        Ok(())
    }

    async fn kwin_expect_marker(
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

    async fn kwin_windows_json(&self) -> anyhow::Result<Vec<serde_json::Value>> {
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

#[async_trait]
impl DesktopBackend for KdeBackend {
    async fn windows_list(&self) -> anyhow::Result<Vec<protocol::WindowInfo>> {
        let windows = self.kwin_windows_json().await?;
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

    async fn window_focus(&self, id: &str) -> anyhow::Result<()> {
        Self::ensure_window_id(id)?;
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
        let lines = self.kwin_js(&js).await?;
        if !lines.iter().any(|l| l.starts_with("FOCUSED:")) {
            anyhow::bail!("no window matched id: {}", id);
        }
        Ok(())
    }

    async fn window_get(&self, id: &str) -> anyhow::Result<protocol::WindowInfo> {
        Self::ensure_window_id(id)?;
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
        let lines = self.kwin_js(&js).await?;
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

    async fn window_close(&self, id: &str) -> anyhow::Result<()> {
        Self::ensure_window_id(id)?;
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
            Self::kwin_find_window_js(id)
        );
        self.kwin_expect_marker(&js, "CLOSED:", &format!("window not found: {}", id))
            .await
    }

    async fn window_minimize(&self, id: &str) -> anyhow::Result<()> {
        Self::ensure_window_id(id)?;
        let js = format!(
            r#"
{}
if (target) {{
    try {{
        target.minimized = true;
        print("MINIMIZED:" + String(target.internalId));
    }} catch (e) {{
        print("ERROR:" + e);
    }}
}}
"#,
            Self::kwin_find_window_js(id)
        );
        self.kwin_expect_marker(&js, "MINIMIZED:", &format!("window not found: {}", id))
            .await
    }

    async fn window_maximize(&self, id: &str) -> anyhow::Result<()> {
        Self::ensure_window_id(id)?;
        let js = format!(
            r#"
{}
if (target) {{
    try {{
        var ok = false;
        if (typeof target.setMaximize === "function") {{
            target.setMaximize(true, true);
            ok = true;
        }} else {{
            if ("maximized" in target) {{
                target.maximized = true;
                ok = true;
            }}
            if ("maximizedHorizontally" in target) {{
                target.maximizedHorizontally = true;
                ok = true;
            }}
            if ("maximizedVertically" in target) {{
                target.maximizedVertically = true;
                ok = true;
            }}
        }}
        if (ok) {{
            print("MAXIMIZED:" + String(target.internalId));
        }} else {{
            print("ERROR:no maximize method available");
        }}
    }} catch (e) {{
        print("ERROR:" + e);
    }}
}}
"#,
            Self::kwin_find_window_js(id)
        );
        self.kwin_expect_marker(&js, "MAXIMIZED:", &format!("window not found: {}", id))
            .await
    }

    async fn window_move_resize(
        &self,
        id: &str,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> anyhow::Result<()> {
        Self::ensure_window_id(id)?;
        let js = format!(
            r#"
{}
if (target) {{
    try {{
        var geom = {{x: {}, y: {}, width: {}, height: {}}};
        var ok = false;
        if (typeof target.moveResize === "function") {{
            target.moveResize({}, {}, {}, {});
            ok = true;
        }} else if ("frameGeometry" in target) {{
            target.frameGeometry = geom;
            ok = true;
        }} else if ("geometry" in target) {{
            target.geometry = geom;
            ok = true;
        }} else {{
            target.x = {};
            target.y = {};
            target.width = {};
            target.height = {};
            ok = true;
        }}
        if (ok) {{
            print("MOVED_RESIZED:" + String(target.internalId));
        }} else {{
            print("ERROR:no move/resize method available");
        }}
    }} catch (e) {{
        print("ERROR:" + e);
    }}
}}
"#,
            Self::kwin_find_window_js(id),
            x,
            y,
            width,
            height,
            x,
            y,
            width,
            height,
            x,
            y,
            width,
            height
        );
        self.kwin_expect_marker(&js, "MOVED_RESIZED:", &format!("window not found: {}", id))
            .await
    }

    async fn workspaces_list(&self) -> anyhow::Result<Vec<protocol::WorkspaceInfo>> {
        let js = r#"
var manager = workspace.virtualDesktopManager;
if (manager) {
    var desktops = manager.desktops;
    for (var i = 0; i < desktops.length; i++) {
        var d = desktops[i];
        print(JSON.stringify({
            id: Number(d.x11DesktopNumber || (i + 1)),
            name: String(d.name || "")
        }));
    }
}
"#;
        let lines = self.kwin_js(js).await?;
        let mut workspaces = Vec::new();
        for line in &lines {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(line.trim()) {
                workspaces.push(protocol::WorkspaceInfo {
                    id: val["id"].as_i64().unwrap_or(1) as u32,
                    name: val["name"].as_str().unwrap_or("").to_string(),
                    is_active: false,
                });
            }
        }
        if workspaces.is_empty() {
            workspaces.push(protocol::WorkspaceInfo {
                id: 1,
                name: "Desktop 1".into(),
                is_active: false,
            });
        }
        Ok(workspaces)
    }

    async fn workspace_switch(&self, id: u32) -> anyhow::Result<()> {
        self.qdbus(
            "org.kde.KWin",
            "/KWin",
            "org.kde.KWin.setCurrentDesktop",
            &[&id.to_string()],
        )
        .await?;
        Ok(())
    }

    async fn workspace_move_window(
        &self,
        window_id: &str,
        workspace_id: u32,
        follow: bool,
    ) -> anyhow::Result<()> {
        Self::ensure_window_id(window_id)?;
        let wid = window_id.replace('\\', "\\\\").replace('\'', "\\'");
        let js = format!(
            r#"
var windows = workspace.windowList();
for (var i = 0; i < windows.length; i++) {{
    var w = windows[i];
    if (String(w.internalId) === "{}" || String(w.resourceClass) === "{}") {{
        var manager = workspace.virtualDesktopManager;
        if (manager && manager.desktops && manager.desktops.length > {}) {{
            w.desktops = [manager.desktops[{}]];
            print("MOVED:true");
        }}
        break;
    }}
}}
"#,
            wid,
            wid,
            workspace_id - 1,
            workspace_id - 1
        );
        self.kwin_js(&js).await?;
        if follow {
            self.workspace_switch(workspace_id).await?;
        }
        Ok(())
    }

    async fn keyboard_type(&self, text: &str) -> anyhow::Result<()> {
        self.sh("ydotool", &["type", &text.replace('\n', "\\n")])
            .await?;
        Ok(())
    }

    async fn keyboard_key(&self, key: &str) -> anyhow::Result<()> {
        self.sh("ydotool", &["key", key]).await?;
        Ok(())
    }

    async fn keyboard_combo(&self, keys: &[String]) -> anyhow::Result<()> {
        self.sh("ydotool", &["key", &keys.join("+")]).await?;
        Ok(())
    }

    async fn mouse_move(&self, x: f64, y: f64) -> anyhow::Result<()> {
        self.sh(
            "ydotool",
            &[
                "mousemove",
                "--absolute",
                &format!("{}", x as i32),
                &format!("{}", y as i32),
            ],
        )
        .await?;
        Ok(())
    }

    async fn mouse_click(&self, button: &str) -> anyhow::Result<()> {
        let btn_id = match button {
            "left" => "0x1",
            "middle" => "0x2",
            "right" => "0x3",
            _ => anyhow::bail!("unknown button: {button}"),
        };
        self.sh("ydotool", &["click", btn_id]).await?;
        Ok(())
    }

    async fn mouse_scroll(&self, dx: f64, dy: f64) -> anyhow::Result<()> {
        if dx == 0.0 && dy == 0.0 {
            return Ok(());
        }
        self.sh(
            "ydotool",
            &[
                "mousemove",
                "--wheel",
                &format!("{}", dx as i32),
                &format!("{}", dy as i32),
            ],
        )
        .await?;
        Ok(())
    }

    async fn clipboard_read(&self) -> anyhow::Result<String> {
        self.sh("wl-paste", &[]).await
    }

    async fn clipboard_write(&self, text: &str) -> anyhow::Result<()> {
        use tokio::io::AsyncWriteExt;
        let mut child = Command::new("wl-copy")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .env("XDG_RUNTIME_DIR", &self.xdg_runtime)
            .spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes()).await?;
        }
        child.wait().await?;
        Ok(())
    }

    async fn screenshot(
        &self,
        _monitor: Option<u32>,
        _region: Option<protocol::Region>,
        _window_id: Option<String>,
    ) -> anyhow::Result<protocol::ScreenshotResult> {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        let raw_path = format!("/tmp/deskbrid_screenshot_{}.png", ts);
        let out_path = format!("/tmp/deskbrid_screenshot_out_{}.png", std::process::id());

        // Window screenshot via geometry: full-screen capture then crop
        if let Some(ref wid) = _window_id {
            let info = self.window_get(wid).await?;
            if let Some(geo) = info.geometry {
                self.sh("spectacle", &["-b", "-n", "-o", &raw_path]).await?;
                let crop = format!("{}x{}+{}+{}", geo.width, geo.height, geo.x, geo.y);
                self.sh("convert", &[&raw_path, "-crop", &crop, &out_path])
                    .await?;
                tokio::fs::remove_file(&raw_path).await.ok();
                return Ok(protocol::ScreenshotResult {
                    path: out_path,
                    width: geo.width,
                    height: geo.height,
                    format: "png".into(),
                });
            }
        }

        // Region screenshot
        if let Some(ref r) = _region {
            self.sh("spectacle", &["-b", "-n", "-o", &raw_path]).await?;
            let crop = format!("{}x{}+{}+{}", r.width, r.height, r.x, r.y);
            self.sh("convert", &[&raw_path, "-crop", &crop, &out_path])
                .await?;
            tokio::fs::remove_file(&raw_path).await.ok();
            return Ok(protocol::ScreenshotResult {
                path: out_path,
                width: r.width,
                height: r.height,
                format: "png".into(),
            });
        }

        // Full screen
        self.sh("spectacle", &["-b", "-n", "-o", &out_path]).await?;

        // Get image dimensions via identify
        let dims = self
            .sh("identify", &["-format", "%w %h", &out_path])
            .await
            .unwrap_or_default();
        let wh: Vec<u32> = dims
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();

        Ok(protocol::ScreenshotResult {
            path: out_path,
            width: wh.first().copied().unwrap_or(0),
            height: wh.get(1).copied().unwrap_or(0),
            format: "png".into(),
        })
    }

    async fn notification_send(
        &self,
        app_name: &str,
        title: &str,
        body: &str,
        urgency: &str,
    ) -> anyhow::Result<u32> {
        let urgency_map = match urgency {
            "critical" => "2",
            "high" => "1",
            _ => "0",
        };
        self.sh(
            "notify-send",
            &["-a", app_name, "-u", urgency_map, title, body],
        )
        .await?;
        Ok(0)
    }

    async fn notification_close(&self, _id: u32) -> anyhow::Result<()> {
        Ok(())
    }

    async fn system_info(&self) -> anyhow::Result<protocol::SystemInfo> {
        let _hostname = self.sh("hostname", &[]).await.unwrap_or_default();
        let _kernel = self.sh("uname", &["-r"]).await.unwrap_or_default();

        let version = self
            .qdbus(
                "org.kde.KWin",
                "/KWin",
                "org.kde.KWin.supportInformation",
                &[],
            )
            .await
            .unwrap_or_default();
        let first_line = version.lines().next().unwrap_or("KDE Plasma 6").to_string();

        Ok(protocol::SystemInfo {
            desktop: "KDE Plasma".into(),
            desktop_version: first_line,
            compositor: "KWin".into(),
            session_type: "wayland".into(),
            monitors: self.get_monitors().await.unwrap_or_default(),
            workspace_count: 1,
            current_workspace: 0,
            idle_seconds: 0,
        })
    }

    async fn idle_seconds(&self) -> anyhow::Result<u64> {
        let out = self
            .sh("loginctl", &["show-session", "auto", "-p", "IdleSinceHint"])
            .await?;
        if let Some(val) = out.strip_prefix("IdleSinceHint=") {
            let micros: u64 = val.trim().parse().unwrap_or(0);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64;
            if micros > 0 && now > micros {
                return Ok((now - micros) / 1_000_000);
            }
        }
        Ok(0)
    }

    async fn power_action(&self, action: &str) -> anyhow::Result<()> {
        match action {
            "suspend" => self.sh("systemctl", &["suspend"]).await?,
            "hibernate" => self.sh("systemctl", &["hibernate"]).await?,
            "shutdown" => self.sh("systemctl", &["poweroff"]).await?,
            "reboot" => self.sh("systemctl", &["reboot"]).await?,
            "lock" => self.sh("loginctl", &["lock-session"]).await?,
            "logout" => {
                self.qdbus(
                    "org.kde.ksmserver",
                    "/KSMServer",
                    "org.kde.KSMServerInterface.logout",
                    &["0", "0", "0"],
                )
                .await?
            }
            _ => anyhow::bail!("unknown power action: {action}"),
        };
        Ok(())
    }

    async fn battery_status(&self) -> anyhow::Result<Vec<protocol::BatteryInfo>> {
        let out = self.sh("upower", &["-e"]).await?;
        let mut batteries = Vec::new();
        for line in out.lines() {
            if line.contains("battery") {
                let info = self
                    .sh("upower", &["-i", line.trim()])
                    .await
                    .unwrap_or_default();
                let pct = info
                    .lines()
                    .find(|l| l.trim().starts_with("percentage:"))
                    .and_then(|l| l.split(':').nth(1))
                    .and_then(|s| s.trim().trim_end_matches('%').parse::<f64>().ok())
                    .unwrap_or(0.0);
                let state = info
                    .lines()
                    .find(|l| l.trim().starts_with("state:"))
                    .and_then(|l| l.split(':').nth(1))
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();
                batteries.push(protocol::BatteryInfo {
                    source: line.trim().to_string(),
                    percentage: pct,
                    state,
                    time_remaining_minutes: None,
                });
            }
        }
        Ok(batteries)
    }

    async fn network_status(&self) -> anyhow::Result<protocol::NetworkStatusInfo> {
        let out = self.sh("nmcli", &["-t", "-f", "STATE", "general"]).await?;
        Ok(protocol::NetworkStatusInfo {
            online: out.trim().contains("connected"),
            net_type: String::new(),
        })
    }

    async fn network_interfaces(&self) -> anyhow::Result<Vec<protocol::NetworkInterfaceInfo>> {
        let out = self
            .sh(
                "nmcli",
                &["-t", "-f", "NAME,TYPE,DEVICE,STATE,IP4", "device", "status"],
            )
            .await?;
        let mut interfaces = Vec::new();
        for line in out.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                interfaces.push(protocol::NetworkInterfaceInfo {
                    name: parts.first().unwrap_or(&"").to_string(),
                    state: parts.get(3).unwrap_or(&"").to_string(),
                    ipv4: parts
                        .get(4)
                        .map(|s| s.to_string())
                        .filter(|s| !s.is_empty()),
                    ipv6: None,
                });
            }
        }
        Ok(interfaces)
    }

    async fn wifi_scan(&self) -> anyhow::Result<Vec<protocol::WifiNetworkInfo>> {
        let out = self
            .sh(
                "nmcli",
                &["-t", "-f", "SSID,SIGNAL,SECURITY", "device", "wifi", "list"],
            )
            .await?;
        let mut networks = Vec::new();
        for line in out.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                networks.push(protocol::WifiNetworkInfo {
                    ssid: parts[0].to_string(),
                    strength: parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0),
                    secured: !parts
                        .get(2)
                        .map(|s| s.is_empty() || s.contains("--"))
                        .unwrap_or(true),
                    frequency: None,
                });
            }
        }
        Ok(networks)
    }

    async fn wifi_connect(&self, ssid: &str, password: Option<&str>) -> anyhow::Result<()> {
        let ssid_escaped = ssid.replace('\\', "\\\\").replace('\'', "\\'");
        if let Some(pass) = password {
            self.sh(
                "nmcli",
                &["device", "wifi", "connect", &ssid_escaped, "password", pass],
            )
            .await?;
        } else {
            self.sh("nmcli", &["device", "wifi", "connect", &ssid_escaped])
                .await?;
        }
        Ok(())
    }

    async fn bluetooth_list(&self) -> anyhow::Result<Vec<protocol::BluetoothDeviceInfo>> {
        let out = self.sh("bluetoothctl", &["devices"]).await?;
        let mut devices = Vec::new();
        for line in out.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && parts[0] == "Device" {
                devices.push(protocol::BluetoothDeviceInfo {
                    address: parts[1].to_string(),
                    name: parts[2..].join(" "),
                    connected: false,
                    paired: false,
                    rssi: None,
                });
            }
        }
        Ok(devices)
    }

    async fn bluetooth_scan(&self, _duration: Option<u32>) -> anyhow::Result<()> {
        self.sh("bluetoothctl", &["scan", "on"]).await?;
        Ok(())
    }

    async fn bluetooth_stop_scan(&self) -> anyhow::Result<()> {
        self.sh("bluetoothctl", &["scan", "off"]).await?;
        Ok(())
    }

    async fn bluetooth_connect(&self, address: &str) -> anyhow::Result<()> {
        self.sh("bluetoothctl", &["connect", address]).await?;
        Ok(())
    }

    async fn bluetooth_disconnect(&self, address: &str) -> anyhow::Result<()> {
        self.sh("bluetoothctl", &["disconnect", address]).await?;
        Ok(())
    }

    async fn files_watch(
        &self,
        path: &str,
        recursive: bool,
        patterns: Option<&[String]>,
    ) -> anyhow::Result<()> {
        use notify::{Event, EventKind, RecursiveMode, Watcher};
        let mut watchers = self.watchers.lock().unwrap();
        if watchers.contains_key(path) {
            anyhow::bail!("already watching: {path}");
        }
        let event_tx = self.event_tx.clone();
        let path_owned = path.to_string();
        let patterns_owned = patterns.map(|p| p.to_vec());

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let mut ev = match event.kind {
                    EventKind::Create(_) => {
                        event.paths.first().map(|p| DeskbridEvent::FileCreated {
                            path: p.to_string_lossy().to_string(),
                            timestamp: now,
                        })
                    }
                    EventKind::Modify(_) => {
                        event.paths.first().map(|p| DeskbridEvent::FileModified {
                            path: p.to_string_lossy().to_string(),
                            timestamp: now,
                        })
                    }
                    EventKind::Remove(_) => {
                        event.paths.first().map(|p| DeskbridEvent::FileDeleted {
                            path: p.to_string_lossy().to_string(),
                            timestamp: now,
                        })
                    }
                    _ => None,
                };
                if let Some(ref found) = ev {
                    if let Some(ref pats) = patterns_owned {
                        let path_str = match found {
                            DeskbridEvent::FileCreated { path, .. } => path,
                            DeskbridEvent::FileModified { path, .. } => path,
                            DeskbridEvent::FileDeleted { path, .. } => path,
                            _ => return,
                        };
                        if !pats
                            .iter()
                            .any(|pat| path_str.ends_with(pat.trim_start_matches('*')))
                        {
                            return;
                        }
                    }
                    let _ = event_tx.send(ev.take().unwrap());
                }
            }
        })?;

        let mode = if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        watcher.watch(std::path::Path::new(path), mode)?;
        watchers.insert(path_owned, watcher);
        Ok(())
    }

    async fn files_unwatch(&self, path: &str) -> anyhow::Result<()> {
        self.watchers.lock().unwrap().remove(path);
        Ok(())
    }

    async fn files_search(
        &self,
        pattern: &str,
        root: Option<&str>,
        max_results: u32,
    ) -> anyhow::Result<Vec<String>> {
        let root_path = root.unwrap_or(".");
        let output = self
            .sh(
                "find",
                &[root_path, "-type", "f", "-name", pattern, "-maxdepth", "5"],
            )
            .await
            .unwrap_or_default();
        Ok(output
            .lines()
            .take(max_results as usize)
            .map(|l| l.to_string())
            .collect())
    }

    async fn audio_list_sinks(&self) -> anyhow::Result<Vec<protocol::AudioSinkInfo>> {
        let out = self
            .sh("pactl", &["list", "sinks"])
            .await
            .unwrap_or_default();
        let mut sinks = Vec::new();
        let mut current: Option<protocol::AudioSinkInfo> = None;
        for line in out.lines() {
            if line.starts_with("Sink #") {
                if let Some(sink) = current.take() {
                    sinks.push(sink);
                }
                let id_str = line.trim_start_matches("Sink #");
                current = Some(protocol::AudioSinkInfo {
                    id: id_str.parse().unwrap_or(0),
                    name: String::new(),
                    description: String::new(),
                    volume: 0.0,
                    muted: false,
                });
            } else if let Some(ref mut sink) = current {
                let trimmed = line.trim();
                if let Some(name) = trimmed.strip_prefix("Name: ") {
                    sink.name = name.to_string();
                } else if let Some(desc) = trimmed.strip_prefix("Description: ") {
                    sink.description = desc.to_string();
                } else if trimmed.starts_with("Volume:") {
                    // Parse "front-left: 65536 / 100% / ..." — extract the percentage
                    if let Some(pct_str) = trimmed.split('/').nth(1) {
                        let pct: f64 = pct_str.trim().trim_end_matches('%').parse().unwrap_or(0.0);
                        sink.volume = (pct / 100.0).clamp(0.0, 1.0);
                    }
                } else if let Some(mute) = trimmed.strip_prefix("Mute: ") {
                    sink.muted = mute.trim() == "yes";
                }
            }
        }
        if let Some(sink) = current.take() {
            sinks.push(sink);
        }
        Ok(sinks)
    }

    async fn audio_set_sink_volume(&self, sink_id: u32, volume: f64) -> anyhow::Result<()> {
        let pct = (volume * 100.0) as u32;
        self.sh(
            "pactl",
            &[
                "set-sink-volume",
                &sink_id.to_string(),
                &format!("{}%", pct),
            ],
        )
        .await?;
        Ok(())
    }

    async fn monitor_set_primary(&self, output: &str) -> anyhow::Result<()> {
        self.sh_owned("kscreen-doctor", vec![format!("output.{}.primary", output)])
            .await?;
        Ok(())
    }

    async fn monitor_set_resolution(
        &self,
        output: &str,
        width: u32,
        height: u32,
        refresh_rate: Option<f64>,
    ) -> anyhow::Result<()> {
        let mode = if let Some(refresh) = refresh_rate {
            format!("{}x{}@{}", width, height, format_monitor_float(refresh))
        } else {
            self.kscreen_mode_for(output, width, height)
                .await
                .unwrap_or_else(|_| format!("{}x{}", width, height))
        };
        self.sh_owned(
            "kscreen-doctor",
            vec![format!("output.{}.mode.{}", output, mode)],
        )
        .await?;
        Ok(())
    }

    async fn monitor_set_scale(&self, output: &str, scale: f64) -> anyhow::Result<()> {
        self.sh_owned(
            "kscreen-doctor",
            vec![format!(
                "output.{}.scale.{}",
                output,
                format_monitor_float(scale)
            )],
        )
        .await?;
        Ok(())
    }

    async fn monitor_set_rotation(&self, output: &str, rotation: &str) -> anyhow::Result<()> {
        self.sh_owned(
            "kscreen-doctor",
            vec![format!(
                "output.{}.rotation.{}",
                output,
                kde_rotation(rotation)?
            )],
        )
        .await?;
        Ok(())
    }

    async fn monitor_set_enabled(&self, output: &str, enabled: bool) -> anyhow::Result<()> {
        self.sh_owned(
            "kscreen-doctor",
            vec![format!(
                "output.{}.{}",
                output,
                if enabled { "enable" } else { "disable" }
            )],
        )
        .await?;
        Ok(())
    }
}

