use crate::protocol;
use crate::protocol::{DeskbridEvent, Geometry, Region};
use async_trait::async_trait;
use notify::Watcher;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::process::Command;
use tokio::sync::broadcast;
use tracing::debug;
use zbus::zvariant;

// ─── Backend struct ────────────────────────────────────

pub struct GnomeBackend {
    /// DBus session connection for standard freedesktop interfaces.
    conn: zbus::Connection,
    /// Broadcast sender for push events to subscribed clients.
    event_tx: broadcast::Sender<DeskbridEvent>,
    /// Active file watchers keyed by path.
    watchers: Arc<Mutex<HashMap<String, notify::RecommendedWatcher>>>,
    /// Mutter RemoteDesktop session path for input injection via compositor.
    rd_session_path: String,
    /// Mutter ScreenCast stream path for absolute mouse positioning.
    sc_stream_path: String,
    /// Last known mouse position for relative delta calculation.
    last_mouse: std::sync::Mutex<(f64, f64)>,
}

impl GnomeBackend {
    pub async fn new(event_tx: broadcast::Sender<DeskbridEvent>) -> anyhow::Result<Self> {
        let conn = zbus::Connection::session().await?;
        let mut backend = Self {
            conn,
            event_tx,
            watchers: Arc::new(Mutex::new(HashMap::new())),
            rd_session_path: String::new(),
            sc_stream_path: String::new(),
            last_mouse: std::sync::Mutex::new((960.0, 540.0)),
        };
        backend.init_remote_desktop().await?;
        // ScreenCast is best-effort — required for absolute mouse positioning.
        // Relative motion works without it.
        if let Err(e) = backend.init_screen_cast().await {
            tracing::warn!(
                "ScreenCast unavailable (absolute mouse positioning disabled): {}",
                e
            );
        }
        Ok(backend)
    }

    // ─── Shell helpers ──────────────────────────────────

    /// Run a command, return stdout as String. Fails on non-zero exit.
    async fn sh(&self, cmd: &str, args: &[&str]) -> anyhow::Result<String> {
        let output = Command::new(cmd)
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("{} failed: {}", cmd, stderr.trim());
        }
        Ok(String::from_utf8(output.stdout)?.trim().to_string())
    }

    /// Run a command, return true if exit code is 0 (ignore output).
    async fn sh_ok(&self, cmd: &str, args: &[&str]) -> bool {
        Command::new(cmd)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    // ─── Extension DBus helpers ─────────────────────────

    /// Path to the GNOME Shell extension's DBus object.
    const EXT_BUS: &'static str = "org.deskbrid.WindowManager";
    const EXT_PATH: &'static str = "/org/deskbrid/WindowManager";
    const EXT_IFACE: &'static str = "org.deskbrid.WindowManager";

    /// Call an extension DBus method via gdbus. Returns raw string.
    async fn ext_call_parsed(&self, method: &str, extra_args: &[&str]) -> anyhow::Result<String> {
        let method_full = format!("{}.{}", Self::EXT_IFACE, method);
        let mut args = vec![
            "call",
            "--session",
            "--dest",
            Self::EXT_BUS,
            "--object-path",
            Self::EXT_PATH,
            "--method",
            &method_full,
        ];
        args.extend(extra_args);
        self.sh("gdbus", &args).await
    }

    // ─── Remote Desktop input injection ─────────────────

    /// Initialise a Mutter RemoteDesktop session for input injection.
    /// The session lives as long as the zbus connection (the daemon's lifetime).
    async fn init_remote_desktop(&mut self) -> anyhow::Result<()> {
        let reply = self
            .conn
            .call_method(
                Some("org.gnome.Mutter.RemoteDesktop"),
                "/org/gnome/Mutter/RemoteDesktop",
                Some("org.gnome.Mutter.RemoteDesktop"),
                "CreateSession",
                &(),
            )
            .await?;
        let path: zbus::zvariant::OwnedObjectPath = reply.body().deserialize()?;

        self.conn
            .call_method(
                Some("org.gnome.Mutter.RemoteDesktop"),
                path.as_str(),
                Some("org.gnome.Mutter.RemoteDesktop.Session"),
                "Start",
                &(),
            )
            .await?;

        self.rd_session_path = path.to_string();
        tracing::info!("RemoteDesktop session started: {}", self.rd_session_path);
        Ok(())
    }

    /// Initialise a Mutter ScreenCast session and record the primary monitor.
    /// The resulting stream path is needed for absolute mouse positioning.
    async fn init_screen_cast(&mut self) -> anyhow::Result<()> {
        use std::collections::HashMap;

        // Create ScreenCast session
        let props: HashMap<&str, zbus::zvariant::Value<'_>> = HashMap::new();
        let reply = self
            .conn
            .call_method(
                Some("org.gnome.Mutter.ScreenCast"),
                "/org/gnome/Mutter/ScreenCast",
                Some("org.gnome.Mutter.ScreenCast"),
                "CreateSession",
                &(props,),
            )
            .await?;
        let session_path: zbus::zvariant::OwnedObjectPath = reply.body().deserialize()?;

        // Start the session
        self.conn
            .call_method(
                Some("org.gnome.Mutter.ScreenCast"),
                session_path.as_str(),
                Some("org.gnome.Mutter.ScreenCast.Session"),
                "Start",
                &(),
            )
            .await?;

        // RecordMonitor with the known connector name.
        // RecordVirtual returns "Unknown monitor" on GNOME 46.
        let connector = "DP-1";
        tracing::info!("Recording monitor: {}", connector);

        let stream_props: HashMap<&str, zbus::zvariant::Value<'_>> = HashMap::new();
        let reply = self
            .conn
            .call_method(
                Some("org.gnome.Mutter.ScreenCast"),
                session_path.as_str(),
                Some("org.gnome.Mutter.ScreenCast.Session"),
                "RecordMonitor",
                &(connector, stream_props),
            )
            .await?;
        let stream_path: zbus::zvariant::OwnedObjectPath = reply.body().deserialize()?;
        self.sc_stream_path = stream_path.to_string();
        tracing::info!("ScreenCast stream created: {}", self.sc_stream_path);
        Ok(())
    }

    /// Call a void method on the RemoteDesktop session.
    async fn rd_call<B>(&self, method: &str, body: &B) -> anyhow::Result<()>
    where
        B: serde::Serialize + zbus::zvariant::Type,
    {
        self.conn
            .call_method(
                Some("org.gnome.Mutter.RemoteDesktop"),
                self.rd_session_path.as_str(),
                Some("org.gnome.Mutter.RemoteDesktop.Session"),
                method,
                body,
            )
            .await?;
        Ok(())
    }

    /// Press or release a keysym through the Mutter compositor pipeline.
    async fn rd_keysym(&self, keysym: u32, pressed: bool) -> anyhow::Result<()> {
        self.rd_call("NotifyKeyboardKeysym", &(keysym, pressed))
            .await
    }

    /// Press or release a mouse button.
    async fn rd_button(&self, button: i32, pressed: bool) -> anyhow::Result<()> {
        self.rd_call("NotifyPointerButton", &(button, pressed))
            .await
    }
}

