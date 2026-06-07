use std::sync::Arc;

use tracing::{debug, error, info, warn};

use crate::DaemonState;
use crate::protocol::{Action, DeskbridEvent, EventTrigger, Rule, RuleCondition};

/// Resolve the app_id for an event by looking up the window via `windows_list()`.
/// This fills in the gap for backends that don't include app_id in their event payloads.
async fn resolve_event_app_id(mut event: DeskbridEvent, state: &DaemonState) -> DeskbridEvent {
    let window_id = match &event {
        DeskbridEvent::WindowFocused {
            window_id,
            app_id: None,
            ..
        } => Some(window_id.clone()),
        DeskbridEvent::WindowOpened {
            window_id,
            app_id: None,
            ..
        } => Some(window_id.clone()),
        DeskbridEvent::WindowClosed {
            window_id,
            app_id: None,
            ..
        } => Some(window_id.clone()),
        _ => None,
    };

    let Some(wid) = window_id else {
        return event; // already has app_id, or not a window event
    };

    // Resolve app_id from backend if available
    let app_id = {
        let backend_guard = state.backend.read().await;
        if let Some(ref backend) = *backend_guard {
            match backend.windows_list().await {
                Ok(windows) => windows
                    .iter()
                    .find(|w| w.id == wid)
                    .map(|w| w.app_id.clone()),
                Err(e) => {
                    debug!("resolve_event_app_id: windows_list() failed: {}", e);
                    None
                }
            }
        } else {
            None
        }
    };

    match &mut event {
        DeskbridEvent::WindowFocused { app_id: a, .. }
        | DeskbridEvent::WindowOpened { app_id: a, .. }
        | DeskbridEvent::WindowClosed { app_id: a, .. }
            if a.is_none() && app_id.is_some() =>
        {
            *a = app_id;
            debug!(
                "resolve_event_app_id: resolved app_id={:?} for window {}",
                a, wid
            );
        }
        _ => {}
    }

    event
}

/// Check whether a given EventTrigger matches a DeskbridEvent.
pub(super) fn trigger_matches_event(trigger: &EventTrigger, event: &DeskbridEvent) -> bool {
    match trigger {
        EventTrigger::ClipboardChanged => {
            // No dedicated clipboard-changed event yet — reserved for future use.
            // TODO: emit ClipboardChanged from clipboard write path.
            false
        }
        EventTrigger::WindowOpened { app_id: filter } => {
            if let DeskbridEvent::WindowOpened {
                window_id: _,
                app_id: event_app_id,
                timestamp: _,
            } = event
            {
                match (filter, event_app_id) {
                    (None, _) => true,
                    (Some(f), Some(e)) => f == e,
                    (Some(_), None) => {
                        debug!(
                            "WindowOpened: trigger has app_id filter but event lacks app_id \
                             (backend doesn't emit it) — no match"
                        );
                        false
                    }
                }
            } else {
                false
            }
        }
        EventTrigger::WindowClosed { app_id: filter } => {
            if let DeskbridEvent::WindowClosed {
                window_id: _,
                app_id: event_app_id,
                timestamp: _,
            } = event
            {
                match (filter, event_app_id) {
                    (None, _) => true,
                    (Some(f), Some(e)) => f == e,
                    (Some(_), None) => {
                        debug!(
                            "WindowClosed: trigger has app_id filter but event lacks app_id \
                             (backend doesn't emit it) — no match"
                        );
                        false
                    }
                }
            } else {
                false
            }
        }
        EventTrigger::WindowFocused { app_id: filter } => {
            if let DeskbridEvent::WindowFocused {
                window_id: _,
                app_id: event_app_id,
                timestamp: _,
            } = event
            {
                match (filter, event_app_id) {
                    (None, _) => true,
                    (Some(f), Some(e)) => f == e,
                    (Some(_), None) => {
                        debug!(
                            "WindowFocused: trigger has app_id filter but event lacks app_id \
                             (backend doesn't provide it) — no match"
                        );
                        false
                    }
                }
            } else {
                false
            }
        }
        EventTrigger::SessionLocked
        | EventTrigger::SessionUnlocked
        | EventTrigger::IdleStarted
        | EventTrigger::IdleEnded => {
            // These triggers are reserved for future DeskbridEvent variants.
            false
        }
        EventTrigger::FileChanged { path } => match event {
            DeskbridEvent::FileCreated {
                path: ev_path,
                timestamp: _,
            }
            | DeskbridEvent::FileModified {
                path: ev_path,
                timestamp: _,
            }
            | DeskbridEvent::FileDeleted {
                path: ev_path,
                timestamp: _,
            } => ev_path.starts_with(path),
            _ => false,
        },
        EventTrigger::TimeRange {
            start_hour: _,
            end_hour: _,
            days: _,
        } => {
            // TimeRange triggers are evaluated on a timer, not per-event.
            false
        }
        EventTrigger::PresenceChanged { to: _ } => {
            // Reserved for future presence events.
            false
        }
    }
}

