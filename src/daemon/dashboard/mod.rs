use crate::DaemonState;
use std::sync::Arc;
use tracing::{error, info, warn};

const DASHBOARD_PORT: u16 = 20129;

mod render_data;
mod server;

use render_data::{
    render_audit, render_clipboard, render_macros, render_notifications, render_rules,
    render_sessions,
};

pub async fn start(state: Arc<DaemonState>) {
    let addr = format!("0.0.0.0:{}", DASHBOARD_PORT);
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("Dashboard bind {}: {}", addr, e);
            return;
        }
    };
    info!("Dashboard: http://{}", addr);
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let state = Arc::clone(&state);
                tokio::spawn(async move {
                    if let Err(e) = server::handle_request(stream, state).await {
                        warn!("Dashboard: {}", e);
                    }
                });
            }
            Err(e) => error!("Dashboard accept: {}", e),
        }
    }
}

// ── Card renderers (system state) ────────────────────────

pub(super) fn render_system(info: &Option<crate::protocol::SystemInfo>) -> String {
    let Some(info) = info else {
        return r#"<div class="empty">No backend</div>"#.into();
    };
    let mut rows = String::new();
    rows.push_str(&kv("Desktop", &info.desktop));
    rows.push_str(&kv("Version", &info.desktop_version));
    rows.push_str(&kv("Compositor", &info.compositor));
    rows.push_str(&kv("Session", &info.session_type));
    rows.push_str(&kv(
        "Workspace",
        &format!("{}/{}", info.current_workspace, info.workspace_count),
    ));
    rows.push_str(&kv("Idle", &format!("{}s", info.idle_seconds)));
    rows
}

pub(super) async fn render_desktop_settings(
    backend: &Option<Box<dyn crate::backend::DesktopBackend>>,
) -> String {
    let Some(backend) = backend else {
        return r#"<div class="empty">No backend</div>"#.into();
    };
    match backend.desktop_list_schemas().await {
        Ok(schemas) => {
            if schemas.is_empty() {
                return r#"<div class="empty">No schemas</div>"#.into();
            }
            let mut rows = String::new();
            for s in schemas.iter().take(8) {
                rows.push_str(&kv(s, ""));
            }
            if schemas.len() > 8 {
                rows.push_str(&format!(
                    r#"<div class="empty">… and {} more</div>"#,
                    schemas.len() - 8
                ));
            }
            rows
        }
        Err(_) => r#"<div class="empty">Not supported</div>"#.into(),
    }
}

pub(super) fn render_backlight(info: &Option<crate::protocol::BacklightInfo>) -> String {
    let Some(info) = info else {
        return r#"<div class="empty">No backlight</div>"#.into();
    };
    let bar = volume_bar(info.percentage);
    let mut rows = String::new();
    rows.push_str(&kv("Device", &info.device));
    rows.push_str(&kv(
        "Brightness",
        &format!(
            "{} {}% ({}/{})",
            bar, info.percentage, info.brightness, info.max_brightness
        ),
    ));
    rows
}

pub(super) fn render_monitors(info: &Option<crate::protocol::SystemInfo>) -> String {
    let Some(info) = info else {
        return r#"<div class="empty">No backend</div>"#.into();
    };
    let mut rows = String::new();
    for m in &info.monitors {
        let star = if m.primary { " ⭐" } else { "" };
        let scale = if m.scale != 1.0 {
            format!(" ({}x)", m.scale)
        } else {
            String::new()
        };
        let hz = m
            .refresh_rate
            .map(|r| format!("{:.0}Hz", r))
            .unwrap_or_else(|| "?".into());
        rows.push_str(&kv(
            &format!("Monitor {}", m.id),
            &format!("{}x{} @ {}{}{}", m.width, m.height, hz, scale, star),
        ));
    }
    if rows.is_empty() {
        r#"<div class="empty">No monitors</div>"#.into()
    } else {
        rows
    }
}

