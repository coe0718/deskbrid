use crate::DaemonState;
use crate::protocol::{Action, RequestOptions};
use std::future::Future;
use std::time::Duration;
use tracing::warn;

use super::AuditRecord;
use super::audit::record_audit_entry;

pub(crate) fn effective_timeout_ms(
    action: &Action,
    state: &DaemonState,
    options: &RequestOptions,
) -> Option<u64> {
    if let Some(timeout_ms) = options.timeout_ms {
        return Some(timeout_ms);
    }
    if let Action::WaitFor { timeout_ms, .. } = action {
        return Some(timeout_ms.saturating_add(1000));
    }
    state.action_timeout_ms
}

pub(crate) async fn with_action_timeout<F>(
    action: &Action,
    timeout_ms: Option<u64>,
    future: F,
) -> anyhow::Result<serde_json::Value>
where
    F: Future<Output = anyhow::Result<serde_json::Value>>,
{
    let Some(timeout_ms) = timeout_ms else {
        return future.await;
    };
    match tokio::time::timeout(Duration::from_millis(timeout_ms), future).await {
        Ok(result) => result,
        Err(_) => anyhow::bail!(
            "action timed out after {} ms: {}",
            timeout_ms,
            action.action_type()
        ),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn action_response(
    request_id: &str,
    state: &DaemonState,
    action: &Action,
    peer_uid: u32,
    seq: u64,
    result: anyhow::Result<serde_json::Value>,
    started: std::time::Instant,
    dry_run: Option<bool>,
) -> serde_json::Value {
    let response = match result {
        Ok(data) => {
            emit_action_event(state, action, &data);
            serde_json::json!({
                "type": "response", "id": request_id, "seq": seq, "status": "ok", "data": data
            })
        }
        Err(e) => {
            warn!("Action failed: {}", e);
            serde_json::json!({
                "type": "response", "id": request_id, "seq": seq, "status": "error",
                "error": { "code": "INTERNAL_ERROR", "message": format!("{}", e) }
            })
        }
    };
    audit_response(state, action, peer_uid, seq, &response, started, dry_run).await;
    response
}

pub(crate) async fn audit_response(
    state: &DaemonState,
    action: &Action,
    peer_uid: u32,
    seq: u64,
    response: &serde_json::Value,
    started: std::time::Instant,
    dry_run: Option<bool>,
) {
    let status = response["status"].as_str().unwrap_or("unknown").to_string();
    let error = response
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(|message| message.as_str())
        .map(String::from);
    record_audit_entry(
        state,
        AuditRecord {
            seq,
            peer_uid,
            action_type: action.action_type().to_string(),
            status,
            duration_ms: started.elapsed().as_millis().try_into().unwrap_or(u64::MAX),
            error,
            dry_run,
        },
    )
    .await;
}

pub(crate) fn implied_permission_actions(action: &Action) -> Vec<Action> {
    match action {
        Action::LayoutProfileSave { .. } => {
            vec![
                Action::WindowsList,
                Action::WorkspacesList,
                Action::SystemInfo,
            ]
        }
        Action::LayoutProfileRestore { .. } => vec![
            Action::WindowsMoveResize {
                window_id: "profile".into(),
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            },
            Action::WindowsMinimize("profile".into()),
            Action::WorkspaceSwitch(0),
            Action::WorkspaceMoveWindow {
                window_id: "profile".into(),
                workspace_id: 0,
                follow: false,
            },
        ],
        _ => Vec::new(),
    }
}

pub(crate) fn emit_action_event(state: &DaemonState, action: &Action, data: &serde_json::Value) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let event = match action {
        // Use the resolved window ID from the response data when available,
        // so subscribers get the canonical ID, not the caller-provided selector.
        Action::WindowsFocus(_) => {
            let window_id = data
                .get("focused")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            Some(crate::protocol::DeskbridEvent::WindowFocused {
                window_id,
                timestamp: now,
            })
        }
        Action::WorkspaceSwitch(id) => Some(crate::protocol::DeskbridEvent::WorkspaceChanged {
            workspace_id: *id,
            timestamp: now,
        }),
        Action::WorkspaceMoveWindow {
            window_id,
            workspace_id,
            ..
        } => Some(crate::protocol::DeskbridEvent::WorkspaceWindowMoved {
            window_id: window_id.clone(),
            workspace_id: *workspace_id,
            timestamp: now,
        }),
        _ => None,
    };
    if let Some(evt) = event {
        let _ = state.event_tx.send(evt);
    }
    let _ = data;
}

/// Rate-limited response with namespace info (#129).
pub(crate) fn namespace_rate_limited_response(
    action: &Action,
    seq: u64,
    hit: &crate::daemon::RateLimitHit,
) -> serde_json::Value {
    let namespace = crate::daemon::action_namespace(action);
    serde_json::json!({
        "type": "response",
        "id": "action",
        "seq": seq,
        "status": "error",
        "error": {
            "code": "RATE_LIMITED",
            "message": format!(
                "rate limit exceeded for action '{}'; retry after {} ms",
                action.action_type(),
                hit.retry_after_ms
            ),
            "retry_after_ms": hit.retry_after_ms,
            "namespace": namespace
        }
    })
}
