use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::{DeskbridEvent, Region};
use anyhow::Context;
use serde_json::{Value, json};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::helpers::{ensure_safe_pid, expand_path};

const DEFAULT_INTERVAL_MS: u64 = 200;
const MAX_INTERVAL_MS: u64 = 1_000;
const MAX_FILE_READ_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Default)]
struct WaitState {
    screenshot_last: Option<Vec<u8>>,
    screenshot_same_since: Option<Instant>,
}

struct CheckOutcome {
    matched: bool,
    value: Value,
}

pub async fn wait_for_condition(
    state: &DaemonState,
    backend: &dyn DesktopBackend,
    condition: &str,
    params: Value,
    timeout_ms: u64,
    interval_ms: Option<u64>,
) -> anyhow::Result<Value> {
    let condition = condition.trim().to_lowercase();
    if condition.is_empty() {
        anyhow::bail!("condition must not be empty");
    }
    let timeout = Duration::from_millis(timeout_ms.max(1));
    let base_interval = Duration::from_millis(
        interval_ms
            .unwrap_or(DEFAULT_INTERVAL_MS)
            .clamp(50, MAX_INTERVAL_MS),
    );
    let wait_id = uuid::Uuid::new_v4().to_string();
    let started = Instant::now();
    let mut wait_state = WaitState::default();
    let mut attempts = 0u64;

    loop {
        attempts += 1;
        let outcome = check_condition(backend, &condition, &params, &mut wait_state).await?;
        let elapsed_ms = started.elapsed().as_millis();
        if outcome.matched {
            let response = json!({
                "wait_id": wait_id,
                "condition": condition,
                "matched": true,
                "elapsed_ms": elapsed_ms,
                "attempts": attempts,
                "value": outcome.value
            });
            let _ = state.event_tx.send(DeskbridEvent::WaitMatched {
                wait_id,
                condition,
                value: response["value"].clone(),
                elapsed_ms,
                timestamp: unix_now(),
            });
            return Ok(response);
        }
        if started.elapsed() >= timeout {
            return Ok(json!({
                "wait_id": wait_id,
                "condition": condition,
                "matched": false,
                "elapsed_ms": elapsed_ms,
                "attempts": attempts,
                "value": outcome.value,
                "reason": "timeout"
            }));
        }

        let remaining = timeout.saturating_sub(started.elapsed());
        tokio::time::sleep(base_interval.min(remaining)).await;
    }
}

async fn check_condition(
    backend: &dyn DesktopBackend,
    condition: &str,
    params: &Value,
    state: &mut WaitState,
) -> anyhow::Result<CheckOutcome> {
    match condition {
        "window_exists" => check_window_exists(backend, params).await,
        "window_title" => check_window_title(backend, params).await,
        "clipboard_contains" => check_clipboard_contains(backend, params).await,
        "process_exits" | "process_exit" => check_process_exits(params),
        "process_exists" => check_process_exists(params),
        "file_exists" => check_file_exists(params).await,
        "file_content" | "file_contains" => check_file_content(params).await,
        "idle_seconds" => check_idle_seconds(backend, params).await,
        "screenshot_stable" => check_screenshot_stable(backend, params, state).await,
        other => anyhow::bail!("unknown wait condition: {other}"),
    }
}

async fn check_window_exists(
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

async fn check_window_title(
    backend: &dyn DesktopBackend,
    params: &Value,
) -> anyhow::Result<CheckOutcome> {
    let title = param_string(params, &["title", "contains", "text"])
        .ok_or_else(|| anyhow::anyhow!("window_title requires params.title"))?;
    let mut params = params.clone();
    params["title"] = json!(title);
    check_window_exists(backend, &params).await
}

async fn check_clipboard_contains(
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

fn check_process_exits(params: &Value) -> anyhow::Result<CheckOutcome> {
    let pid = param_u32(params, &["pid"])?;
    ensure_safe_pid(pid)?;
    let exists = process_exists(pid)?;
    Ok(CheckOutcome {
        matched: !exists,
        value: json!({"pid": pid, "exists": exists}),
    })
}

fn check_process_exists(params: &Value) -> anyhow::Result<CheckOutcome> {
    let pid = param_u32(params, &["pid"])?;
    ensure_safe_pid(pid)?;
    let exists = process_exists(pid)?;
    Ok(CheckOutcome {
        matched: exists,
        value: json!({"pid": pid, "exists": exists}),
    })
}

async fn check_file_exists(params: &Value) -> anyhow::Result<CheckOutcome> {
    let path = param_string(params, &["path"])
        .ok_or_else(|| anyhow::anyhow!("file_exists requires params.path"))?;
    let path = expand_path(&path)?;
    let exists = tokio::fs::metadata(&path).await.is_ok();
    Ok(CheckOutcome {
        matched: exists,
        value: json!({"path": path.to_string_lossy(), "exists": exists}),
    })
}

async fn check_file_content(params: &Value) -> anyhow::Result<CheckOutcome> {
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

async fn check_idle_seconds(
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

async fn check_screenshot_stable(
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

fn process_exists(pid: u32) -> anyhow::Result<bool> {
    let rc = unsafe { libc::kill(pid as i32, 0) };
    if rc == 0 {
        return Ok(true);
    }
    let errno = std::io::Error::last_os_error()
        .raw_os_error()
        .unwrap_or_default();
    if errno == libc::ESRCH {
        Ok(false)
    } else {
        anyhow::bail!(
            "failed to check pid {}: {}",
            pid,
            std::io::Error::last_os_error()
        )
    }
}

fn region_param(params: &Value) -> anyhow::Result<Option<Region>> {
    let Some(region) = params.get("region") else {
        return Ok(None);
    };
    if region.is_null() {
        return Ok(None);
    }
    Ok(Some(Region {
        x: region["x"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("region.x is required"))? as u32,
        y: region["y"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("region.y is required"))? as u32,
        width: region["width"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("region.width is required"))? as u32,
        height: region["height"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("region.height is required"))? as u32,
    }))
}

fn param_string(params: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        params
            .get(*key)
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned)
    })
}

fn param_u32(params: &Value, keys: &[&str]) -> anyhow::Result<u32> {
    let value = param_u64(params, keys)?;
    if value == 0 || value > u32::MAX as u64 {
        anyhow::bail!("parameter '{}' must be a positive u32", keys[0]);
    }
    Ok(value as u32)
}

fn param_u64(params: &Value, keys: &[&str]) -> anyhow::Result<u64> {
    param_u64_optional(params, keys)
        .ok_or_else(|| anyhow::anyhow!("missing numeric parameter '{}'", keys[0]))
}

fn param_u64_optional(params: &Value, keys: &[&str]) -> Option<u64> {
    keys.iter().find_map(|key| {
        params.get(*key).and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_str().and_then(|s| s.parse::<u64>().ok()))
        })
    })
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