pub(super) async fn render_network() -> String {
    use tokio::process::Command;
    let status = Command::new("nmcli")
        .args(["-t", "-f", "STATE", "general", "status"])
        .output()
        .await;
    let mut rows = match status {
        Ok(o) if o.status.success() => {
            let state = String::from_utf8_lossy(&o.stdout).trim().to_string();
            kv("State", &state)
        }
        _ => return r#"<div class="empty">nmcli unavailable</div>"#.into(),
    };
    if let Ok(o) = Command::new("nmcli")
        .args([
            "-t",
            "-f",
            "DEVICE,TYPE,STATE,CONNECTION",
            "device",
            "status",
        ])
        .output()
        .await
        && o.status.success()
    {
        for line in String::from_utf8_lossy(&o.stdout).lines().take(4) {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 4 {
                rows.push_str(&kv(
                    parts[0],
                    &format!("{} — {} ({})", parts[1], parts[2], parts[3]),
                ));
            }
        }
    }
    rows
}

pub(super) async fn render_audio() -> String {
    use tokio::process::Command;
    let out = Command::new("pactl")
        .args(["get-default-sink"])
        .output()
        .await;
    let sink = match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => return r#"<div class="empty">PipeWire/PulseAudio unavailable</div>"#.into(),
    };
    let vol_out = Command::new("pactl")
        .args(["get-sink-volume", &sink])
        .output()
        .await;
    let mut rows = kv("Sink", &sink);
    if let Ok(o) = vol_out
        && o.status.success()
    {
        let txt = String::from_utf8_lossy(&o.stdout);
        if let Some(vol) = txt.split('/').nth(1) {
            let pct: i32 = vol.trim().trim_end_matches('%').parse().unwrap_or(0);
            rows.push_str(&kv(
                "Volume",
                &format!("{} {}%", volume_bar(pct as u8), pct),
            ));
        }
    }
    let mute_out = Command::new("pactl")
        .args(["get-sink-mute", &sink])
        .output()
        .await;
    if let Ok(o) = mute_out
        && o.status.success()
    {
        let txt = String::from_utf8_lossy(&o.stdout);
        let muted = txt.contains("yes");
        rows.push_str(&kv("Muted", if muted { "🔇 Yes" } else { "🔊 No" }));
    }
    rows
}

pub(super) async fn render_windows(
    backend: &Option<Box<dyn crate::backend::DesktopBackend>>,
) -> String {
    let Some(backend) = backend else {
        return r#"<div class="empty">No backend</div>"#.into();
    };
    match backend.windows_list().await {
        Ok(windows) => {
            if windows.is_empty() {
                return r#"<div class="empty">No windows</div>"#.into();
            }
            let mut rows = String::new();
            for w in windows.iter().take(30) {
                let fc = if w.is_focused { "window-focused" } else { "" };
                let min = if w.is_minimized { " 🗕" } else { "" };
                let title = if w.title.is_empty() {
                    &w.app_id
                } else {
                    &w.title
                };
                rows.push_str(&format!(
                    r#"<div class="window-row"><span class="window-icon">🪟</span><span class="window-title {fc}">{t}{min}</span><span class="window-ws">WS{ws}</span></div>"#,
                    fc = fc,
                    t = html_escape(title),
                    min = min,
                    ws = w.workspace_id,
                ));
            }
            rows
        }
        Err(e) => format!(
            r#"<div class="empty">Error: {}</div>"#,
            html_escape(&e.to_string())
        ),
    }
}

// ── Helpers ──────────────────────────────────────────────

fn kv(key: &str, value: &str) -> String {
    format!(
        r#"<div class="kv"><span class="key">{}</span><span class="val">{}</span></div>"#,
        html_escape(key),
        html_escape(value)
    )
}

fn volume_bar(vol: u8) -> String {
    let n = (vol.min(100) / 10) as usize;
    let filled = "█".repeat(n);
    let empty = "░".repeat(10 - n);
    format!("{}{}", filled, empty)
}

fn error_box_html(msg: &str) -> String {
    format!(r#"<div class="error-box">⚠ {}</div>"#, html_escape(msg))
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}
