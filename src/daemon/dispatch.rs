use crate::DaemonState;
use crate::protocol::{Action, RequestOptions};

use super::dispatch_helpers::*;
use super::execute::execute_action;
use super::execute_blackboard::{execute_blackboard_action, is_blackboard_action};
use super::execute_macro::{execute_macro_action, is_macro_action};
use super::execute_rules::{execute_rules_action, is_rules_action};
use super::execute_secrets::{execute_secrets_action, is_secrets_action};
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

    // Per-namespace rate limit check (#129) — runs before global check
    if let Some(hit) = state.rate_limit_store.check(peer_uid, &action) {
        let response = namespace_rate_limited_response(&action, seq, &hit);
        audit_response(state, &action, peer_uid, seq, &response, started, None).await;
        return response;
    }

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

    // Handle confirmation gate (#37)
    if options.require_confirmation == Some(true) {
        let confirm_id = state.next_confirmation_id();
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let entry = crate::daemon::execute_confirmation::PendingConfirmation {
            request_id: request_id.to_string(),
            action: action.clone(),
            options: options.clone(),
            peer_uid,
            seq,
            session_id: session_id.to_string(),
            created_at: now_ms,
        };
        state
            .pending_confirmations
            .lock()
            .await
            .insert(confirm_id.clone(), entry);
        let response = serde_json::json!({
            "type": "response",
            "id": request_id,
            "seq": seq,
            "status": "action_requires_confirmation",
            "confirmation_id": confirm_id,
            "action_type": action.action_type(),
        });
        audit_response(
            state,
            &action,
            peer_uid,
            seq,
            &response,
            started,
            Some(true),
        )
        .await;
        return response;
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
    if is_blackboard_action(&action) {
        let result = with_action_timeout(
            &action,
            action_timeout_ms,
            execute_blackboard_action(action.clone(), state),
        )
        .await;
        return action_response(
            request_id, state, &action, peer_uid, seq, result, started, None,
        )
        .await;
    }
    if is_rules_action(&action) {
        let result = with_action_timeout(
            &action,
            action_timeout_ms,
            execute_rules_action(action.clone(), state),
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
    if is_secrets_action(&action) {
        // Secrets reads and writes always require confirmation
        if !matches!(&action, Action::SecretsListCollections)
            && options.require_confirmation != Some(true)
        {
            let response = serde_json::json!({
                "type": "response",
                "id": request_id,
                "seq": seq,
                "status": "error",
                "error": {
                    "code": "CONFIRMATION_REQUIRED",
                    "message": "secrets.get_secret and secrets.store_secret require confirmation"
                }
            });
            audit_response(state, &action, peer_uid, seq, &response, started, None).await;
            return response;
        }
        let result = with_action_timeout(
            &action,
            action_timeout_ms,
            execute_secrets_action(action.clone(), state),
        )
        .await;
        return action_response(
            request_id, state, &action, peer_uid, seq, result, started, None,
        )
        .await;
    }

    // Backend-free system actions — no desktop session required.
    // D-Bus calls talk to the message bus directly; network actions use nmcli.
    if matches!(&action, Action::DbusCall { .. }) {
        let result = with_action_timeout(
            &action,
            action_timeout_ms,
            super::execute_system::execute_dbus_call(&action),
        )
        .await;
        return action_response(
            request_id, state, &action, peer_uid, seq, result, started, None,
        )
        .await;
    }

    if is_network_action_backend_free(&action) {
        let result = with_action_timeout(
            &action,
            action_timeout_ms,
            super::execute_network::execute_network(action.clone()),
        )
        .await;
        return action_response(
            request_id, state, &action, peer_uid, seq, result, started, None,
        )
        .await;
    }

    // A11y actions are backend-free — they talk directly to AT-SPI2 over D-Bus.
    if is_a11y_action(&action) {
        let result = with_action_timeout(
            &action,
            action_timeout_ms,
            super::execute_a11y::execute_a11y(action.clone(), state),
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

/// Check if an action is a network action that doesn't require a desktop backend.
/// Network actions use nmcli/zbus directly — no GUI needed.
fn is_network_action_backend_free(action: &Action) -> bool {
    matches!(
        action,
        Action::NetworkStatus
            | Action::NetworkInterfaces
            | Action::NetworkWifiScan
            | Action::NetworkWifiConnect { .. }
            | Action::NetworkConnectionList
            | Action::NetworkConnectionProfiles
            | Action::NetworkCreateHotspot { .. }
            | Action::NetworkStopHotspot
            | Action::NetworkWifiEnable { .. }
            | Action::NetworkWwanEnable { .. }
            | Action::NetworkDnsSet { .. }
            | Action::NetworkDnsReset
            | Action::NetworkVpnConnect { .. }
            | Action::NetworkVpnDisconnect
    )
}

/// Check if an action is an AT-SPI2 accessibility action.
/// A11y actions talk directly to the AT-SPI bus over D-Bus — no desktop backend needed.
fn is_a11y_action(action: &Action) -> bool {
    matches!(
        action,
        Action::A11yTree { .. }
            | Action::A11yGetElement { .. }
            | Action::A11yClickElement { .. }
            | Action::A11yGetText { .. }
            | Action::A11ySnapshotTree { .. }
            | Action::A11yPerformAction { .. }
            | Action::A11ySetValue { .. }
            | Action::A11yGetElementText { .. }
            | Action::A11yListApps { .. }
            | Action::A11yDoctor
            | Action::A11ySetupAccessibility
            | Action::A11yClickElementByRef { .. }
    )
}
