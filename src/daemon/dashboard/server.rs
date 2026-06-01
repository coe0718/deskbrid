use crate::DaemonState;
use std::sync::Arc;

use super::{
    base64_encode, error_box_html, render_audio, render_audit, render_backlight, render_clipboard,
    render_desktop_settings, render_macros, render_monitors, render_network, render_notifications,
    render_rules, render_sessions, render_system, render_windows,
};

const HTML_PAGE: &str = include_str!("template.html");

pub(crate) async fn build_page(state: &DaemonState, show_screenshot: bool) -> String {
    let mut page = HTML_PAGE
        .replace("__VERSION__", env!("CARGO_PKG_VERSION"))
        .to_string();

    let backend_guard = state.backend.read().await;
    let backend_available = backend_guard.is_some();

    let (status_class, status_text) = if backend_available {
        ("online", "daemon running")
    } else {
        ("offline", "no backend loaded")
    };
    page = page.replace("__STATUS_CLASS__", status_class);
    page = page.replace("__STATUS_TEXT__", status_text);

    let mut error_box = String::new();
    let mut screenshot_html = String::new();

    let system_info = if let Some(ref backend) = *backend_guard {
        match backend.system_info().await {
            Ok(info) => Some(info),
            Err(e) => {
                error_box = error_box_html(&format!("System info failed: {}", e));
                None
            }
        }
    } else {
        None
    };

    page = page.replace("__SYSTEM__", &super::render_system(&system_info));
    page = page.replace("__MONITORS__", &super::render_monitors(&system_info));
    page = page.replace("__NETWORK__", &render_network().await);
    page = page.replace("__AUDIO__", &super::render_audio().await);
    page = page.replace("__WINDOWS__", &super::render_windows(&backend_guard).await);
    page = page.replace("__CLIPBOARD__", &render_clipboard(state).await);
    page = page.replace("__AUDIT__", &render_audit(state).await);
    page = page.replace("__SESSIONS__", &render_sessions(state).await);
    page = page.replace("__RULES__", &render_rules(state).await);
    page = page.replace("__NOTIFICATIONS__", &render_notifications(state).await);
    page = page.replace("__MACROS__", &render_macros().await);

    let backlight_info = if let Some(ref backend) = *backend_guard {
        backend.backlight_get(None).await.ok()
    } else {
        None
    };
    page = page.replace("__BACKLIGHT__", &render_backlight(&backlight_info));

    page = page.replace(
        "__DESKTOP_SETTINGS__",
        &render_desktop_settings(&backend_guard).await,
    );

    if show_screenshot && let Some(ref backend) = *backend_guard {
        match backend.screenshot(None, None, None).await {
            Ok(result) => match std::fs::read(&result.path) {
                Ok(bytes) => {
                    let b64 = base64_encode(&bytes);
                    screenshot_html = format!(
                        r#"<div class="screenshot-wrap card"><h2>📸 Screenshot ({w}x{h})</h2><img src="data:image/png;base64,{b64}" alt="Screenshot"></div>"#,
                        w = result.width,
                        h = result.height,
                        b64 = b64,
                    );
                }
                Err(e) => {
                    screenshot_html = error_box_html(&format!("Failed to read screenshot: {}", e))
                }
            },
            Err(e) => screenshot_html = error_box_html(&format!("Screenshot failed: {}", e)),
        }
    }
    page = page.replace("__SCREENSHOT_HTML__", &screenshot_html);
    page = page.replace("__ERROR_BOX__", &error_box);
    page
}

fn parse_request_line(line: &str) -> Option<(&str, &str)> {
    let mut parts = line.split_whitespace();
    let method = parts.next()?;
    let path = parts.next()?;
    Some((method, path))
}

fn http_response(status: u16, content_type: &str, body: &[u8]) -> Vec<u8> {
    let status_text = match status {
        200 => "OK",
        404 => "Not Found",
        _ => "Internal Server Error",
    };
    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status,
        status_text,
        content_type,
        body.len()
    );
    let mut response = header.into_bytes();
    response.extend_from_slice(body);
    response
}