// ─── Keysym mapping ─────────────────────────────────────

/// XKB keysyms for common characters and keys.
/// Reference: /usr/include/X11/keysymdef.h
mod keysym {
    // Modifier keys
    pub const SHIFT_L: u32 = 0xFFE1;
    pub const CTRL_L: u32 = 0xFFE3;
    pub const ALT_L: u32 = 0xFFE9;
    pub const SUPER_L: u32 = 0xFFEB;

    // Special keys
    pub const RETURN: u32 = 0xFF0D;
    pub const TAB: u32 = 0xFF09;
    pub const ESCAPE: u32 = 0xFF1B;
    pub const BACKSPACE: u32 = 0xFF08;
    pub const DELETE: u32 = 0xFFFF;
    pub const UP: u32 = 0xFF52;
    pub const DOWN: u32 = 0xFF54;
    pub const LEFT: u32 = 0xFF51;
    pub const RIGHT: u32 = 0xFF53;
    pub const HOME: u32 = 0xFF50;
    pub const END: u32 = 0xFF57;
    pub const PAGE_UP: u32 = 0xFF55;
    pub const PAGE_DOWN: u32 = 0xFF56;
    pub const SPACE: u32 = 0x0020;

    /// Map a key name string to its XKB keysym.
    pub fn from_name(name: &str) -> Option<u32> {
        Some(match name.to_lowercase().as_str() {
            "return" | "enter" => RETURN,
            "tab" => TAB,
            "escape" | "esc" => ESCAPE,
            "backspace" => BACKSPACE,
            "delete" | "del" => DELETE,
            "up" => UP,
            "down" => DOWN,
            "left" => LEFT,
            "right" => RIGHT,
            "home" => HOME,
            "end" => END,
            "page_up" | "pgup" => PAGE_UP,
            "page_down" | "pgdn" => PAGE_DOWN,
            "space" => SPACE,
            // Modifier names
            "shift" | "shift_l" => SHIFT_L,
            "ctrl" | "control" | "control_l" => CTRL_L,
            "alt" | "alt_l" => ALT_L,
            "super" | "super_l" | "meta" | "win" | "windows" => SUPER_L,
            _ => return None,
        })
    }

    /// Map a printable ASCII character to its XKB keysym.
    /// Returns (keysym, needs_shift).
    pub fn from_char(c: char) -> Option<(u32, bool)> {
        match c {
            'a'..='z' => Some((0x0061 + (c as u32 - 'a' as u32), false)),
            'A'..='Z' => Some((0x0061 + (c as u32 - 'A' as u32), true)),
            '0'..='9' => Some((0x0030 + (c as u32 - '0' as u32), false)),
            ' ' => Some((0x0020, false)),
            '.' => Some((0x002E, false)),
            ',' => Some((0x002C, false)),
            ';' => Some((0x003B, false)),
            ':' => Some((0x003B, true)),
            '\'' => Some((0x0027, false)),
            '"' => Some((0x0027, true)),
            '/' => Some((0x002F, false)),
            '?' => Some((0x002F, true)),
            '\\' => Some((0x005C, false)),
            '|' => Some((0x005C, true)),
            '[' => Some((0x005B, false)),
            '{' => Some((0x005B, true)),
            ']' => Some((0x005D, false)),
            '}' => Some((0x005D, true)),
            '-' => Some((0x002D, false)),
            '_' => Some((0x002D, true)),
            '=' => Some((0x003D, false)),
            '+' => Some((0x003D, true)),
            '`' => Some((0x0060, false)),
            '~' => Some((0x0060, true)),
            '!' => Some((0x0031, true)),
            '@' => Some((0x0032, true)),
            '#' => Some((0x0033, true)),
            '$' => Some((0x0034, true)),
            '%' => Some((0x0035, true)),
            '^' => Some((0x0036, true)),
            '&' => Some((0x0037, true)),
            '*' => Some((0x0038, true)),
            '(' => Some((0x0039, true)),
            ')' => Some((0x0030, true)),
            '\n' => Some((RETURN, false)),
            '\t' => Some((TAB, false)),
            _ => None,
        }
    }
}

// ─── Trait implementation ───────────────────────────────

#[async_trait]
impl crate::backend::DesktopBackend for GnomeBackend {
    // ═══════════════════════════════════════════════════════
    //  WINDOWS
    // ═══════════════════════════════════════════════════════

    async fn windows_list(&self) -> anyhow::Result<Vec<protocol::WindowInfo>> {
        // Extension ListWindows() → JSON string of window array
        let raw = self.ext_call_parsed("ListWindows", &[]).await?;
        parse_extension_json_windows(&raw)
    }

    async fn window_focus(&self, id: &str) -> anyhow::Result<()> {
        // Pass the user's string directly to the extension's FocusWindow.
        // The extension already does case-insensitive substring matching on both
        // app_id and title, so "code" finds VS Code, "kinsafe" finds the right window.
        // For hex XIDs (0x...), find the window locally first and pass its title.
        if id.starts_with("0x") || id.starts_with("0X") {
            let windows = self.windows_list().await?;
            let target = windows
                .iter()
                .find(|w| w.id == id)
                .ok_or_else(|| anyhow::anyhow!("window not found: {}", id))?;
            self.ext_call_parsed("FocusWindow", &[&target.app_id, &target.title, "false"])
                .await?;
        } else {
            // Direct pass-through: extension matches title fragment
            self.ext_call_parsed("FocusWindow", &[id, id, "false"])
                .await?;
        }
        Ok(())
    }

    async fn window_get(&self, id: &str) -> anyhow::Result<protocol::WindowInfo> {
        let windows = self.windows_list().await?;
        windows
            .into_iter()
            .find(|w| w.id == id || w.app_id == id)
            .ok_or_else(|| anyhow::anyhow!("window not found: {}", id))
    }

    // ═══════════════════════════════════════════════════════
    //  WORKSPACES  (via org.gnome.Shell.Extensions)
    // ═══════════════════════════════════════════════════════

