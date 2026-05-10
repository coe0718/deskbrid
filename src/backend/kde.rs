use crate::protocol;
use crate::protocol::DeskbridEvent;
use async_trait::async_trait;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::process::Command;
use tokio::sync::broadcast;

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

    async fn sh_ok(&self, cmd: &str, args: &[&str]) -> bool {
        let mut command = Command::new(cmd);
        command.args(args).stdin(Stdio::null());
        command.env("XDG_RUNTIME_DIR", &self.xdg_runtime);
        if let Some(sock) = &self.wl_socket {
            command.env("WAYLAND_DISPLAY", sock);
        }
        command
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false)
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
        use std::io::Write;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap();
        let marker = format!("KWIN_DESKBRID_{}", now.as_nanos());
        let wrapped = format!("print(\"{}\");\n{}\nprint(\"{}\");", marker, js, marker);
        let tmp = format!("/tmp/deskbrid_kwin_{}.js", std::process::id());
        {
            let mut f = std::fs::File::create(&tmp)?;
            f.write_all(wrapped.as_bytes())?;
        }

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

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        let out = self
            .sh(
                "journalctl",
                &["_COMM=kwin_wayland", "-o", "cat", "--since", "1 minute ago"],
            )
            .await
            .unwrap_or_default();

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

        let _ = std::fs::remove_file(&tmp);

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

    async fn kwin_windows_json(&self) -> anyhow::Result<Vec<serde_json::Value>> {
        let js = r#"
var windows = workspace.windowList();
for (var i = 0; i < windows.length; i++) {
    var w = windows[i];
    print(JSON.stringify({
        id: String(w.internalId),
        title: String(w.caption || ""),
        app_id: String(w.resourceClass || ""),
        x: w.x, y: w.y, width: w.width, height: w.height,
        active: Boolean(w.active),
        minimized: Boolean(w.minimized),
        pid: Number(w.pid)
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
impl super::DesktopBackend for KdeBackend {
    async fn windows_list(&self) -> anyhow::Result<Vec<protocol::WindowInfo>> {
        let windows = self.kwin_windows_json().await?;
        Ok(windows
            .into_iter()
            .map(|w| protocol::WindowInfo {
                id: w["id"].as_str().unwrap_or("").to_string(),
                title: w["title"].as_str().unwrap_or("").to_string(),
                app_id: w["app_id"].as_str().unwrap_or("").to_string(),
                workspace_id: 0,
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
        let id_escaped = id.replace('\\', "\\\\").replace('\'', "\\'");
        let js = format!(
            r#"
var windows = workspace.windowList();
for (var i = 0; i < windows.length; i++) {{
    var w = windows[i];
    if (String(w.internalId) === "{}" ||
        String(w.resourceClass) === "{}" ||
        (w.caption && String(w.caption).indexOf("{}") !== -1)) {{
        workspace.activeClient = w;
        print("FOCUSED:" + String(w.internalId));
        break;
    }}
}}
"#,
            id_escaped, id_escaped, id_escaped
        );
        let lines = self.kwin_js(&js).await?;
        if !lines.iter().any(|l| l.starts_with("FOCUSED:")) {
            anyhow::bail!("no window matched id: {}", id);
        }
        Ok(())
    }

    async fn window_get(&self, id: &str) -> anyhow::Result<protocol::WindowInfo> {
        let id_escaped = id.replace('\\', "\\\\").replace('\'', "\\'");
        let js = format!(
            r#"
var windows = workspace.windowList();
for (var i = 0; i < windows.length; i++) {{
    var w = windows[i];
    if (String(w.internalId) === "{}" || String(w.resourceClass) === "{}") {{
        print(JSON.stringify({{
            id: String(w.internalId),
            title: String(w.caption || ""),
            app_id: String(w.resourceClass || ""),
            x: w.x, y: w.y, width: w.width, height: w.height,
            active: Boolean(w.active),
            minimized: Boolean(w.minimized),
            pid: Number(w.pid)
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
            if trimmed.starts_with('{') {
                if let Ok(w) = serde_json::from_str::<serde_json::Value>(trimmed) {
                    return Ok(protocol::WindowInfo {
                        id: w["id"].as_str().unwrap_or("").to_string(),
                        title: w["title"].as_str().unwrap_or("").to_string(),
                        app_id: w["app_id"].as_str().unwrap_or("").to_string(),
                        workspace_id: 0,
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
        }
        anyhow::bail!("window not found: {}", id)
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
                "x",
                &format!("{}", x as i32),
                "y",
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
        let path = format!("/tmp/deskbrid_screenshot_{}.png", std::process::id());
        self.sh("grim", &[&path]).await?;

        // Get image dimensions via identify
        let dims = self
            .sh("identify", &["-format", "%w %h", &path])
            .await
            .unwrap_or_default();
        let wh: Vec<u32> = dims
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();

        let path_out = format!("/tmp/deskbrid_screenshot_out_{}.png", std::process::id());
        tokio::fs::rename(&path, &path_out).await.ok();

        Ok(protocol::ScreenshotResult {
            path: path_out,
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
            monitors: Vec::new(),
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
                    name: parts.get(0).unwrap_or(&"").to_string(),
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
            .sh("pactl", &["list", "short", "sinks"])
            .await
            .unwrap_or_default();
        let mut sinks = Vec::new();
        for line in out.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 {
                sinks.push(protocol::AudioSinkInfo {
                    id: parts[0].parse().unwrap_or(0),
                    name: parts[1].to_string(),
                    description: String::new(),
                    volume: 0.0,
                    muted: false,
                });
            }
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
}