/// SSE card dispatcher — called from the poll loop in handle_request.
async fn sse_card_html(card: &str, state: &DaemonState) -> String {
    match card {
        "system" => {
            let backend = state.backend.read().await;
            let info = if let Some(ref b) = *backend {
                b.system_info().await.ok()
            } else {
                None
            };
            render_system(&info)
        }
        "monitors" => {
            let backend = state.backend.read().await;
            let info = if let Some(ref b) = *backend {
                b.system_info().await.ok()
            } else {
                None
            };
            render_monitors(&info)
        }
        "windows" => {
            let backend = state.backend.read().await;
            render_windows(&backend).await
        }
        "clipboard" => render_clipboard(state).await,
        "audit" => render_audit(state).await,
        "network" => render_network().await,
        "audio" => render_audio().await,
        "sessions" => render_sessions(state).await,
        "rules" => render_rules(state).await,
        "notifications" => render_notifications(state).await,
        "macros" => render_macros().await,
        "desktop-settings" => {
            let backend = state.backend.read().await;
            render_desktop_settings(&backend).await
        }
        "backlight" => {
            let backend = state.backend.read().await;
            let info = if let Some(ref b) = *backend {
                b.backlight_get(None).await.ok()
            } else {
                None
            };
            render_backlight(&info)
        }
        _ => r#"<div class="empty">Unknown card</div>"#.into(),
    }
}

pub(crate) async fn handle_request(
    mut stream: tokio::net::TcpStream,
    state: Arc<DaemonState>,
) -> anyhow::Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let (read_half, mut write_half) = stream.split();
    let mut reader = BufReader::new(read_half);
    let mut request_line = String::new();
    reader.read_line(&mut request_line).await?;

    let (method, path) = parse_request_line(request_line.trim()).unwrap_or(("GET", "/"));

    loop {
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        if line.trim().is_empty() {
            break;
        }
    }

    // SSE event stream — polls cards every 3 seconds
    if method == "GET" && path == "/events" {
        let header = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\n\r\n";
        write_half.write_all(header.as_bytes()).await?;
        write_half.flush().await?;

        let connected = "data: {\"type\":\"connected\"}\n\n";
        write_half.write_all(connected.as_bytes()).await?;
        write_half.flush().await?;

        let volatile_cards = [
            "windows",
            "clipboard",
            "audit",
            "network",
            "audio",
            "sessions",
            "rules",
            "notifications",
            "macros",
            "desktop-settings",
            "backlight",
        ];
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            for card in &volatile_cards {
                let html = sse_card_html(card, &state).await;
                let json = serde_json::json!({"card": card, "html": html});
                if let Ok(line) = serde_json::to_string(&json) {
                    let sse = format!("data: {}\n\n", line);
                    if write_half.write_all(sse.as_bytes()).await.is_err() {
                        return Ok(()); // client disconnected
                    }
                }
            }
            if write_half.flush().await.is_err() {
                return Ok(());
            }
        }
    }

    if method == "GET" && (path == "/screenshot" || path.starts_with("/screenshot?")) {
        let backend_guard = state.backend.read().await;
        if let Some(ref backend) = *backend_guard {
            match backend.screenshot(None, None, None).await {
                Ok(result) => match std::fs::read(&result.path) {
                    Ok(bytes) => {
                        drop(backend_guard);
                        write_half
                            .write_all(&http_response(200, "image/png", &bytes))
                            .await?;
                        return Ok(());
                    }
                    Err(e) => {
                        drop(backend_guard);
                        let body = format!("Failed to read screenshot: {}", e);
                        write_half
                            .write_all(&http_response(500, "text/plain", body.as_bytes()))
                            .await?;
                        return Ok(());
                    }
                },
                Err(e) => {
                    drop(backend_guard);
                    let body = format!("Screenshot failed: {}", e);
                    write_half
                        .write_all(&http_response(500, "text/plain", body.as_bytes()))
                        .await?;
                    return Ok(());
                }
            }
        } else {
            drop(backend_guard);
            write_half
                .write_all(&http_response(503, "text/plain", b"No backend loaded"))
                .await?;
            return Ok(());
        }
    }

    if method == "GET" && (path == "/" || path.starts_with("/?")) {
        let show_screenshot = path.contains("screenshot=1");
        let html = build_page(&state, show_screenshot).await;
        write_half
            .write_all(&http_response(
                200,
                "text/html; charset=utf-8",
                html.as_bytes(),
            ))
            .await?;
    } else {
        write_half
            .write_all(&http_response(404, "text/plain", b"Not Found"))
            .await?;
    }

    Ok(())
}
