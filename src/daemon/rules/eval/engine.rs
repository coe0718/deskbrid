//! Rules engine background task: subscribes to events, evaluates rules, dispatches actions.

use std::sync::Arc;

use tracing::{debug, error, info, warn};

use crate::DaemonState;
use crate::MAX_RULE_DISPATCH_DEPTH;
use crate::protocol::{Action, Rule};

use super::matching::resolve_event_app_id;

pub fn spawn_rules_engine(state: Arc<DaemonState>) {
    tokio::spawn(async move {
        let mut event_rx = state.event_tx.subscribe();
        info!("Rules engine started");

        // Load persisted rules into engine
        {
            let persisted = {
                let db = state.database.lock().await;
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
                        // Check dispatch depth to prevent rule→action→event→rule loops (W4).
                        let depth = state
                            .rule_dispatch_depth
                            .load(std::sync::atomic::Ordering::Relaxed);
                        if depth >= MAX_RULE_DISPATCH_DEPTH {
                            warn!(
                                "Rule dispatch depth {} reached max {} — blocking rule '{}' action '{}'",
                                depth,
                                MAX_RULE_DISPATCH_DEPTH,
                                rule.name,
                                action.action_type()
                            );
                            continue;
                        }

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
                                    state
                                        .rule_dispatch_depth
                                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                    let result = crate::daemon::dispatch::dispatch_action(
                                        &request_id,
                                        parsed_action,
                                        &state,
                                        0, // peer_uid: rule actions run as daemon
                                        seq,
                                    )
                                    .await;
                                    state
                                        .rule_dispatch_depth
                                        .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
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