/// Evaluate a RuleCondition against the current state.
/// Returns true if the condition passes (or if there is no condition).
/// Called from within `evaluate()`, which holds no external locks.
pub(super) fn condition_matches(
    condition: &Option<RuleCondition>,
    state: &DaemonState,
    _event: &DeskbridEvent,
) -> bool {
    let Some(cond) = condition else {
        return true; // no condition → always passes
    };

    match cond {
        RuleCondition::VarEquals { name, value } => {
            // Try to read session "default". If locked (rare), skip this tick.
            let Ok(sessions) = state.sessions.try_lock() else {
                debug!("condition_matches: sessions lock held, skipping");
                return false;
            };
            if let Some(session) = sessions.get("default") {
                session.vars.get(name).map(|v| v == value).unwrap_or(false)
            } else {
                false
            }
        }
        RuleCondition::VarExists { name } => {
            let Ok(sessions) = state.sessions.try_lock() else {
                return false;
            };
            if let Some(session) = sessions.get("default") {
                session.vars.contains_key(name)
            } else {
                false
            }
        }
    }
}

/// Evaluate TimeRange rules on a timer.
/// Called periodically (every 60s) from the background timer task.
pub(super) async fn evaluate_timerange_rules(state: &Arc<DaemonState>, now_ms: u64) {
    let now = chrono::Local::now();
    let current_hour: u8 = now.format("%H").to_string().parse().unwrap_or(0);
    let current_day: u8 = now.format("%u").to_string().parse().unwrap_or(1);

    let to_dispatch: Vec<(Rule, Action)> = {
        let mut engine = state.rules.lock().await;
        let mut actions = Vec::new();

        for rule in &engine.list().to_vec() {
            if !rule.enabled {
                continue;
            }

            let EventTrigger::TimeRange {
                start_hour,
                end_hour,
                days,
            } = &rule.trigger
            else {
                continue;
            };

            let in_range = if *start_hour <= *end_hour {
                current_hour >= *start_hour && current_hour < *end_hour
            } else {
                current_hour >= *start_hour || current_hour < *end_hour
            };

            if !in_range {
                continue;
            }

            if !days.is_empty() && !days.contains(&current_day) {
                continue;
            }

            // Check condition
            // TimeRange rules fire on a timer with no associated event,
            // so we pass a dummy event for condition evaluation.
            if !condition_matches(
                &rule.condition,
                state,
                &DeskbridEvent::WorkspaceChanged {
                    workspace_id: 0,
                    timestamp: 0,
                },
            ) {
                continue;
            }

            // Check cooldown
            if let Some(cooldown_ms) = rule.cooldown_ms {
                let rt = engine.runtime.get(&rule.id);
                if let Some(rt) = rt
                    && now_ms.saturating_sub(rt.last_fire_ms) < cooldown_ms
                {
                    continue;
                }
            }

            // Check max_fires
            if let Some(max_fires) = rule.max_fires {
                let count = engine
                    .runtime
                    .get(&rule.id)
                    .map(|r| r.fire_count)
                    .unwrap_or(0);
                if count >= max_fires {
                    continue;
                }
            }

            // Build action JSON
            let mut action_json = serde_json::json!({
                "type": rule.action_type,
                "id": format!("rule-{}", rule.id),
            });
            if !rule.action_params.is_null()
                && let serde_json::Value::Object(ref params) = rule.action_params
            {
                for (k, v) in params {
                    action_json[k] = v.clone();
                }
            }

            let action_str = serde_json::to_string(&action_json).unwrap_or_default();
            match Action::from_json(&action_str) {
                Ok((_request_id, action)) => {
                    let rt = engine.runtime.entry(rule.id.clone()).or_default();
                    rt.fire_count += 1;
                    rt.last_fire_ms = now_ms;

                    actions.push((rule.clone(), action));
                }
                Err(e) => {
                    error!(
                        "TimeRange: failed to parse rule '{}' action: {}",
                        rule.name, e
                    );
                }
            }
        }

        actions
    };

    for (rule, action) in to_dispatch {
        info!(
            "TimeRange rule '{}' firing action: {}",
            rule.name,
            action.action_type()
        );
        let state = Arc::clone(state);
        tokio::spawn(async move {
            let seq = crate::daemon::helpers::unix_timestamp();
            let result = crate::daemon::dispatch::dispatch_action(
                &format!("timerange-{}", rule.id),
                action,
                &state,
                0,
                seq,
            )
            .await;
            debug!(
                "TimeRange rule '{}' action completed: {:?}",
                rule.name,
                result.get("status")
            );
        });
    }
}