    async fn workspaces_list(&self) -> anyhow::Result<Vec<protocol::WorkspaceInfo>> {
        // Count workspaces from windows list + Mutter for active
        let windows = self.windows_list().await?;
        let max_ws = windows.iter().map(|w| w.workspace_id).max().unwrap_or(0) + 1;
        let current = self.get_current_workspace().await?;
        let workspaces: Vec<protocol::WorkspaceInfo> = (0..max_ws)
            .map(|i| protocol::WorkspaceInfo {
                id: i,
                name: format!("Workspace {}", i + 1),
                is_active: i == current,
            })
            .collect();
        Ok(workspaces)
    }

    async fn workspace_switch(&self, id: u32) -> anyhow::Result<()> {
        // Call extension's SwitchWorkspace(index) over DBus — no Eval needed
        self.ext_call_parsed("SwitchWorkspace", &[&id.to_string()])
            .await?;
        Ok(())
    }

    async fn workspace_move_window(
        &self,
        window_id: &str,
        workspace_id: u32,
        _follow: bool,
    ) -> anyhow::Result<()> {
        // Call extension's MoveWindowToWorkspace(app_id, workspace_index) over DBus
        let windows = self.windows_list().await?;
        let target = windows
            .iter()
            .find(|w| w.id == window_id || w.app_id == window_id)
            .ok_or_else(|| anyhow::anyhow!("window not found: {}", window_id))?;

        self.ext_call_parsed(
            "MoveWindowToWorkspace",
            &[&target.app_id, &workspace_id.to_string()],
        )
        .await?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    //  INPUT  (via Mutter RemoteDesktop compositor pipeline)
    // ═══════════════════════════════════════════════════════

    async fn keyboard_type(&self, text: &str) -> anyhow::Result<()> {
        for c in text.chars() {
            let (keysym, needs_shift) = keysym::from_char(c)
                .ok_or_else(|| anyhow::anyhow!("no keysym for char: {:?}", c))?;
            if needs_shift {
                self.rd_keysym(keysym::SHIFT_L, true).await?;
            }
            self.rd_keysym(keysym, true).await?;
            self.rd_keysym(keysym, false).await?;
            if needs_shift {
                self.rd_keysym(keysym::SHIFT_L, false).await?;
            }
        }
        Ok(())
    }

    async fn keyboard_key(&self, key: &str) -> anyhow::Result<()> {
        let keysym = keysym::from_name(key)
            .or_else(|| {
                key.chars()
                    .next()
                    .and_then(|c| keysym::from_char(c).map(|(k, _)| k))
            })
            .ok_or_else(|| anyhow::anyhow!("unknown key: {}", key))?;
        self.rd_keysym(keysym, true).await?;
        self.rd_keysym(keysym, false).await?;
        Ok(())
    }

    async fn keyboard_combo(&self, keys: &[String]) -> anyhow::Result<()> {
        if keys.is_empty() {
            return Ok(());
        }
        // Press all modifiers, then the final key
        let (modifiers, final_key) = keys.split_at(keys.len().saturating_sub(1));
        let final_key_str = &final_key[0];

        // Resolve all keysyms upfront so we don't fail mid-combo
        let mut modifier_syms: Vec<u32> = Vec::new();
        for k in modifiers {
            let sym = keysym::from_name(k)
                .or_else(|| {
                    k.chars()
                        .next()
                        .and_then(|c| keysym::from_char(c).map(|(s, _)| s))
                })
                .ok_or_else(|| anyhow::anyhow!("unknown modifier: {}", k))?;
            modifier_syms.push(sym);
        }
        let target_sym = keysym::from_name(final_key_str)
            .or_else(|| {
                final_key_str
                    .chars()
                    .next()
                    .and_then(|c| keysym::from_char(c).map(|(s, _)| s))
            })
            .ok_or_else(|| anyhow::anyhow!("unknown key: {}", final_key_str))?;

        // Press modifiers
        for &sym in &modifier_syms {
            self.rd_keysym(sym, true).await?;
        }

        // Press and release the target key
        self.rd_keysym(target_sym, true).await?;
        self.rd_keysym(target_sym, false).await?;

        // Release modifiers in reverse order
        for &sym in modifier_syms.iter().rev() {
            self.rd_keysym(sym, false).await?;
        }
        Ok(())
    }

    async fn mouse_move(&self, x: f64, y: f64) -> anyhow::Result<()> {
        // Use relative motion from the last known position.
        // Absolute positioning requires a ScreenCast stream which we don't have yet.
        let (last_x, last_y) = {
            let pos = self.last_mouse.lock().unwrap();
            *pos
        };
        let dx = x - last_x;
        let dy = y - last_y;

        // Update tracked position
        {
            let mut pos = self.last_mouse.lock().unwrap();
            *pos = (x, y);
        }

        self.rd_call("NotifyPointerMotionRelative", &(dx, dy))
            .await?;
        Ok(())
    }

    async fn mouse_click(&self, button: &str) -> anyhow::Result<()> {
        let btn: i32 = match button {
            "left" => 1,
            "middle" => 2,
            "right" => 3,
            _ => anyhow::bail!("unknown button: {}", button),
        };
        self.rd_button(btn, true).await?;
        self.rd_button(btn, false).await?;
        Ok(())
    }

    async fn mouse_scroll(&self, dx: f64, dy: f64) -> anyhow::Result<()> {
        if dy != 0.0 {
            self.rd_call("NotifyPointerAxisDiscrete", &(0u32, dy as i32))
                .await?;
        }
        if dx != 0.0 {
            self.rd_call("NotifyPointerAxisDiscrete", &(1u32, dx as i32))
                .await?;
        }
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    //  CLIPBOARD
    // ═══════════════════════════════════════════════════════

    async fn clipboard_read(&self) -> anyhow::Result<String> {
        self.sh("wl-paste", &[]).await
    }

    async fn clipboard_write(&self, text: &str) -> anyhow::Result<()> {
        // Pipe text into wl-copy
        let mut child = Command::new("wl-copy")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;
        use tokio::io::AsyncWriteExt;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes()).await?;
        }
        let status = child.wait().await?;
        if !status.success() {
            anyhow::bail!("wl-copy failed");
        }
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    //  SCREENSHOT
    // ═══════════════════════════════════════════════════════

    async fn screenshot(
        &self,
        monitor: Option<u32>,
        region: Option<Region>,
        window_id: Option<String>,
    ) -> anyhow::Result<protocol::ScreenshotResult> {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        let path = format!("/tmp/deskbrid_screenshot_{}.png", ts);

        // If window_id is set, use grim -g based on window geometry
        if let Some(ref wid) = window_id {
            let info = self.window_get(wid).await?;
            if let Some(geo) = info.geometry {
                let region_str = format!("{}x{}+{}+{}", geo.width, geo.height, geo.x, geo.y);
                self.sh("grim", &["-g", &region_str, &path]).await?;
                return Ok(protocol::ScreenshotResult {
                    path: path.clone(),
                    width: geo.width,
                    height: geo.height,
                    format: "png".into(),
                });
            }
        }

        // Region screenshot
        if let Some(ref r) = region {
            let region_str = format!("{}x{}+{}+{}", r.width, r.height, r.x, r.y);
            self.sh("grim", &["-g", &region_str, &path]).await?;
            return Ok(protocol::ScreenshotResult {
                path: path.clone(),
                width: r.width,
                height: r.height,
                format: "png".into(),
            });
        }

        // Full screen (or specific monitor by name)
        if let Some(idx) = monitor {
            // grim expects monitor name (e.g., "DP-1"), not index
            let monitors = self.get_monitors().await?;
            let name = monitors
                .get(idx as usize)
                .map(|m| m.name.clone())
                .unwrap_or_else(|| idx.to_string());
            self.sh("grim", &["-o", &name, &path]).await?;
        } else {
            self.sh("grim", &[&path]).await?;
        }

        // Get dimensions from the file
        let dims = get_png_dimensions(&path)?;
        Ok(protocol::ScreenshotResult {
            path,
            width: dims.0,
            height: dims.1,
            format: "png".into(),
        })
    }

    // ═══════════════════════════════════════════════════════
    //  NOTIFICATIONS
    // ═══════════════════════════════════════════════════════

    async fn notification_send(
        &self,
        app_name: &str,
        title: &str,
        body: &str,
        urgency: &str,
    ) -> anyhow::Result<u32> {
        let urgency_byte = match urgency {
            "low" => 0u8,
            "normal" => 1u8,
            "critical" => 2u8,
            _ => 1u8,
        };

        // org.freedesktop.Notifications.Notify(
        //   app_name, replaces_id, app_icon, summary, body,
        //   actions, hints, expire_timeout
        // ) → u32 (notification ID)
        let reply = self
            .conn
            .call_method(
                Some("org.freedesktop.Notifications"),
                "/org/freedesktop/Notifications",
                Some("org.freedesktop.Notifications"),
                "Notify",
                &(
                    app_name,
                    0u32,           // replaces_id
                    "",             // app_icon
                    title,          // summary
                    body,           // body
                    &[] as &[&str], // actions
                    &[("urgency", zvariant::Value::U8(urgency_byte))]
                        as &[(&str, zvariant::Value)],
                    5000i32, // expire_timeout ms (-1 = default)
                ),
            )
            .await?;
        let id: u32 = reply.body().deserialize()?;
        Ok(id)
    }

    async fn notification_close(&self, id: u32) -> anyhow::Result<()> {
        self.conn
            .call_method(
                Some("org.freedesktop.Notifications"),
                "/org/freedesktop/Notifications",
                Some("org.freedesktop.Notifications"),
                "CloseNotification",
                &(id,),
            )
            .await?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    //  SYSTEM
    // ═══════════════════════════════════════════════════════

    async fn system_info(&self) -> anyhow::Result<protocol::SystemInfo> {
        let _hostname = self.sh("hostname", &[]).await.unwrap_or_default();
        let version = self
            .sh("gnome-shell", &["--version"])
            .await
            .unwrap_or_else(|_| "unknown".into());
        // gnome-shell --version outputs "GNOME Shell 46.2" or similar
        let version = version
            .strip_prefix("GNOME Shell ")
            .unwrap_or(&version)
            .to_string();

        // Detect session type
        let session_type = if std::env::var("WAYLAND_DISPLAY").is_ok() {
            "wayland"
        } else if std::env::var("DISPLAY").is_ok() {
            "x11"
        } else {
            "unknown"
        };

        // Monitor info from Mutter DBus
        let monitors = self.get_monitors().await?;
        let workspace_count = self.get_workspace_count().await?;
        let current_workspace = self.get_current_workspace().await?;
        let idle_seconds = self.idle_seconds_inner().await.unwrap_or(0);

        Ok(protocol::SystemInfo {
            desktop: "GNOME".into(),
            desktop_version: version,
            compositor: "mutter".into(),
            session_type: session_type.into(),
            monitors,
            workspace_count,
            current_workspace,
            idle_seconds,
        })
    }

    async fn idle_seconds(&self) -> anyhow::Result<u64> {
        self.idle_seconds_inner().await
    }

    async fn power_action(&self, action: &str) -> anyhow::Result<()> {
        match action {
            "suspend" => {
                self.sh("systemctl", &["suspend"]).await?;
            }
            "hibernate" => {
                self.sh("systemctl", &["hibernate"]).await?;
            }
            "shutdown" | "poweroff" => {
                self.sh("systemctl", &["poweroff"]).await?;
            }
            "reboot" | "restart" => {
                self.sh("systemctl", &["reboot"]).await?;
            }
            "lock" => {
                // GNOME 46: use loginctl lock-session
                self.sh("loginctl", &["lock-session"]).await?;
            }
            "logout" => {
                self.sh("gnome-session-quit", &["--logout", "--no-prompt"])
                    .await?;
            }
            _ => anyhow::bail!("unsupported power action: {}", action),
        }
        Ok(())
    }

    async fn battery_status(&self) -> anyhow::Result<Vec<protocol::BatteryInfo>> {
        // Query UPower devices
        let reply = self
            .conn
            .call_method(
                Some("org.freedesktop.UPower"),
                "/org/freedesktop/UPower",
                Some("org.freedesktop.UPower"),
                "EnumerateDevices",
                &(),
            )
            .await?;
        let paths: Vec<zvariant::OwnedObjectPath> = reply.body().deserialize()?;

        let mut batteries = Vec::new();
        for path in &paths {
            let path_str = path.as_str();
            // Only process battery devices
            let type_reply = self
                .conn
                .call_method(
                    Some("org.freedesktop.UPower"),
                    path_str,
                    Some("org.freedesktop.DBus.Properties"),
                    "Get",
                    &("org.freedesktop.UPower.Device", "Type"),
                )
                .await;

            if let Ok(reply) = type_reply {
                let type_val: u32 = reply.body().deserialize().unwrap_or(0);
                if type_val != 2 {
                    // 2 = Battery
                    continue;
                }
            } else {
                continue;
            }

            // Get percentage and state
            let pct: f64 = self
                .get_upower_property(path_str, "Percentage")
                .await
                .unwrap_or(0.0);
            let state_val: u32 = self
                .get_upower_property(path_str, "State")
                .await
                .unwrap_or(0);
            let energy_rate: f64 = self
                .get_upower_property(path_str, "EnergyRate")
                .await
                .unwrap_or(0.0);
            let energy: f64 = self
                .get_upower_property(path_str, "Energy")
                .await
                .unwrap_or(0.0);

            let state = match state_val {
                1 => "charging",
                2 => "discharging",
                4 => "fully_charged",
                _ => "unknown",
            };

            let time_remaining = if state == "discharging" && energy_rate > 0.0 {
                Some(((energy / energy_rate) * 60.0) as u32)
            } else if state == "charging" && energy_rate > 0.0 {
                let remaining_energy = energy * (100.0 - pct) / 100.0;
                Some(((remaining_energy / energy_rate) * 60.0) as u32)
            } else {
                None
            };

            batteries.push(protocol::BatteryInfo {
                source: path_str.to_string(),
                percentage: pct,
                state: state.into(),
                time_remaining_minutes: time_remaining,
            });
        }

        Ok(batteries)
    }

    // ═══════════════════════════════════════════════════════
    //  NETWORK
    // ═══════════════════════════════════════════════════════

    async fn network_status(&self) -> anyhow::Result<protocol::NetworkStatusInfo> {
        // Query NM's State property for real connectivity status
        let online = match self
            .conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                "/org/freedesktop/NetworkManager",
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &("org.freedesktop.NetworkManager", "State"),
            )
            .await
        {
            Ok(reply) => {
                let state: u32 = reply.body().deserialize().unwrap_or(0);
                // NM_STATE_CONNECTED_GLOBAL = 70
                state >= 70
            }
            Err(_) => {
                // NM not available, fall back to ping
                self.sh_ok("ping", &["-c", "1", "-W", "2", "8.8.8.8"]).await
            }
        };

        Ok(protocol::NetworkStatusInfo {
            online,
            net_type: if online {
                "ethernet_or_wifi".into()
            } else {
                "offline".into()
            },
        })
    }

    async fn network_interfaces(&self) -> anyhow::Result<Vec<protocol::NetworkInterfaceInfo>> {
        // Get devices from NetworkManager
        let reply = match self
            .conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                "/org/freedesktop/NetworkManager",
                Some("org.freedesktop.NetworkManager"),
                "GetDevices",
                &(),
            )
            .await
        {
            Ok(r) => r,
            Err(_) => {
                // NM not running, parse /proc/net/dev
                let out = self.sh("cat", &["/proc/net/dev"]).await.unwrap_or_default();
                let mut ifaces = Vec::new();
                for line in out.lines().skip(2) {
                    let name = line.split(':').next().unwrap_or("").trim();
                    if name.is_empty() || name == "lo" {
                        continue;
                    }
                    ifaces.push(protocol::NetworkInterfaceInfo {
                        name: name.to_string(),
                        state: "up".into(),
                        ipv4: None,
                        ipv6: None,
                    });
                }
                return Ok(ifaces);
            }
        };

        let paths: Vec<zvariant::OwnedObjectPath> = reply.body().deserialize()?;
        let mut ifaces = Vec::new();

        for path in &paths {
            let path_str = path.as_str();

            // Get interface name, state, and IP config via GetAll
            let props: std::collections::HashMap<String, zvariant::OwnedValue> = match self
                .conn
                .call_method(
                    Some("org.freedesktop.NetworkManager"),
                    path_str,
                    Some("org.freedesktop.DBus.Properties"),
                    "GetAll",
                    &("org.freedesktop.NetworkManager.Device",),
                )
                .await
            {
                Ok(r) => r.body().deserialize().unwrap_or_default(),
                Err(_) => continue,
            };

            let name = if let Some(v) = props.get("Interface") {
                if let Ok(s) = v.downcast_ref::<zvariant::Str>() {
                    s.to_string()
                } else {
                    path_str.to_string()
                }
            } else {
                path_str.to_string()
            };

            let state_num: u32 = props
                .get("State")
                .and_then(|v| v.downcast_ref::<u32>().ok())
                .unwrap_or(0);
            let state = match state_num {
                100 => "connected",
                70 => "connecting",
                50 | 60 => "disconnected",
                _ => "unknown",
            };

            // Get IPv4 address from IP4Config
            let ipv4 = match props.get("Ip4Config") {
                Some(v) => {
                    if let Ok(obj_path) = v.downcast_ref::<zvariant::ObjectPath>() {
                        self.get_nm_ip4_address(obj_path.as_str()).await
                    } else {
                        None
                    }
                }
                None => None,
            };

            ifaces.push(protocol::NetworkInterfaceInfo {
                name,
                state: state.into(),
                ipv4,
                ipv6: None,
            });
        }

        Ok(ifaces)
    }

