use anyhow::Context;
use serde_json::Value;
use std::time::Duration;

/// CDP websocket target info.
#[derive(Debug, serde::Deserialize)]
struct CdpTarget {
    #[serde(rename = "type")]
    target_type: Option<String>,
    title: Option<String>,
    url: Option<String>,
    #[serde(rename = "webSocketDebuggerUrl")]
    ws_url: Option<String>,
    id: Option<String>,
}

/// Discover Chrome/Chromium DevTools targets via the local HTTP API.
/// Tries common ports: 9222 (user-launched), 9229 (Node), and the default user-data-dir socket.
async fn discover_targets() -> anyhow::Result<Vec<CdpTarget>> {
    let ports = [9222, 9229];
    let mut last_err = None;

    for port in ports {
        let url = format!("http://127.0.0.1:{port}/json");
        match reqwest::get(&url).await {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(targets) = resp.json::<Vec<CdpTarget>>().await {
                    return Ok(targets);
                }
            }
            Ok(resp) => {
                last_err = Some(anyhow::anyhow!("CDP port {port}: HTTP {}", resp.status()));
            }
            Err(e) => {
                last_err = Some(anyhow::anyhow!("CDP port {port}: {e}"));
            }
        }
    }

    // Try default Chrome user-data-dir remote debugging socket
    let chrome_socket = dirs::home_dir()
        .map(|h| h.join(".config/google-chrome/DevToolsActivePort"))
        .unwrap_or_default();
    if chrome_socket.exists()
        && let Ok(contents) = tokio::fs::read_to_string(&chrome_socket).await
        && let Some(port_str) = contents.lines().next()
        && let Ok(port) = port_str.trim().parse::<u16>()
        && let Ok(resp) = reqwest::get(&format!("http://127.0.0.1:{port}/json")).await
        && let Ok(targets) = resp.json::<Vec<CdpTarget>>().await
    {
        return Ok(targets);
    }

    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("no Chrome/Chromium DevTools endpoint found")))
}

/// Send a CDP command and return the result.
async fn send_cdp_command(ws_url: &str, method: &str, params: Value) -> anyhow::Result<Value> {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::connect_async;

    let (mut ws, _) = connect_async(ws_url)
        .await
        .with_context(|| format!("failed to connect to CDP websocket: {ws_url}"))?;

    let id: u32 = 1;
    let msg = serde_json::json!({
        "id": id,
        "method": method,
        "params": params,
    });

    ws.send(tokio_tungstenite::tungstenite::Message::Text(
        msg.to_string().into(),
    ))
    .await
    .context("failed to send CDP command")?;

    // Read responses until we get the one with our id
    let timeout = Duration::from_secs(30);
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        if tokio::time::Instant::now() > deadline {
            anyhow::bail!("CDP response timeout after {timeout:?}");
        }

        match tokio::time::timeout(Duration::from_secs(5), ws.next()).await {
            Ok(Some(Ok(msg))) => {
                let text = match msg {
                    tokio_tungstenite::tungstenite::Message::Text(t) => t.to_string(),
                    tokio_tungstenite::tungstenite::Message::Close(_) => {
                        anyhow::bail!("CDP websocket closed unexpectedly");
                    }
                    _ => continue,
                };

                let resp: Value = serde_json::from_str(&text)
                    .with_context(|| format!("failed to parse CDP response: {text}"))?;

                if resp.get("id").and_then(|v| v.as_u64()) == Some(id as u64) {
                    if let Some(error) = resp.get("error") {
                        let msg = error
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown CDP error");
                        anyhow::bail!("CDP error: {msg}");
                    }
                    return Ok(resp.get("result").cloned().unwrap_or(Value::Null));
                }
                // Skip events (no id field)
            }
            Ok(Some(Err(e))) => anyhow::bail!("CDP websocket error: {e}"),
            Ok(None) => anyhow::bail!("CDP websocket closed"),
            Err(_) => anyhow::bail!("CDP response timeout"),
        }
    }
}

/// Get the ws URL for a page target, optionally by index.
fn get_page_ws_url(targets: &[CdpTarget], tab_index: Option<u32>) -> anyhow::Result<String> {
    let pages: Vec<&CdpTarget> = targets
        .iter()
        .filter(|t| t.target_type.as_deref() == Some("page"))
        .collect();

    if pages.is_empty() {
        anyhow::bail!(
            "no browser page targets found — is Chrome running with --remote-debugging-port?"
        );
    }

    let target = match tab_index {
        Some(idx) => pages.get(idx as usize).ok_or_else(|| {
            anyhow::anyhow!("tab index {idx} out of range ({} pages)", pages.len())
        })?,
        None => pages
            .first()
            .ok_or_else(|| anyhow::anyhow!("no page targets found"))?,
    };

    target
        .ws_url
        .clone()
        .ok_or_else(|| anyhow::anyhow!("target has no websocket URL"))
}