/// Spawn the TimeRange evaluator background task.
/// Runs every 60 seconds, checking all enabled TimeRange rules.
pub fn spawn_timerange_evaluator(state: Arc<DaemonState>) {
    tokio::spawn(async move {
        info!("TimeRange rules evaluator started (60s interval)");
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            evaluate_timerange_rules(&state, now_ms).await;
        }
    });
}

/// Spawn the rules engine background task.
/// Subscribes to the event broadcast channel and evaluates rules on each event.
pub fn spawn_rules_engine(state: Arc<DaemonState>) {
    tokio::spawn(async move {
        let mut event_rx = state.event_tx.subscribe();
        info!("Rules engine started");

        // Load persisted rules into engine
        {
            let persisted = {
                let db = state.database.lock().unwrap();
                db.load_rules().unwrap_or_else(|e| {
                    warn!("Failed to load persisted rules: {}", e);
                    Vec::new()
                })
            };
            let mut engine = state.rules.lock().await;
            engine.load_persisted(persisted);
            info!("Loaded {} persisted rules", engine.list().len());
        }

        loop {
            match event_rx.recv().await {
                Ok(event) => {
                    // Resolve app_id for window events where backends don't provide it.
                    // This lets WindowFocused/WindowOpened/WindowClosed triggers with
                    // app_id filters actually match even when the event payload is sparse.
                    let event = resolve_event_app_id(event, &state).await;

                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;

                    let to_dispatch: Vec<(Rule, Action)> = {
                        let mut engine = state.rules.lock().await;
                        engine.evaluate(&event, now_ms, &state)
                    };

                    for (rule, action) in to_dispatch {
                        info!(
                            "Rule '{}' firing action: {}",
                            rule.name,
                            action.action_type()
                        );

                        let action_str = action.to_json().unwrap_or_default();
                        match Action::from_json(&action_str) {
                            Ok((request_id, parsed_action)) => {
                                let state = Arc::clone(&state);
                                tokio::spawn(async move {
                                    let seq = crate::daemon::helpers::unix_timestamp();
                                    let result = crate::daemon::dispatch::dispatch_action(
                                        &request_id,
                                        parsed_action,
                                        &state,
                                        0, // peer_uid: rule actions run as daemon
                                        seq,
                                    )
                                    .await;
                                    debug!(
                                        "Rule '{}' action completed: {:?}",
                                        rule.name,
                                        result.get("status")
                                    );
                                });
                            }
                            Err(e) => {
                                error!("Failed to re-parse rule '{}' action: {}", rule.name, e);
                            }
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    warn!("Rules engine lagged by {} events — skipping", n);
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    info!("Event channel closed — rules engine shutting down");
                    break;
                }
            }
        }
    });
}