    async fn wifi_scan(&self) -> anyhow::Result<Vec<protocol::WifiNetworkInfo>> {
        // Get WiFi device paths
        let reply = self
            .conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                "/org/freedesktop/NetworkManager",
                Some("org.freedesktop.NetworkManager"),
                "GetDevices",
                &(),
            )
            .await?;
        let all_paths: Vec<zvariant::OwnedObjectPath> = reply.body().deserialize()?;

        let mut networks = Vec::new();

        for path in &all_paths {
            let path_str = path.as_str();

            // Check device type (2 = WiFi)
            let device_type: u32 = match self.get_nm_property(path_str, "DeviceType").await {
                Ok(t) => t,
                Err(_) => continue,
            };
            if device_type != 2 {
                continue;
            }

            // Request a scan
            let _ = self
                .conn
                .call_method(
                    Some("org.freedesktop.NetworkManager"),
                    path_str,
                    Some("org.freedesktop.NetworkManager.Device.Wireless"),
                    "RequestScan",
                    &(std::collections::HashMap::<&str, zvariant::Value>::new(),),
                )
                .await;

            // Get access points
            let ap_reply = self
                .conn
                .call_method(
                    Some("org.freedesktop.NetworkManager"),
                    path_str,
                    Some("org.freedesktop.NetworkManager.Device.Wireless"),
                    "GetAccessPoints",
                    &(),
                )
                .await?;
            let ap_paths: Vec<zvariant::OwnedObjectPath> = ap_reply.body().deserialize()?;

            for ap_path in &ap_paths {
                let props: std::collections::HashMap<String, zvariant::OwnedValue> = match self
                    .conn
                    .call_method(
                        Some("org.freedesktop.NetworkManager"),
                        ap_path.as_str(),
                        Some("org.freedesktop.DBus.Properties"),
                        "GetAll",
                        &("org.freedesktop.NetworkManager.AccessPoint",),
                    )
                    .await
                {
                    Ok(r) => r.body().deserialize().unwrap_or_default(),
                    Err(_) => continue,
                };

                // SSID is a byte array
                let ssid = if let Some(v) = props.get("Ssid") {
                    if let Ok(arr) = v.downcast_ref::<zvariant::Array>() {
                        let bytes: Vec<u8> = arr
                            .iter()
                            .filter_map(|v| v.downcast_ref::<u8>().ok())
                            .collect();
                        String::from_utf8_lossy(&bytes).to_string()
                    } else {
                        "(hidden)".into()
                    }
                } else {
                    "(hidden)".into()
                };

                let strength: u32 = props
                    .get("Strength")
                    .and_then(|v| v.downcast_ref::<u8>().ok())
                    .map(|s| s as u32)
                    .unwrap_or(0);

                let flags: u32 = props
                    .get("Flags")
                    .and_then(|v| v.downcast_ref::<u32>().ok())
                    .unwrap_or(0);
                // NM 80211ApFlags: 0x1 = privacy (WEP/WPA)
                let secured = (flags & 0x1) != 0;

                let frequency: Option<u32> = props
                    .get("Frequency")
                    .and_then(|v| v.downcast_ref::<u32>().ok());

                networks.push(protocol::WifiNetworkInfo {
                    ssid,
                    strength,
                    secured,
                    frequency,
                });
            }
        }

