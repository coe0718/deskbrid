use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::DeskbridEvent;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

pub(crate) const DEFAULT_INTERVAL_MS: u64 = 200;
pub(crate) const MAX_INTERVAL_MS: u64 = 1_000;
pub(crate) const MAX_FILE_READ_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Default)]
pub(crate) struct WaitState {
    pub(crate) screenshot_last: Option<Vec<u8>>,
    pub(crate) screenshot_same_since: Option<Instant>,
}

pub(crate) struct CheckOutcome {
    pub(crate) matched: bool,
    pub(crate) value: Value,
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

use super::wait_checks::*;
use super::wait_params::*;
