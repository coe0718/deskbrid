mod cdp;

use anyhow::Context;
use cdp::{discover_targets, get_page_ws_url, send_cdp_command};
use serde_json::Value;

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

/// Set text on an element by CSS selector (input fields, textareas, contenteditable).
pub async fn set_text(tab_index: Option<u32>, selector: &str, text: &str) -> anyhow::Result<Value> {
    let targets = discover_targets().await?;
    let ws_url = get_page_ws_url(&targets, tab_index)?;

    let escaped_text = text
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r");
    let escaped_selector = selector.replace('\'', "\\'");

    let js = format!(
        "(function() {{ \
            const el = document.querySelector('{escaped_selector}'); \
            if (!el) return {{ ok: false, error: 'element not found' }}; \
            el.focus(); \
            el.value = '{escaped_text}'; \
            el.dispatchEvent(new Event('input', {{ bubbles: true }})); \
            el.dispatchEvent(new Event('change', {{ bubbles: true }})); \
            return {{ ok: true }}; \
        }})()"
    );

    let result = send_cdp_command(
        &ws_url,
        "Runtime.evaluate",
        serde_json::json!({
            "expression": js,
            "returnByValue": true,
        }),
    )
    .await?;

    let value = result
        .get("result")
        .and_then(|r| r.get("value"))
        .cloned()
        .unwrap_or(serde_json::json!({"ok": false, "error": "no result"}));

    Ok(serde_json::json!({
        "selector": selector,
        "text_length": text.len(),
        "result": value,
    }))
}