        Ok(networks)
    }

    async fn wifi_connect(&self, ssid: &str, password: Option<&str>) -> anyhow::Result<()> {
        // Use nmcli for reliable connection setup (NM DBus ActivateConnection is complex —
        // needs a full connection profile with settings dict). nmcli handles all of that.
        let mut args = vec!["device", "wifi", "connect", ssid];
        if let Some(pw) = password {
            args.push("password");
            args.push(pw);
        }
        self.sh("nmcli", &args).await?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    //  BLUETOOTH
    // ═══════════════════════════════════════════════════════

    async fn bluetooth_list(&self) -> anyhow::Result<Vec<protocol::BluetoothDeviceInfo>> {
        let reply = self
            .conn
            .call_method(
                Some("org.bluez"),
                "/",
                Some("org.freedesktop.DBus.ObjectManager"),
                "GetManagedObjects",
                &(),
            )
            .await?;

        // Returns a{oa{sa{sv}}}
        let managed: std::collections::HashMap<
            zvariant::OwnedObjectPath,
            std::collections::HashMap<String, zvariant::OwnedValue>,
        > = reply.body().deserialize()?;

        let mut devices = Vec::new();

        for ifaces in managed.values() {
            // Only process objects that have the Device1 interface
            if !ifaces.contains_key("org.bluez.Device1") {
                continue;
            }
            // Get device properties from the Device1 interface dict
            // The value is an a{sv} map of properties
            let props = if let Some(v) = ifaces.get("org.bluez.Device1") {
                if let Ok(map) = v.downcast_ref::<zvariant::Dict>() {
                    map
                } else {
                    continue;
                }
            } else {
                continue;
            };

            let mut address = String::new();
            let mut name = "(unknown)".to_string();
            let mut paired = false;
            let mut connected = false;
            let mut rssi: Option<i32> = None;

            for (prop_key, prop_val) in props.iter() {
                // Dict keys are zvariant Values — downcast to string for comparison
                let key_str = if let Ok(s) = prop_key.downcast_ref::<zvariant::Str>() {
                    s.to_string()
                } else {
                    continue;
                };
                match key_str.as_str() {
                    "Address" => {
                        if let Ok(s) = prop_val.downcast_ref::<zvariant::Str>() {
                            address = s.to_string();
                        }
                    }
                    "Name" => {
                        if let Ok(s) = prop_val.downcast_ref::<zvariant::Str>() {
                            name = s.to_string();
                        }
                    }
                    "Paired" => {
                        if let Ok(b) = prop_val.downcast_ref::<bool>() {
                            paired = b;
                        }
                    }
                    "Connected" => {
                        if let Ok(b) = prop_val.downcast_ref::<bool>() {
                            connected = b;
                        }
                    }
                    "RSSI" => {
                        if let Ok(v) = prop_val.downcast_ref::<i16>() {
                            rssi = Some(v as i32);
                        }
                    }
                    _ => {}
                }
            }

            devices.push(protocol::BluetoothDeviceInfo {
                address,
                name,
                paired,
                connected,
                rssi,
            });
        }

        Ok(devices)
    }

    async fn bluetooth_scan(&self, _duration: Option<u32>) -> anyhow::Result<()> {
        // Find the first adapter and start discovery
        let adapter_path = self.find_bluetooth_adapter().await?;
        self.conn
            .call_method(
                Some("org.bluez"),
                adapter_path.as_str(),
                Some("org.bluez.Adapter1"),
                "StartDiscovery",
                &(),
            )
            .await?;
        Ok(())
    }

    async fn bluetooth_stop_scan(&self) -> anyhow::Result<()> {
        if let Ok(adapter_path) = self.find_bluetooth_adapter().await {
            let _ = self
                .conn
                .call_method(
                    Some("org.bluez"),
                    adapter_path.as_str(),
                    Some("org.bluez.Adapter1"),
                    "StopDiscovery",
                    &(),
                )
                .await;
        }
        Ok(())
    }

    async fn bluetooth_connect(&self, address: &str) -> anyhow::Result<()> {
        let path = self.device_path(address);
        self.conn
            .call_method(
                Some("org.bluez"),
                path.as_str(),
                Some("org.bluez.Device1"),
                "Connect",
                &(),
            )
            .await?;
        Ok(())
    }

    async fn bluetooth_disconnect(&self, address: &str) -> anyhow::Result<()> {
        let path = self.device_path(address);
        // Disconnect may fail if not connected — that's fine for our purposes
        let _ = self
            .conn
            .call_method(
                Some("org.bluez"),
                path.as_str(),
                Some("org.bluez.Device1"),
                "Disconnect",
                &(),
            )
            .await;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    //  FILES
    // ═══════════════════════════════════════════════════════

    async fn files_watch(
        &self,
        path: &str,
        recursive: bool,
        _patterns: Option<&[String]>,
    ) -> anyhow::Result<()> {
        let meta = tokio::fs::metadata(path).await?;
        if !meta.is_dir() && !meta.is_file() {
            anyhow::bail!("path does not exist: {}", path);
        }

        let path_owned = path.to_string();
        let event_tx = self.event_tx.clone();

        // Create a notify watcher for this path
        let mode = if recursive {
            notify::RecursiveMode::Recursive
        } else {
            notify::RecursiveMode::NonRecursive
        };

        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                if let Ok(event) = event {
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    for path in &event.paths {
                        let path_str = path.to_string_lossy().to_string();
                        let evt = match event.kind {
                            notify::EventKind::Create(_) => DeskbridEvent::FileCreated {
                                path: path_str,
                                timestamp: ts,
                            },
                            notify::EventKind::Modify(_) => DeskbridEvent::FileModified {
                                path: path_str,
                                timestamp: ts,
                            },
                            notify::EventKind::Remove(_) => DeskbridEvent::FileDeleted {
                                path: path_str,
                                timestamp: ts,
                            },
                            _ => continue,
                        };
                        let _ = event_tx.send(evt);
                    }
                }
            })?;

        watcher.watch(std::path::Path::new(&path_owned), mode)?;

        // Store the watcher so it stays alive
        let mut guard = self.watchers.lock().unwrap();
        guard.insert(path_owned.clone(), watcher);

        debug!(
            "File watch active on {} (recursive={})",
            path_owned, recursive
        );
        Ok(())
    }

    async fn files_unwatch(&self, path: &str) -> anyhow::Result<()> {
        let mut guard = self.watchers.lock().unwrap();
        guard.remove(path);
        debug!("File watch removed on {}", path);
        Ok(())
    }

    async fn files_search(
        &self,
        pattern: &str,
        root: Option<&str>,
        max_results: u32,
    ) -> anyhow::Result<Vec<String>> {
        let base = root.unwrap_or(".");
        // Use fd if available, fall back to find
        if self.sh_ok("which", &["fd"]).await {
            let out = self
                .sh(
                    "fd",
                    &[
                        "--max-results",
                        &max_results.to_string(),
                        "--search-path",
                        base,
                        pattern,
                    ],
                )
                .await?;
            Ok(out.lines().map(|l| l.to_string()).collect())
        } else {
            let out = self
                .sh("find", &[base, "-name", pattern, "-maxdepth", "10"])
                .await?;
            let lines: Vec<String> = out
                .lines()
                .take(max_results as usize)
                .map(|l| l.to_string())
                .collect();
            Ok(lines)
        }
    }

    // ═══════════════════════════════════════════════════════
    //  AUDIO
    // ═══════════════════════════════════════════════════════

    async fn audio_list_sinks(&self) -> anyhow::Result<Vec<protocol::AudioSinkInfo>> {
        // Use pactl for PipeWire-PulseAudio compat
        let out = self.sh("pactl", &["list", "sinks"]).await?;
        parse_pactl_sinks(&out)
    }

    async fn audio_set_sink_volume(&self, sink_id: u32, volume: f64) -> anyhow::Result<()> {
        // pactl set-sink-volume <id> <volume>%
        let vol_pct = (volume * 100.0) as u32;
        self.sh(
            "pactl",
            &[
                "set-sink-volume",
                &sink_id.to_string(),
                &format!("{}%", vol_pct),
            ],
        )
        .await?;
        Ok(())
    }
}

