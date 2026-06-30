use crate::DaemonState;
use crate::protocol::{Action, RequestOptions};

use super::dispatch_helpers::*;
use super::execute::execute_action;
use super::execute_agent::{execute_agent, is_agent_action};
use super::execute_blackboard::{execute_blackboard_action, is_blackboard_action};
use super::execute_macro::{execute_macro_action, is_macro_action};
use super::execute_rules::{execute_rules_action, is_rules_action};
use super::execute_secrets::{execute_secrets_action, is_secrets_action};
use super::execute_sessions::{execute_session_action, is_session_action};
use super::helpers::{not_supported_response, permission_denied_response};
use super::locks::{execute_lock_action, is_lock_action};
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
    let session_profile = state
        .sessions
        .get(session_id)
        .and_then(|session| session.profile.clone());

    if let Some(suspension) = state.auto_suspend.is_suspended(session_id).await
        && !session_suspension_bypass(&action)
    {
        let response = serde_json::json!({
            "type": "response",
            "id": request_id,
            "seq": seq,
            "status": "error",
            "error": {
                "code": "SESSION_SUSPENDED",
                "message": format!(
                    "session '{}' is suspended: {}",
                    session_id,
                    suspension.reason
                ),
                "session": session_id,
                "reason": suspension.reason,
                "trigger": suspension.trigger,
            }
        });
        audit_response(state, &action, peer_uid, seq, &response, started, None).await;
        return response;
    }

    // Per-namespace rate limit check (#129) — runs before global check
    if let Some(hit) = state.rate_limit_store.check(peer_uid, &action) {
        let response = namespace_rate_limited_response(&action, seq, &hit);
        audit_response(state, &action, peer_uid, seq, &response, started, None).await;
        return response;
    }

    if let Some(hit) =
        state
            .profile_rate_limit_store
            .check(session_profile.as_deref(), session_id, &action)
    {
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

    if let crate::permissions::ProfileCheck::Denied { profile, reason } = state
        .permissions
        .check_profile(session_profile.as_deref(), &action)
    {
        let response = profile_denied_response(request_id, seq, &profile, &reason);
        audit_response(state, &action, peer_uid, seq, &response, started, None).await;
        return response;
    }

    // Rules engine (peer_uid=0) must not dispatch HIGH_RISK actions without
    // explicit confirmation. Prevents privilege escalation under allow_all()
    // where uid 0 passes all permission checks. (W2+W3)
    if peer_uid == 0
        && crate::permissions::is_high_risk(action.action_type())
        && options.require_confirmation != Some(true)
    {
        let response = serde_json::json!({
            "type": "response",
            "id": request_id,
            "seq": seq,
            "status": "error",
            "error": {
                "code": "RULES_HIGH_RISK_BLOCKED",
                "message": format!(
                    "rules engine cannot dispatch high-risk action '{}' without confirmation",
                    action.action_type()
                )
            }
        });
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
        if let crate::permissions::ProfileCheck::Denied { profile, reason } = state
            .permissions
            .check_profile(session_profile.as_deref(), &implied_action)
        {
            let response = profile_denied_response(request_id, seq, &profile, &reason);
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
        if let crate::permissions::ProfileCheck::Denied { profile, reason } = state
            .permissions
            .check_profile(session_profile.as_deref(), &process_start)
        {
            let response = profile_denied_response(request_id, seq, &profile, &reason);
            audit_response(state, &action, peer_uid, seq, &response, started, None).await;
            return response;
        }
    }

    if let Some(event) = state.auto_suspend.record_action(session_id, &action).await {
        let reason = match &event {
            crate::protocol::DeskbridEvent::AgentSuspended { reason, .. } => reason.clone(),
            _ => "session suspended".to_string(),
        };
        let _ = state.event_tx.send(event);
        let response = serde_json::json!({
            "type": "response",
            "id": request_id,
            "seq": seq,
            "status": "error",
            "error": {
                "code": "SESSION_SUSPENDED",
                "message": reason,
                "session": session_id,
            }
        });
        audit_response(state, &action, peer_uid, seq, &response, started, None).await;
        return response;
    }

    // Record action if macro recording is active.
    // Skip recording control commands and sensitive namespaces to avoid
    // persisting secrets, clipboard contents, or process arguments to disk.
    {
        let at = action.action_type();
        if !at.starts_with("macro.")
            && !at.starts_with("secrets.")
            && !at.starts_with("clipboard.")
            && !at.starts_with("process.")
        {
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
    let profile_requires_confirmation = state
        .permissions
        .profile_requires_confirmation(session_profile.as_deref(), &action);
    if options.require_confirmation == Some(true) || profile_requires_confirmation {
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
            .insert(confirm_id.clone(), entry);
        let response = serde_json::json!({
            "type": "response",
            "id": request_id,
            "seq": seq,
            "status": "action_requires_confirmation",
            "confirmation_id": confirm_id,
            "action_type": action.action_type(),
            "profile_required": profile_requires_confirmation,
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

    state
        .agent_registry
        .record_action(session_id, action.action_type())
        .await;

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
    if crate::daemon::execute_confirmation::is_confirmation_action(&action) {
        let caller = peer_uid;
        let result = with_action_timeout(
            &action,
            action_timeout_ms,
            crate::daemon::execute_confirmation::execute_confirmation(
                action.clone(),
                state,
                caller,
            ),
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
    if is_agent_action(&action) {
        let sid = session_id.to_string();
        let result = with_action_timeout(
            &action,
            action_timeout_ms,
            execute_agent(action.clone(), state, &sid, peer_uid),
        )
        .await;
        return action_response(
            request_id, state, &action, peer_uid, seq, result, started, None,
        )
        .await;
    }
    if is_lock_action(&action) {
        let sid = session_id.to_string();
        let result = with_action_timeout(
            &action,
            action_timeout_ms,
            execute_lock_action(action.clone(), state, &sid),
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

    let backend_guard = state.backend.clone().read_owned().await;
    let backend = match backend_guard.as_ref() {
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
    // OwnedReadGuard is held for the action duration, but the underlying
    // Arc<RwLock<...>> can still be independently cloned by other tasks.
    // This avoids the borrow-on-DaemonState approach that tied the guard
    // lifetime to the state reference.

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

fn profile_denied_response(
    request_id: &str,
    seq: u64,
    profile: &str,
    reason: &str,
) -> serde_json::Value {
    serde_json::json!({
        "type": "response",
        "id": request_id,
        "seq": seq,
        "status": "error",
        "error": {
            "code": "PROFILE_DENIED",
            "message": reason,
            "profile": profile,
        }
    })
}

fn session_suspension_bypass(action: &Action) -> bool {
    matches!(
        action,
        Action::SessionResume { .. }
            | Action::SessionList
            | Action::AgentList
            | Action::AgentGet { .. }
            | Action::ConfirmationList
    )
}
