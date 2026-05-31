use std::sync::Arc;

use tracing::{debug, error, info, warn};

use crate::DaemonState;
use crate::protocol::{Action, DeskbridEvent, EventTrigger, Rule};


/// Check whether a given EventTrigger matches a DeskbridEvent.
pub(super) fn trigger_matches_event(trigger: &EventTrigger, event: &DeskbridEvent) -> bool {
    match trigger {
        EventTrigger::ClipboardChanged => {
            // No dedicated clipboard event yet — reserved for future use
            false
        }
        EventTrigger::WindowOpened { app_id: _ } => {
            // No dedicated window-opened event yet — reserved for future
            false
        }
        EventTrigger::WindowClosed { app_id: _ } => {
            // No dedicated window-closed event yet — reserved for future
            false
        }
        EventTrigger::WindowFocused { app_id } => {
            if let DeskbridEvent::WindowFocused {
                window_id: _,
                timestamp: _,
            } = event
            {
                // If app_id filter is set, we'd need to look up the app_id
                // from the window. For now, match any WindowFocused event.
                if let Some(_filter) = app_id {
                    // TODO: resolve app_id from window_id via backend
                    // For now, skip filtered window matches
                    return false;
                }
                true
            } else {
                false
            }
        }
        EventTrigger::SessionLocked
        | EventTrigger::SessionUnlocked
        | EventTrigger::IdleStarted
        | EventTrigger::IdleEnded => {
            // These triggers are reserved for future DeskbridEvent variants
            false
        }
        EventTrigger::FileChanged { path } => {
            match event {
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
                } => {
                    // Simple prefix match — the trigger path is a directory prefix
                    ev_path.starts_with(path)
                }
                _ => false,
            }
        }
        EventTrigger::TimeRange {
            start_hour: _,
            end_hour: _,
            days: _,
        } => {
            // TimeRange triggers are evaluated on a timer, not per-event
            // For now, always skip in event-driven evaluation
            false
        }
        EventTrigger::PresenceChanged { to: _ } => {
            // Reserved for future presence events
            false
        }
    }
}

/// Spawn the rules engine background task.
/// Subscribes to the event broadcast channel and evaluates rules on each event.
pub fn spawn_rules_engine(state: Arc<DaemonState>) {
    tokio::spawn(async move {
        let mut event_rx = state.event_tx.subscribe();
        info!("Rules engine started");

        // Load persisted rules into engine
        {
            let db = state.database.lock().await;
            match db.load_rules() {
                Ok(persisted) => {
                    let mut engine = state.rules.lock().await;
                    engine.load_persisted(persisted);
                    info!("Loaded {} persisted rules", engine.list().len());
                }
                Err(e) => {
                    warn!("Failed to load persisted rules: {}", e);
                }
            }
        }

        loop {
            match event_rx.recv().await {
                Ok(event) => {
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;

                    let to_dispatch: Vec<(Rule, Action)> = {
                        let mut engine = state.rules.lock().await;
                        engine.evaluate(&event, now_ms)
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