// ─── Private helpers ─────────────────────────────────────

impl GnomeBackend {
    async fn idle_seconds_inner(&self) -> anyhow::Result<u64> {
        let reply = self
            .conn
            .call_method(
                Some("org.gnome.Mutter.IdleMonitor"),
                "/org/gnome/Mutter/IdleMonitor/Core",
                Some("org.gnome.Mutter.IdleMonitor"),
                "GetIdletime",
                &(),
            )
            .await?;
        let ms: u64 = reply.body().deserialize()?;
        Ok(ms / 1000)
    }

    async fn get_monitors(&self) -> anyhow::Result<Vec<protocol::MonitorInfo>> {
        // Try wlr-randr, fall back to placeholder
        let mut monitors = Vec::new();
        // Try wlr-randr (wlroots-based but sometimes available)
        if let Ok(out) = self.sh("wlr-randr", &[]).await {
            let mut current_name = String::new();
            let mut current_width = 1920u32;
            let mut current_height = 1080u32;
            let mut current_scale = 1.0f64;
            let mut idx = 0u32;

            for line in out.lines() {
                if !line.starts_with(' ') && !line.is_empty() {
                    // Header line, save previous
                    if !current_name.is_empty() {
                        monitors.push(protocol::MonitorInfo {
                            id: idx,
                            name: current_name.clone(),
                            width: current_width,
                            height: current_height,
                            scale: current_scale,
                            primary: idx == 0,
                        });
                        idx += 1;
                    }
                    current_name = line.split(' ').next().unwrap_or("").to_string();
                }
                if line.contains("current") {
                    // "   1920x1080 @ 60Hz"
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if let Some(res) = parts.first() {
                        let dims: Vec<&str> = res.split('x').collect();
                        if dims.len() == 2 {
                            current_width = dims[0].parse().unwrap_or(1920);
                            current_height = dims[1]
                                .split('@')
                                .next()
                                .unwrap_or("1080")
                                .parse()
                                .unwrap_or(1080);
                        }
                    }
                }
                if line.contains("Scale:") {
                    current_scale = line
                        .split("Scale:")
                        .nth(1)
                        .unwrap_or("1.0")
                        .trim()
                        .parse()
                        .unwrap_or(1.0);
                }
            }
            if !current_name.is_empty() {
                monitors.push(protocol::MonitorInfo {
                    id: idx,
                    name: current_name,
                    width: current_width,
                    height: current_height,
                    scale: current_scale,
                    primary: idx == 0,
                });
            }
            if !monitors.is_empty() {
                return Ok(monitors);
            }
        }

        // Absolute fallback: single 1920x1080 monitor
        monitors.push(protocol::MonitorInfo {
            id: 0,
            name: "Unknown".into(),
            width: 1920,
            height: 1080,
            scale: 1.0,
            primary: true,
        });
        Ok(monitors)
    }

