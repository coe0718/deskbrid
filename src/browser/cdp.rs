use anyhow::Context;
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static CDP_COMMAND_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, serde::Deserialize)]
pub struct CdpTarget {
    #[serde(rename = "type")]
    pub target_type: Option<String>,
    pub title: Option<String>,
    pub url: Option<String>,
    #[serde(rename = "webSocketDebuggerUrl")]
    ws_url: Option<String>,
    pub id: Option<String>,
}

pub async fn discover_targets() -> anyhow::Result<Vec<CdpTarget>> {
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

    if let Some(targets) = discover_chrome_active_port().await {
        return Ok(targets);
    }

    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("no Chrome/Chromium DevTools endpoint found")))
}

async fn discover_chrome_active_port() -> Option<Vec<CdpTarget>> {
    let chrome_socket = dirs::home_dir()?.join(".config/google-chrome/DevToolsActivePort");
    let contents = tokio::fs::read_to_string(&chrome_socket).await.ok()?;
    let port = contents.lines().next()?.trim().parse::<u16>().ok()?;
    reqwest::get(&format!("http://127.0.0.1:{port}/json"))
        .await
        .ok()?
        .json::<Vec<CdpTarget>>()
        .await
        .ok()
}

pub async fn send_cdp_command(ws_url: &str, method: &str, params: Value) -> anyhow::Result<Value> {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::connect_async;

    let (mut ws, _) = connect_async(ws_url)
        .await
        .with_context(|| format!("failed to connect to CDP websocket: {ws_url}"))?;

    let id = CDP_COMMAND_ID.fetch_add(1, Ordering::Relaxed);
    let msg = serde_json::json!({ "id": id, "method": method, "params": params });
    ws.send(tokio_tungstenite::tungstenite::Message::Text(
        msg.to_string().into(),
    ))
    .await
    .context("failed to send CDP command")?;

    let timeout = Duration::from_secs(30);
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if tokio::time::Instant::now() > deadline {
            anyhow::bail!("CDP response timeout after {timeout:?}");
        }
        match tokio::time::timeout(Duration::from_secs(5), ws.next()).await {
            Ok(Some(Ok(msg))) => {
                if let Some(result) = cdp_response_result(msg, id)? {
                    return Ok(result);
                }
            }
            Ok(Some(Err(e))) => anyhow::bail!("CDP websocket error: {e}"),
            Ok(None) => anyhow::bail!("CDP websocket closed"),
            Err(_) => anyhow::bail!("CDP response timeout"),
        }
    }
}

fn cdp_response_result(
    msg: tokio_tungstenite::tungstenite::Message,
    id: u64,
) -> anyhow::Result<Option<Value>> {
    let text = match msg {
        tokio_tungstenite::tungstenite::Message::Text(t) => t.to_string(),
        tokio_tungstenite::tungstenite::Message::Close(_) => {
            anyhow::bail!("CDP websocket closed unexpectedly");
        }
        _ => return Ok(None),
    };

    let resp: Value = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse CDP response: {text}"))?;
    if resp.get("id").and_then(|v| v.as_u64()) != Some(id) {
        return Ok(None);
    }
    if let Some(error) = resp.get("error") {
        let msg = error
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown CDP error");
        anyhow::bail!("CDP error: {msg}");
    }
    Ok(Some(resp.get("result").cloned().unwrap_or(Value::Null)))
}

pub fn get_page_ws_url(targets: &[CdpTarget], tab_index: Option<u32>) -> anyhow::Result<String> {
    let pages: Vec<&CdpTarget> = targets
        .iter()
        .filter(|t| t.target_type.as_deref() == Some("page"))
        .collect();

    if pages.is_empty() {
        anyhow::bail!(
            "no browser page targets found - is Chrome running with --remote-debugging-port?"
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
