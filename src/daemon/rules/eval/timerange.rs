//! TimeRange rule evaluation on a background timer.

use std::sync::Arc;

use tracing::{debug, error, info};

use crate::DaemonState;
use crate::protocol::{Action, DeskbridEvent, EventTrigger, Rule};

use super::matching::condition_matches;

pub(crate) async fn evaluate_timerange_rules(state: &Arc<DaemonState>, now_ms: u64) {
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
