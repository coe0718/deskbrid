//! Event-driven Rules Engine (#83).
//!
//! Listens to `DeskbridEvent`s from the broadcast channel and evaluates
//! registered rules. When a rule's trigger matches the event (and any
//! optional condition holds), the associated action is dispatched.

use crate::DaemonState;
use crate::protocol::Action;
use crate::protocol::{DeskbridEvent, Rule};
use std::collections::HashMap;
use tracing::{debug, error};

/// Per-rule runtime state: tracks fire count and last-fire timestamp.
#[derive(Debug, Default)]
struct RuleRuntime {
    fire_count: u32,
    last_fire_ms: u64,
}

/// The in-memory rules engine — holds registered rules plus runtime state.
pub struct RuleEngine {
    rules: Vec<Rule>,
    pub(super) runtime: HashMap<String, RuleRuntime>,
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
        let active_ids: Vec<String> = self.rules.iter().map(|r| r.id.clone()).collect();
        self.runtime.retain(|k, _| active_ids.contains(k));
    }

    /// Evaluate an event against all enabled rules and return the list
    /// of actions to dispatch. Conditions are checked against `state`.
    pub fn evaluate(
        &mut self,
        event: &DeskbridEvent,
        now_ms: u64,
        state: &DaemonState,
    ) -> Vec<(Rule, Action)> {
        let mut actions: Vec<(Rule, Action)> = Vec::new();

        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            if !eval::trigger_matches_event(&rule.trigger, event) {
                continue;
            }

            // Check condition (VarEquals, VarExists)
            if !eval::condition_matches(&rule.condition, state, event) {
                debug!("Rule '{}' condition not met", rule.name);
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
                    let rt = self.runtime.entry(rule.id.clone()).or_default();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{EventTrigger, Rule};

    fn make_rule(id: &str, name: &str, trigger: EventTrigger) -> Rule {
        Rule {
            id: id.into(),
            name: name.into(),
            trigger,
            condition: None,
            action_type: "notification.send".into(),
            action_params: serde_json::json!({"title": "fired"}),
            enabled: true,
            cooldown_ms: None,
            max_fires: None,
        }
    }

    #[test]
    fn rule_engine_register_and_list() {
        let mut engine = RuleEngine::new();
        engine.register(make_rule("r1", "Rule 1", EventTrigger::ClipboardChanged));
        engine.register(make_rule("r2", "Rule 2", EventTrigger::SessionLocked));
        assert_eq!(engine.list().len(), 2);
    }

    #[test]
    fn rule_engine_register_duplicate_id_overwrites() {
        let mut engine = RuleEngine::new();
        engine.register(make_rule("r1", "Original", EventTrigger::ClipboardChanged));
        engine.register(make_rule("r1", "Replacement", EventTrigger::IdleStarted));
        let list = engine.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Replacement");
    }

    #[test]
    fn rule_engine_remove() {
        let mut engine = RuleEngine::new();
        engine.register(make_rule("r1", "Rule", EventTrigger::ClipboardChanged));
        assert!(engine.remove("r1").is_some());
        assert!(engine.list().is_empty());
        assert!(engine.remove("nonexistent").is_none());
    }

    #[test]
    fn rule_engine_set_enabled() {
        let mut engine = RuleEngine::new();
        engine.register(make_rule("r1", "Rule", EventTrigger::ClipboardChanged));

        assert!(engine.set_enabled("r1", false));
        assert!(!engine.list()[0].enabled);

        assert!(engine.set_enabled("r1", true));
        assert!(engine.list()[0].enabled);
    }

    #[test]
    fn rule_engine_get() {
        let mut engine = RuleEngine::new();
        engine.register(make_rule("r1", "My Rule", EventTrigger::ClipboardChanged));

        let r = engine.get("r1").unwrap();
        assert_eq!(r.name, "My Rule");
        assert!(engine.get("nonexistent").is_none());
    }

    #[test]
    fn rule_disabled_does_not_evaluate() {
        let mut engine = RuleEngine::new();
        let mut rule = make_rule("r1", "Off", EventTrigger::ClipboardChanged);
        rule.enabled = false;
        engine.register(rule);

        let state = DaemonState::new();
        let results = engine.evaluate(
            &crate::protocol::DeskbridEvent::WindowFocused {
                window_id: "x".into(),
                timestamp: 0,
            },
            1000,
            &state,
        );
        assert!(results.is_empty());
    }

    #[test]
    fn window_opened_trigger_matches_no_filter() {
        let mut engine = RuleEngine::new();
        engine.register({
            let mut r = make_rule(
                "r1",
                "AnyWindow",
                EventTrigger::WindowOpened { app_id: None },
            );
            r.action_type = "system.info".into();
            r
        });

        let state = DaemonState::new();
        let results = engine.evaluate(
            &DeskbridEvent::WindowOpened {
                window_id: "w1".into(),
                app_id: Some("code".into()),
                timestamp: 1000,
            },
            1000,
            &state,
        );
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn window_opened_trigger_filters_by_app_id() {
        let mut engine = RuleEngine::new();
        engine.register({
            let mut r = make_rule(
                "r1",
                "CodeOnly",
                EventTrigger::WindowOpened {
                    app_id: Some("code".into()),
                },
            );
            r.action_type = "system.info".into();
            r
        });

        let state = DaemonState::new();
        // Matching app_id
        let results = engine.evaluate(
            &DeskbridEvent::WindowOpened {
                window_id: "w1".into(),
                app_id: Some("code".into()),
                timestamp: 1000,
            },
            1000,
            &state,
        );
        assert_eq!(results.len(), 1);

        // Non-matching app_id
        let results = engine.evaluate(
            &DeskbridEvent::WindowOpened {
                window_id: "w2".into(),
                app_id: Some("firefox".into()),
                timestamp: 2000,
            },
            2000,
            &state,
        );
        assert_eq!(results.len(), 0);
    }
}

mod eval;
pub use eval::{spawn_rules_engine, spawn_timerange_evaluator};
