use crate::DaemonState;
use crate::protocol::{Action, RequestOptions};

use super::dispatch_helpers::*;
use super::execute::execute_action;
use super::execute_macro::{execute_macro_action, is_macro_action};
use super::execute_sessions::{execute_session_action, is_session_action};
use super::helpers::{not_supported_response, permission_denied_response};
use super::rate_limited_response;
use super::system::{execute_system_control_action, is_system_control_action};
use super::terminal::{execute_terminal_action, is_terminal_action};
use super::wait_for_condition;
use super::{
    check_rate_limit, execute_app_catalog_action, execute_audit_action,
    execute_clipboard_history_action, execute_mpris_action, is_app_catalog_action, is_audit_action,
    is_clipboard_history_action, is_mpris_action,
};

pub async fn dispatch_action(
    request_id: &str,
    action: Action,
    state: &DaemonState,
    peer_uid: u32,
    seq: u64,
) -> serde_json::Value {
    dispatch_action_with_options(
        request_id,
        action,
        state,
        peer_uid,
        seq,
        RequestOptions::default(),
        "default",
    )
    .await
}

pub async fn dispatch_action_with_options(
    request_id: &str,
    action: Action,
    state: &DaemonState,
    peer_uid: u32,
    seq: u64,
    options: RequestOptions,
    session_id: &str,
) -> serde_json::Value {
    let started = std::time::Instant::now();
    let action_timeout_ms = effective_timeout_ms(&action, state, &options);

    if let Some(hit) = check_rate_limit(state, peer_uid).await {
        let response = rate_limited_response(seq, hit);
        audit_response(state, &action, peer_uid, seq, &response, started, None).await;
        return response;
    }

    // Check permissions first
    if !state.permissions.check(peer_uid, &action) {
        let response = permission_denied_response(request_id, action.action_type(), seq);
        audit_response(state, &action, peer_uid, seq, &response, started, None).await;
        return response;
    }
    for implied_action in implied_permission_actions(&action) {
        if !state.permissions.check(peer_uid, &implied_action) {
            let response =
                permission_denied_response(request_id, implied_action.action_type(), seq);
            audit_response(state, &action, peer_uid, seq, &response, started, None).await;
            return response;
        }
    }
    if let Action::WindowsActivateOrLaunch {
        command,
        workdir,
        env,
        ..
    } = &action
    {
        let process_start = Action::ProcessStart {
            command: command.clone(),
            workdir: workdir.clone(),
            env: env.clone(),
        };
        if !state.permissions.check(peer_uid, &process_start) {
            let response = permission_denied_response(request_id, process_start.action_type(), seq);
            audit_response(state, &action, peer_uid, seq, &response, started, None).await;
            return response;
        }
    }

    // Record action if macro recording is active (skip recording control commands)
    {
        let at = action.action_type();
        if !at.starts_with("macro.") {
            let params = action.to_json().unwrap_or_default();
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&params) {
                crate::daemon::macro_engine::record_action(state, at, parsed);
            }
        }
    }

    if options.dry_run {
        let data = serde_json::json!({
            "dry_run": true,
            "would_execute": true,
            "action_type": action.action_type(),
            "timeout_ms": action_timeout_ms,
            "permissions": {"allowed": true}
        });
        return action_response(
            request_id,
            state,
            &action,
            peer_uid,
            seq,
            Ok(data),
            started,
            Some(true),
        )
        .await;
    }

    // Macro actions handled by their own executor
    if is_macro_action(&action) {
        return match execute_macro_action(&action, state, request_id, seq, peer_uid).await {
            Ok(Some(response)) => response,
            Ok(None) => {
                let response = not_supported_response(request_id, "unknown macro action", seq);
                audit_response(state, &action, peer_uid, seq, &response, started, None).await;
                response
            }
            Err(e) => {
                let response = serde_json::json!({
                    "type": "response", "id": request_id, "seq": seq, "status": "error",
                    "error": { "code": "MACRO_ERROR", "message": e.to_string() }
                });
                audit_response(state, &action, peer_uid, seq, &response, started, None).await;
                response
            }
        };
    }

    if is_audit_action(&action) {
        let result = with_action_timeout(
            &action,
            action_timeout_ms,
            execute_audit_action(action.clone(), state),
        )
        .await;
        return action_response(
            request_id, state, &action, peer_uid, seq, result, started, None,
        )
        .await;
    }
    if is_clipboard_history_action(&action) {
        let result = with_action_timeout(
            &action,
            action_timeout_ms,
            execute_clipboard_history_action(action.clone(), state),
        )
        .await;
        return action_response(
            request_id, state, &action, peer_uid, seq, result, started, None,
        )
        .await;
    }
    if is_app_catalog_action(&action) {
        let result = with_action_timeout(
            &action,
            action_timeout_ms,
            execute_app_catalog_action(action.clone(), state),
        )
        .await;
        return action_response(
            request_id, state, &action, peer_uid, seq, result, started, None,
        )
        .await;
    }
    if is_mpris_action(&action) {
        let result = with_action_timeout(
            &action,
            action_timeout_ms,
            execute_mpris_action(action.clone(), state),
        )
        .await;
        return action_response(
            request_id, state, &action, peer_uid, seq, result, started, None,
        )
        .await;
    }
    if is_system_control_action(&action) {
        let result = with_action_timeout(
            &action,
            action_timeout_ms,
            execute_system_control_action(action.clone(), state),
        )
        .await;
        return action_response(
            request_id, state, &action, peer_uid, seq, result, started, None,
        )
        .await;
    }
    if is_terminal_action(&action) {
        let result = with_action_timeout(
            &action,
            action_timeout_ms,
            execute_terminal_action(action.clone(), state),
        )
        .await;
        return action_response(
            request_id, state, &action, peer_uid, seq, result, started, None,
        )
        .await;
    }
    if is_session_action(&action) {
        let sid = session_id.to_string();
        let result = with_action_timeout(
            &action,
            action_timeout_ms,
            execute_session_action(action.clone(), state, &sid),
        )
        .await;
        return action_response(
            request_id, state, &action, peer_uid, seq, result, started, None,
        )
        .await;
    }

    let backend = state.backend.read().await;
    let backend = match backend.as_ref() {
        Some(b) => b,
        None => {
            let response = not_supported_response(
                request_id,
                "no desktop backend loaded (start daemon inside a supported Linux session)",
                seq,
            );
            audit_response(state, &action, peer_uid, seq, &response, started, None).await;
            return response;
        }
    };

    let result = if let Action::WaitFor {
        condition,
        params,
        timeout_ms,
        interval_ms,
    } = &action
    {
        with_action_timeout(
            &action,
            action_timeout_ms,
            wait_for_condition(
                state,
                backend.as_ref(),
                condition,
                params.clone(),
                *timeout_ms,
                *interval_ms,
            ),
        )
        .await
    } else {
        with_action_timeout(
            &action,
            action_timeout_ms,
            execute_action(action.clone(), backend.as_ref(), state),
        )
        .await
    };
    action_response(
        request_id, state, &action, peer_uid, seq, result, started, None,
    )
    .await
}
