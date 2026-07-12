use crate::DaemonState;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

const DASHBOARD_PORT: u16 = 20129;

/// Max concurrent dashboard connections. Protects against fd/memory exhaustion.
const MAX_DASHBOARD_CONNECTIONS: usize = 32;

mod render_data;
mod server;

use render_data::{
    render_agent_mailbox, render_audit, render_clipboard, render_confirmations, render_macros,
    render_notifications, render_rules, render_search, render_secrets, render_sessions,
};

#[allow(clippy::too_many_arguments)]
pub async fn start(state: Arc<DaemonState>, bind_ip: String, token: Option<String>) {
    // When bound to anything other than loopback, REQUIRE a token. This
    // is a hard fail — earlier this only printed a warning, leaving the
    // dashboard open to anyone on the network.
    let is_loopback = matches!(bind_ip.as_str(), "127.0.0.1" | "::1" | "localhost" | "");
    let effective_token: Option<String> = if is_loopback {
        // Loopback: token is optional. If provided, we'll check it.
        token
    } else {
        match token {
            Some(t) if !t.is_empty() => Some(t),
            _ => {
                error!(
                    "Dashboard bound to non-loopback address {} but no --dashboard-token \
                     was provided. Refusing to start — pass --dashboard-token <secret> to \
                     expose the dashboard over the network.",
                    bind_ip
                );
                return;
            }
        }
    };
    let addr = format!("{}:{}", bind_ip, DASHBOARD_PORT);
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("Dashboard bind {}: {}", addr, e);
            return;
        }
    };
    info!("Dashboard: http://{}", addr);
    if !is_loopback {
        info!("Dashboard: non-loopback bind — bearer token required for all requests");
    }
    let semaphore = Arc::new(Semaphore::new(MAX_DASHBOARD_CONNECTIONS));
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let permit = match semaphore.clone().try_acquire_owned() {
                    Ok(p) => p,
                    Err(_) => {
                        // All permits exhausted — drop connection gracefully
                        warn!(
                            "Dashboard: connection limit ({}) reached, rejecting",
                            MAX_DASHBOARD_CONNECTIONS
                        );
                        continue;
                    }
                };
                let state = Arc::clone(&state);
                let token = effective_token.clone();
                tokio::spawn(async move {
                    let _permit = permit; // hold until handler finishes
                    if let Err(e) = server::handle_request(stream, state, token).await {
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

pub(super) async fn render_printers(
    backend: &Option<Box<dyn crate::backend::DesktopBackend>>,
) -> String {
    let Some(backend) = backend else {
        return r#"<div class="empty">No backend</div>"#.into();
    };
    match backend.print_list().await {
        Ok(printers) => {
            if printers.is_empty() {
                return r#"<div class="empty">No printers</div>"#.into();
            }
            let mut rows = String::new();
            for p in &printers {
                let def = if p.is_default { " ⭐" } else { "" };
                let status_icon = match p.status.as_str() {
                    "idle" => "🟢",
                    "printing" => "🔵",
                    "disabled" => "🔴",
                    _ => "⚪",
                };
                rows.push_str(&kv(
                    &p.name,
                    &format!("{} {}{}", status_icon, p.status, def),
                ));
            }
            match backend.print_jobs().await {
                Ok(jobs) if !jobs.is_empty() => {
                    rows.push_str(r#"<div class="section-label">Active Jobs</div>"#);
                    for j in jobs.iter().take(5) {
                        rows.push_str(&kv(
                            &format!("Job #{}", j.id),
                            &format!("{} — {}", j.printer, j.status),
                        ));
                    }
                }
                _ => {}
            }
            rows
        }
        Err(e) => format!(
            r#"<div class="empty">Error: {}</div>"#,
            html_escape(&e.to_string())
        ),
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

pub(super) async fn render_pressure() -> String {
    async fn read_avg10(path: &str) -> f64 {
        tokio::fs::read_to_string(path)
            .await
            .ok()
            .and_then(|s| {
                s.lines().find(|l| l.starts_with("some")).and_then(|l| {
                    l.split_whitespace()
                        .find(|p| p.starts_with("avg10="))
                        .and_then(|p| p.strip_prefix("avg10=")?.parse().ok())
                })
            })
            .unwrap_or(0.0)
    }

    let cpu = read_avg10("/proc/pressure/cpu").await;
    let mem = read_avg10("/proc/pressure/memory").await;
    let io = read_avg10("/proc/pressure/io").await;

    fn bar(val: f64) -> String {
        let pct = (val * 100.0).min(100.0) as u8;
        let color = if pct > 50 {
            "danger"
        } else if pct > 10 {
            "warn"
        } else {
            "ok"
        };
        let blocks = (pct / 5) as usize;
        let filled = "█".repeat(blocks);
        let empty = "░".repeat(20 - blocks);
        format!(
            "<span class=\"{}\">{} {:.1}%</span> <span class=\"bar\">{}{}</span>",
            color,
            if pct > 50 {
                "🔴"
            } else if pct > 10 {
                "🟡"
            } else {
                "🟢"
            },
            val * 100.0,
            filled,
            empty
        )
    }

    let mut rows = String::new();
    rows.push_str(&format!(
        "<div class=\"kv\"><span class=\"key\">CPU</span>{}</div>",
        bar(cpu)
    ));
    rows.push_str(&format!(
        "<div class=\"kv\"><span class=\"key\">Memory</span>{}</div>",
        bar(mem)
    ));
    rows.push_str(&format!(
        "<div class=\"kv\"><span class=\"key\">IO</span>{}</div>",
        bar(io)
    ));
    rows
}
