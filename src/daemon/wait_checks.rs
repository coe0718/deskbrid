use super::helpers::*;
use crate::backend::DesktopBackend;
use anyhow::Context;
use serde_json::{Value, json};
use std::time::Instant;

use super::wait::{CheckOutcome, MAX_FILE_READ_BYTES, WaitState};
use super::wait_params::*;

pub(crate) async fn check_window_exists(
    backend: &dyn DesktopBackend,
    params: &Value,
) -> anyhow::Result<CheckOutcome> {
    let windows = backend.windows_list().await?;
    let title = param_string(params, &["title", "title_contains"]);
    let app_id = param_string(params, &["app_id", "app"]);
    let window_id = param_string(params, &["window_id", "id"]);
    let title_l = title.as_deref().map(str::to_lowercase);
    let app_l = app_id.as_deref().map(str::to_lowercase);
    let window_id_l = window_id.as_deref().map(str::to_lowercase);
    if title_l.is_none() && app_l.is_none() && window_id_l.is_none() {
        anyhow::bail!("window_exists requires params.window_id, params.title, or params.app_id");
    }

    let matched = windows.into_iter().find(|window| {
        let mut ok = true;
        if let Some(id) = &window_id_l {
            ok &= window.id.to_lowercase() == *id;
        }
        if let Some(title) = &title_l {
            ok &= window.title.to_lowercase().contains(title);
        }
        if let Some(app) = &app_l {
            ok &= window.app_id.to_lowercase().contains(app);
        }
        ok
    });
    Ok(match matched {
        Some(window) => CheckOutcome {
            matched: true,
            value: serde_json::to_value(window)?,
        },
        None => CheckOutcome {
            matched: false,
            value: json!({"windows_checked": true}),
        },
    })
}

pub(crate) async fn check_window_title(
    backend: &dyn DesktopBackend,
    params: &Value,
) -> anyhow::Result<CheckOutcome> {
    let title = param_string(params, &["title", "contains", "text"])
        .ok_or_else(|| anyhow::anyhow!("window_title requires params.title"))?;
    let mut params = params.clone();
    params["title"] = json!(title);
    check_window_exists(backend, &params).await
}

pub(crate) async fn check_clipboard_contains(
    backend: &dyn DesktopBackend,
    params: &Value,
) -> anyhow::Result<CheckOutcome> {
    let text = param_string(params, &["text", "contains"])
        .ok_or_else(|| anyhow::anyhow!("clipboard_contains requires params.text"))?;
    let clipboard = backend.clipboard_read().await.unwrap_or_default();
    let matched = clipboard.contains(&text);
    Ok(CheckOutcome {
        matched,
        value: json!({"text": clipboard, "contains": text}),
    })
}

pub(crate) fn check_process_exits(params: &Value) -> anyhow::Result<CheckOutcome> {
    let pid = param_u32(params, &["pid"])?;
    ensure_safe_pid(pid)?;
    let exists = process_exists(pid)?;
    Ok(CheckOutcome {
        matched: !exists,
        value: json!({"pid": pid, "exists": exists}),
    })
}

pub(crate) fn check_process_exists(params: &Value) -> anyhow::Result<CheckOutcome> {
    let pid = param_u32(params, &["pid"])?;
    ensure_safe_pid(pid)?;
    let exists = process_exists(pid)?;
    Ok(CheckOutcome {
        matched: exists,
        value: json!({"pid": pid, "exists": exists}),
    })
}

pub(crate) async fn check_file_exists(params: &Value) -> anyhow::Result<CheckOutcome> {
    let path = param_string(params, &["path"])
        .ok_or_else(|| anyhow::anyhow!("file_exists requires params.path"))?;
    let path = expand_path(&path)?;
    let exists = tokio::fs::metadata(&path).await.is_ok();
    Ok(CheckOutcome {
        matched: exists,
        value: json!({"path": path.to_string_lossy(), "exists": exists}),
    })
}

pub(crate) async fn check_file_content(params: &Value) -> anyhow::Result<CheckOutcome> {
    let path = param_string(params, &["path"])
        .ok_or_else(|| anyhow::anyhow!("file_content requires params.path"))?;
    let pattern = param_string(params, &["pattern", "contains", "text"])
        .ok_or_else(|| anyhow::anyhow!("file_content requires params.pattern"))?;
    let path = expand_path(&path)?;
    let metadata = match tokio::fs::metadata(&path).await {
        Ok(metadata) => metadata,
        Err(_) => {
            return Ok(CheckOutcome {
                matched: false,
                value: json!({"path": path.to_string_lossy(), "exists": false}),
            });
        }
    };
    if metadata.len() > MAX_FILE_READ_BYTES {
        anyhow::bail!("file too large to scan: {}", path.display());
    }
    let text = tokio::fs::read_to_string(&path)
        .await
        .with_context(|| format!("failed to read {}", path.display()))?;
    let matched = text.contains(&pattern);
    Ok(CheckOutcome {
        matched,
        value: json!({"path": path.to_string_lossy(), "contains": pattern, "bytes": text.len()}),
    })
}

pub(crate) async fn check_idle_seconds(
    backend: &dyn DesktopBackend,
    params: &Value,
) -> anyhow::Result<CheckOutcome> {
    let seconds = param_u64(params, &["seconds", "min_seconds", "idle_seconds"])?;
    let current = backend.idle_seconds().await?;
    Ok(CheckOutcome {
        matched: current >= seconds,
        value: json!({"idle_seconds": current, "target_seconds": seconds}),
    })
}

pub(crate) async fn check_screenshot_stable(
    backend: &dyn DesktopBackend,
    params: &Value,
    state: &mut WaitState,
) -> anyhow::Result<CheckOutcome> {
    let stable_ms = param_u64_optional(params, &["stable_ms"]).unwrap_or(1_000);
    let monitor = param_u64_optional(params, &["monitor"]).map(|v| v as u32);
    let region = region_param(params)?;
    let window_id = param_string(params, &["window_id"]);
    let screenshot = backend.screenshot(monitor, region, window_id).await?;
    let bytes = tokio::fs::read(&screenshot.path)
        .await
        .with_context(|| format!("failed to read screenshot {}", screenshot.path))?;
    let _ = tokio::fs::remove_file(&screenshot.path).await;

    let now = Instant::now();
    let stable_for_ms = if state.screenshot_last.as_ref() == Some(&bytes) {
        let since = *state.screenshot_same_since.get_or_insert(now);
        now.duration_since(since).as_millis() as u64
    } else {
        state.screenshot_last = Some(bytes);
        state.screenshot_same_since = Some(now);
        0
    };
    Ok(CheckOutcome {
        matched: stable_for_ms >= stable_ms,
        value: json!({
            "stable_for_ms": stable_for_ms,
            "target_stable_ms": stable_ms,
            "width": screenshot.width,
            "height": screenshot.height
        }),
    })
}