    async fn get_workspace_count(&self) -> anyhow::Result<u32> {
        if let Ok(raw) = self.ext_call_parsed("WorkspacesList", &[]).await {
            // Count the tuples in the array
            let count = raw.matches("('").count() as u32;
            if count > 0 {
                return Ok(count);
            }
        }
        Ok(1)
    }

    async fn get_current_workspace(&self) -> anyhow::Result<u32> {
        if let Ok(raw) = self.ext_call_parsed("ActiveWorkspace", &[]).await {
            // Returns something like "(uint32 0,)"
            if let Some(start) = raw.find("uint32 ") {
                let num_str = &raw[start + 7..];
                if let Some(end) = num_str.find(|c: char| !c.is_ascii_digit()) {
                    return Ok(num_str[..end].parse().unwrap_or(0));
                }
            }
        }
        Ok(0)
    }

    async fn get_upower_property<T: serde::de::DeserializeOwned + zbus::zvariant::Type>(
        &self,
        path: &str,
        prop: &str,
    ) -> anyhow::Result<T> {
        let reply = self
            .conn
            .call_method(
                Some("org.freedesktop.UPower"),
                path,
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &("org.freedesktop.UPower.Device", prop),
            )
            .await?;
        let val: T = reply.body().deserialize()?;
        Ok(val)
    }

