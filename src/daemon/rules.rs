//! Event-driven Rules Engine (#83).
//!
//! Listens to `DeskbridEvent`s from the broadcast channel and evaluates
//! registered rules. When a rule's trigger matches the event (and any
//! optional condition holds), the associated action is dispatched.

use crate::DaemonState;
use crate::protocol::Action;
use crate::protocol::{DeskbridEvent, EventTrigger, Rule};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Per-rule runtime state: tracks fire count and last-fire timestamp.
#[derive(Debug)]
struct RuleRuntime {
    fire_count: u32,
    last_fire_ms: u64,
}

impl RuleRuntime {
    fn new() -> Self {
        Self {
            fire_count: 0,
            last_fire_ms: 0,
        }
    }
}

/// The in-memory rules engine — holds registered rules plus runtime state.
pub struct RuleEngine {
    rules: Vec<Rule>,
    runtime: HashMap<String, RuleRuntime>,
}

impl RuleEngine {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            runtime: HashMap::new(),
        }
    }

    /// Register (or replace) a rule.
    pub fn register(&mut self, rule: Rule) {
        // Remove old version if exists
        self.rules.retain(|r| r.id != rule.id);
        self.rules.push(rule);
    }

    /// Remove a rule by id. Returns the removed rule if found.
    pub fn remove(&mut self, rule_id: &str) -> Option<Rule> {
        self.runtime.remove(rule_id);
        let pos = self.rules.iter().position(|r| r.id == rule_id)?;
        Some(self.rules.remove(pos))
    }

    /// Set the enabled flag for a rule. Returns true if found.
    pub fn set_enabled(&mut self, rule_id: &str, enabled: bool) -> bool {
        if let Some(rule) = self.rules.iter_mut().find(|r| r.id == rule_id) {
            rule.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// Get a rule by id.
    pub fn get(&self, rule_id: &str) -> Option<&Rule> {
        self.rules.iter().find(|r| r.id == rule_id)
    }

    /// List all rules.
    pub fn list(&self) -> &[Rule] {
        &self.rules
    }

    /// Load persisted rules into the engine.
    pub fn load_persisted(&mut self, rules: Vec<Rule>) {
        self.rules = rules;
        // Clear runtime for any stale entries
        let active_ids: Vec<String> = self.rules.iter().map(|r| r.id.clone()).collect();
        self.runtime.retain(|k, _| active_ids.contains(k));
    }

    /// Evaluate an event against all enabled rules and return the list
    /// of actions to dispatch.
    pub fn evaluate(&mut self, event: &DeskbridEvent, now_ms: u64) -> Vec<(Rule, Action)> {
        let mut actions: Vec<(Rule, Action)> = Vec::new();

        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            if !trigger_matches_event(&rule.trigger, event) {
                continue;
            }

            // Check cooldown
            if let Some(cooldown_ms) = rule.cooldown_ms {
                let rt = self.runtime.get(&rule.id);
                if let Some(rt) = rt
                    && now_ms.saturating_sub(rt.last_fire_ms) < cooldown_ms
                {
                    debug!("Rule '{}' is on cooldown", rule.name);
                    continue;
                }
            }

            // Check max_fires
            if let Some(max_fires) = rule.max_fires {
                let count = self
                    .runtime
                    .get(&rule.id)
                    .map(|r| r.fire_count)
                    .unwrap_or(0);
                if count >= max_fires {
                    debug!("Rule '{}' has reached max_fires ({})", rule.name, max_fires);
                    continue;
                }
            }

            // Build the action JSON and parse it
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
                    // Update runtime
                    let rt = self
                        .runtime
                        .entry(rule.id.clone())
                        .or_insert_with(RuleRuntime::new);
                    rt.fire_count += 1;
                    rt.last_fire_ms = now_ms;

                    actions.push((rule.clone(), action));
                }
                Err(e) => {
                    error!("Failed to parse rule '{}' action: {}", rule.name, e);
                }
            }
        }

        actions
    }
}

/// Check whether a given EventTrigger matches a DeskbridEvent.
fn trigger_matches_event(trigger: &EventTrigger, event: &DeskbridEvent) -> bool {
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