/// List all open browser tabs.
pub async fn list_tabs() -> anyhow::Result<Value> {
    let targets = discover_targets().await?;
    let pages: Vec<Value> = targets
        .iter()
        .filter(|t| t.target_type.as_deref() == Some("page"))
        .enumerate()
        .map(|(i, t)| {
            serde_json::json!({
                "index": i,
                "id": t.id.as_deref().unwrap_or("unknown"),
                "title": t.title.as_deref().unwrap_or("untitled"),
                "url": t.url.as_deref().unwrap_or("about:blank"),
            })
        })
        .collect();

    Ok(serde_json::json!({"tabs": pages}))
}

/// Navigate a tab to a URL.
pub async fn navigate(tab_index: Option<u32>, url: &str) -> anyhow::Result<Value> {
    let targets = discover_targets().await?;
    let ws_url = get_page_ws_url(&targets, tab_index)?;

    let result =
        send_cdp_command(&ws_url, "Page.navigate", serde_json::json!({"url": url})).await?;

    Ok(serde_json::json!({
        "navigated": url,
        "result": result,
    }))
}

/// Execute JavaScript in a tab.
pub async fn evaluate(
    tab_index: Option<u32>,
    expression: &str,
    await_promise: bool,
) -> anyhow::Result<Value> {
    let targets = discover_targets().await?;
    let ws_url = get_page_ws_url(&targets, tab_index)?;

    let params = if await_promise {
        serde_json::json!({
            "expression": format!("new Promise((resolve, reject) => {{ try {{ resolve({expression}) }} catch(e) {{ reject(e) }} }})"),
            "awaitPromise": true,
            "returnByValue": true,
        })
    } else {
        serde_json::json!({
            "expression": expression,
            "returnByValue": true,
        })
    };

    let result = send_cdp_command(&ws_url, "Runtime.evaluate", params).await?;

    // Extract the actual value from CDP's wrapped response
    let value = result
        .get("result")
        .and_then(|r| r.get("value"))
        .cloned()
        .unwrap_or(result);

    Ok(serde_json::json!({"result": value}))
}

/// Screenshot a specific tab.
pub async fn screenshot_tab(tab_index: Option<u32>) -> anyhow::Result<Value> {
    let targets = discover_targets().await?;
    let ws_url = get_page_ws_url(&targets, tab_index)?;

    let result = send_cdp_command(
        &ws_url,
        "Page.captureScreenshot",
        serde_json::json!({"format": "png"}),
    )
    .await?;

    let data = result
        .get("data")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("CDP screenshot returned no data"))?;

    Ok(serde_json::json!({
        "format": "png",
        "data": data,
        "size_bytes": data.len(),
    }))
}

/// Click an element by CSS selector.
pub async fn click(tab_index: Option<u32>, selector: &str) -> anyhow::Result<Value> {
    let targets = discover_targets().await?;
    let ws_url = get_page_ws_url(&targets, tab_index)?;

    // First find the element
    let doc_result = send_cdp_command(
        &ws_url,
        "Runtime.evaluate",
        serde_json::json!({
            "expression": format!(
                "document.querySelector('{}')",
                selector.replace('\'', "\\'")
            ),
            "returnByValue": false,
        }),
    )
    .await?;

    let object_id = doc_result
        .get("result")
        .and_then(|r| r.get("objectId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("element not found: {selector}"))?;

    // Scroll into view
    let _ = send_cdp_command(
        &ws_url,
        "Runtime.callFunctionOn",
        serde_json::json!({
            "objectId": object_id,
            "functionDeclaration": "function() { this.scrollIntoView({block:'center'}); return true; }",
            "returnByValue": true,
        }),
    )
    .await;

    // Get bounding box
    let box_result = send_cdp_command(
        &ws_url,
        "DOM.getBoxModel",
        serde_json::json!({"objectId": object_id}),
    )
    .await?;

    let content = box_result
        .get("model")
        .and_then(|m| m.get("content"))
        .ok_or_else(|| anyhow::anyhow!("could not get element bounds for: {selector}"))?;

    let coords: Vec<f64> = serde_json::from_value(content.clone()).unwrap_or_default();
    if coords.len() < 2 {
        anyhow::bail!("invalid bounding box for: {selector}");
    }

    // Calculate center point from content quad (4 corners: [x0,y0,x1,y1,x2,y2,x3,y3])
    let x = (coords[0] + coords[2] + coords[4] + coords[6]) / 4.0;
    let y = (coords[1] + coords[3] + coords[5] + coords[7]) / 4.0;

    // Simulate mouse events: press → release
    for (ev_type, button_state) in [("mousePressed", "pressed"), ("mouseReleased", "released")] {
        send_cdp_command(
            &ws_url,
            "Input.dispatchMouseEvent",
            serde_json::json!({
                "type": ev_type,
                "x": x,
                "y": y,
                "button": "left",
                "clickCount": 1,
            }),
        )
        .await
        .with_context(|| format!("mouse {button_state} at ({x:.0},{y:.0})"))?;
    }

    Ok(serde_json::json!({
        "clicked": selector,
        "position": {"x": x, "y": y},
    }))
}
