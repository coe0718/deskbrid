use crate::DaemonState;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

use super::{
    base64_encode, error_box_html, render_agent_mailbox, render_audio, render_audit,
    render_backlight, render_clipboard, render_confirmations, render_desktop_settings,
    render_macros, render_monitors, render_network, render_notifications, render_pressure,
    render_printers, render_rules, render_search, render_secrets, render_sessions, render_system,
    render_windows,
};

const HTML_PAGE: &str = include_str!("template.html");

/// Max request-line size (64 KB) — rejects oversized HTTP requests.
const MAX_REQUEST_LINE: u64 = 65536;
/// Max header line size (8 KB).
const MAX_HEADER_LINE: u64 = 8192;
/// Max number of HTTP headers.
const MAX_HEADERS: usize = 100;

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

    page = page.replace("__PRINTERS__", &render_printers(&backend_guard).await);
    page = page.replace("__CONFIRMATIONS__", &render_confirmations(state).await);
    page = page.replace("__AGENT_MAILBOX__", &render_agent_mailbox(state).await);
    page = page.replace("__SECRETS__", &render_secrets(state).await);
    page = page.replace("__SEARCH__", &render_search(state).await);
    page = page.replace("__PRESSURE__", &render_pressure().await);

    if show_screenshot && let Some(ref backend) = *backend_guard {
        match backend.screenshot(None, None, None).await {
            Ok(result) => match tokio::fs::read(&result.path).await {
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

// S3 (Vex review): emit a strict Content-Security-Policy on every
    // response so a compromised template can't exfiltrate data to
    // arbitrary origins. `default-src 'self'` covers img/style/script;
//     `'unsafe-inline'` is permitted for style because the dashboard
    // templates use inline <style> blocks. `connect-src 'self'` is
    // required so SSE event-stream URLs still work.
    const CSP_HEADER: &str = "Content-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' data:; frame-ancestors 'none'; base-uri 'self'\r\n";

    fn http_response(status: u16, content_type: &str, body: &[u8]) -> Vec<u8> {
        let status_text = match status {
            200 => "OK",
            404 => "Not Found",
            _ => "Internal Server Error",
        };
        let header = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n{}\r\n",
            status,
            status_text,
            content_type,
            body.len(),
            CSP_HEADER
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
        "printers" => {
            let backend = state.backend.read().await;
            render_printers(&backend).await
        }
        "confirmations" => render_confirmations(state).await,
        "agent-mailbox" => render_agent_mailbox(state).await,
        "search" => render_search(state).await,
        "secrets" => render_secrets(state).await,
        "pressure" => render_pressure().await,
        _ => r#"<div class="empty">Unknown card</div>"#.into(),
    }
}

/// Parse HTTP headers with size and count limits. Returns an error string
/// Send a simple HTTP error response and close the connection.
async fn send_error(writer: &mut tokio::net::tcp::WriteHalf<'_>, status: u16, msg: &str) {
    let body = format!("{} {}\n", status, msg);
    let _ = writer
        .write_all(&http_response(status, "text/plain", body.as_bytes()))
        .await;
}

pub(crate) async fn handle_request(
    mut stream: TcpStream,
    state: Arc<DaemonState>,
    token: Option<String>,
) -> anyhow::Result<()> {
    let (read_half, mut write_half) = stream.split();

    // Cap the total request body at 64KB — prevents memory exhaustion.
    let mut limited = read_half.take(MAX_REQUEST_LINE + MAX_HEADERS as u64 * MAX_HEADER_LINE);
    let mut reader = BufReader::new(&mut limited);

    let mut request_line = String::new();
    if reader.read_line(&mut request_line).await.is_err() {
        send_error(&mut write_half, 400, "Bad Request").await;
        return Ok(());
    }
    if request_line.len() > MAX_REQUEST_LINE as usize {
        send_error(&mut write_half, 414, "URI Too Long").await;
        return Ok(());
    }

    let (method, path) = parse_request_line(request_line.trim()).unwrap_or(("GET", "/"));

    // Read headers with bounds
    let mut auth_header: Option<String> = None;
    let mut header_count = 0;
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => {
                send_error(&mut write_half, 400, "Bad Request").await;
                return Ok(());
            }
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        if line.len() > MAX_HEADER_LINE as usize {
            let _ = send_error(&mut write_half, 431, "Request Header Fields Too Large").await;
            return Ok(());
        }
        if let Some(rest) = line.to_ascii_lowercase().strip_prefix("authorization:") {
            auth_header = Some(rest.trim().to_string());
        }
        header_count += 1;
        if header_count > MAX_HEADERS {
            let _ = send_error(&mut write_half, 431, "Too Many Headers").await;
            return Ok(());
        }
    }

    // Auth gate: if a token is configured, the request MUST carry
    // `Authorization: Bearer <token>`. This is what W2 was missing —
    // the dashboard was reachable to anyone on the network.
    if let Some(expected) = token.as_deref() {
        let presented = auth_header
            .as_deref()
            .and_then(|h| {
                h.strip_prefix("Bearer ")
                    .or_else(|| h.strip_prefix("bearer "))
            })
            .unwrap_or("");
        // Constant-time-ish compare (good enough; not for crypto secrets).
        let presented_bytes = presented.as_bytes();
        let expected_bytes = expected.as_bytes();
        let ok = presented_bytes.len() == expected_bytes.len()
            && presented_bytes
                .iter()
                .zip(expected_bytes.iter())
                .fold(0u8, |acc, (a, b)| acc | (a ^ b))
                == 0
            && !presented.is_empty();
        if !ok {
            let _ = send_error(&mut write_half, 401, "Unauthorized").await;
            return Ok(());
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
            "printers",
            "confirmations",
            "agent-mailbox",
            "secrets",
            "search",
            "pressure",
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
                Ok(result) => match tokio::fs::read(&result.path).await {
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