    /// Get a NetworkManager Device property by name.
    async fn get_nm_property<T: serde::de::DeserializeOwned + zbus::zvariant::Type>(
        &self,
        path: &str,
        prop: &str,
    ) -> anyhow::Result<T> {
        let reply = self
            .conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                path,
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &("org.freedesktop.NetworkManager.Device", prop),
            )
            .await?;
        let val: T = reply.body().deserialize()?;
        Ok(val)
    }

    /// Get the first IPv4 address from an IP4Config object path.
    async fn get_nm_ip4_address(&self, config_path: &str) -> Option<String> {
        let reply = self
            .conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                config_path,
                Some("org.freedesktop.DBus.Properties"),
                "GetAll",
                &("org.freedesktop.NetworkManager.IP4Config",),
            )
            .await
            .ok()?;
        let props: std::collections::HashMap<String, zvariant::OwnedValue> =
            reply.body().deserialize().ok()?;

        // AddressData is aav — array of (address, prefix, gateway) tuples
        let addresses = props.get("AddressData")?;
        let arr = addresses.downcast_ref::<zvariant::Array>().ok()?;
        for entry in arr.iter() {
            if let Ok(inner) = entry.downcast_ref::<zvariant::Structure>() {
                let fields = inner.fields();
                let addr = if let Some(v) = fields.first() {
                    if let Ok(s) = v.downcast_ref::<zvariant::Str>() {
                        Some(s.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some(a) = addr {
                    return Some(a);
                }
            }
        }
        None
    }

    /// Find the first BlueZ adapter path (e.g., /org/bluez/hci0).
    async fn find_bluetooth_adapter(&self) -> anyhow::Result<String> {
        let reply = self
            .conn
            .call_method(
                Some("org.bluez"),
                "/",
                Some("org.freedesktop.DBus.ObjectManager"),
                "GetManagedObjects",
                &(),
            )
            .await?;

        let managed: std::collections::HashMap<
            zvariant::OwnedObjectPath,
            std::collections::HashMap<String, zvariant::OwnedValue>,
        > = reply.body().deserialize()?;

        for (path, ifaces) in &managed {
            if ifaces.contains_key("org.bluez.Adapter1") {
                return Ok(path.as_str().to_string());
            }
        }
        anyhow::bail!("no Bluetooth adapter found")
    }

    /// Convert a Bluetooth address (XX:XX:XX:XX:XX:XX) to a BlueZ device path.
    fn device_path(&self, address: &str) -> String {
        let normalized = address.replace(':', "_").to_uppercase();
        format!("/org/bluez/hci0/dev_{}", normalized)
    }
}

// ─── Extension JSON parser ────────────────────────────────

/// Parse the JSON string returned by the extension's ListWindows() method.
fn parse_extension_json_windows(raw: &str) -> anyhow::Result<Vec<protocol::WindowInfo>> {
    // gdbus wraps the return like: ('[{...},{...}]',)
    let inner = raw.trim().trim_start_matches('(').trim_end_matches(')');
    // Now inner is: '[json]',  — strip leading quote and trailing ',
    let json_str = inner
        .trim()
        .trim_start_matches('\'')
        .trim_end_matches(',')
        .trim()
        .trim_end_matches('\'');
    let parsed: Vec<serde_json::Value> = serde_json::from_str(json_str)?;

    let windows: Vec<protocol::WindowInfo> = parsed
        .into_iter()
        .map(|w| protocol::WindowInfo {
            id: w["id"]
                .as_u64()
                .map(|n| n.to_string())
                .unwrap_or_else(|| w["id"].as_str().unwrap_or("").to_string()),
            title: w["title"].as_str().unwrap_or("").to_string(),
            app_id: w["app_id"].as_str().unwrap_or("").to_string(),
            workspace_id: w["workspace_index"].as_u64().unwrap_or(0) as u32,
            is_focused: w["focused"].as_bool().unwrap_or(false),
            is_minimized: w["minimized"].as_bool().unwrap_or(false),
            geometry: {
                let geo = &w["geometry"];
                if let Some(arr) = geo.as_array() {
                    let x = arr.first().and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                    let y = arr.get(1).and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                    let width = arr.get(2).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    let height = arr.get(3).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    Some(Geometry {
                        x,
                        y,
                        width,
                        height,
                    })
                } else {
                    None
                }
            },
            pid: w["pid"].as_u64().map(|p| p as u32),
        })
        .collect();
    Ok(windows)
}

// ─── Screenshot helpers ─────────────────────────────────

fn get_png_dimensions(path: &str) -> anyhow::Result<(u32, u32)> {
    let data = std::fs::read(path)?;
    if data.len() < 24 {
        anyhow::bail!("PNG file too small");
    }
    let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    Ok((width, height))
}

// ─── Audio parsers ────────────────────────────────────

fn parse_pactl_sinks(raw: &str) -> anyhow::Result<Vec<protocol::AudioSinkInfo>> {
    let mut sinks = Vec::new();
    let mut current_id = 0u32;
    let mut current_name = String::new();
    let mut current_desc = String::new();
    let mut current_volume = 0.0f64;
    let mut current_muted = false;
    let mut in_sink = false;

    for line in raw.lines() {
        if line.starts_with("Sink #") {
            if in_sink {
                sinks.push(protocol::AudioSinkInfo {
                    id: current_id,
                    name: current_name.clone(),
                    description: current_desc.clone(),
                    volume: current_volume,
                    muted: current_muted,
                });
            }
            in_sink = true;
            current_name.clear();
            current_desc.clear();
            current_volume = 0.0;
            current_muted = false;
            // "Sink #0"
            current_id = line
                .strip_prefix("Sink #")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        } else if in_sink {
            let trimmed = line.trim();
            if trimmed.starts_with("Name: ") {
                current_name = trimmed.strip_prefix("Name: ").unwrap_or("").to_string();
            } else if trimmed.starts_with("Description: ") {
                current_desc = trimmed
                    .strip_prefix("Description: ")
                    .unwrap_or("")
                    .to_string();
            } else if trimmed.starts_with("Mute: ") {
                current_muted = trimmed.contains("yes");
            } else if trimmed.starts_with("Volume:") {
                // "Volume: front-left: 32768 /  50% / -18.06 dB, ..."
                if let Some(pct) = trimmed.split('/').nth(1) {
                    current_volume = pct
                        .trim()
                        .trim_end_matches('%')
                        .parse::<f64>()
                        .unwrap_or(0.0)
                        / 100.0;
                }
            }
        }
    }
    if in_sink {
        sinks.push(protocol::AudioSinkInfo {
            id: current_id,
            name: current_name,
            description: current_desc,
            volume: current_volume,
            muted: current_muted,
        });
    }
    Ok(sinks)
}
