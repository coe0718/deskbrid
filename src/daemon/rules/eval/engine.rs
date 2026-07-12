//! Rules engine background task: subscribes to events, evaluates rules, dispatches actions.

use std::sync::Arc;

use tracing::{debug, error, info, warn};

use crate::DaemonState;
use crate::max_rule_dispatch_depth;
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
                        // W5 (Vex review): cycle detection via the rule
                        // dispatch chain. If the same rule name is
                        // already on the stack (A→B→A), reject
                        // immediately rather than burning through the
                        // depth budget.
                        {
                            let chain = state
                                .rule_dispatch_chain
                                .lock()
                                .unwrap_or_else(|p| p.into_inner());
                            if chain.iter().any(|name| name == &rule.name) {
                                warn!(
                                    "Rule '{}' would re-enter itself (chain: {:?}) — skipping action '{}' to prevent cycle",
                                    rule.name,
                                    chain,
                                    action.action_type()
                                );
                                continue;
                            }
                        }

                        // Check dispatch depth to prevent rule→action→event→rule loops (W4).
                        // W26 (CODE_REVIEW_VEX_V1.md): depth cap is env-configurable
                        // via DESKBRID_MAX_RULE_DISPATCH_DEPTH.
                        let depth = state
                            .rule_dispatch_depth
                            .load(std::sync::atomic::Ordering::Relaxed);
                        let max_depth = max_rule_dispatch_depth();
                        if depth >= max_depth {
                            warn!(
                                "Rule dispatch depth {} reached max {} — blocking rule '{}' action '{}'",
                                depth,
                                max_depth,
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
                                let rule_name = rule.name.clone();
                                tokio::spawn(async move {
                                    // W5: push the rule name onto the chain before dispatch,
                                    // pop after. Held for the duration of the dispatch so a
                                    // re-entrant rule firing from the same chain is rejected.
                                    {
                                        let mut chain = state
                                            .rule_dispatch_chain
                                            .lock()
                                            .unwrap_or_else(|p| p.into_inner());
                                        chain.push(rule_name.clone());
                                    }
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
                                    {
                                        let mut chain = state
                                            .rule_dispatch_chain
                                            .lock()
                                            .unwrap_or_else(|p| p.into_inner());
                                        if chain.last() == Some(&rule_name) {
                                            chain.pop();
                                        }
                                    }
                                    debug!(
                                        "Rule '{}' action completed: {:?}",
                                        rule_name,
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
