use crate::permissions::AutoSuspendConfig;
use crate::protocol::{Action, DeskbridEvent};
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize)]
pub struct SessionSuspension {
    pub session_id: String,
    pub reason: String,
    pub trigger: String,
    pub action_type: Option<String>,
    pub suspended_at: u64,
}

#[derive(Default)]
struct AutoSuspendInner {
    suspended: HashMap<String, SessionSuspension>,
    action_windows: HashMap<(String, String), VecDeque<u64>>,
}

pub struct AutoSuspendStore {
    config: AutoSuspendConfig,
    inner: Mutex<AutoSuspendInner>,
}

impl AutoSuspendStore {
    pub fn new(config: AutoSuspendConfig) -> Self {
        Self {
            config,
            inner: Mutex::new(AutoSuspendInner::default()),
        }
    }

    pub async fn is_suspended(&self, session_id: &str) -> Option<SessionSuspension> {
        let inner = self.inner.lock().await;
        inner.suspended.get(session_id).cloned()
    }

    pub async fn list_suspended(&self) -> Vec<SessionSuspension> {
        let inner = self.inner.lock().await;
        let mut items: Vec<_> = inner.suspended.values().cloned().collect();
        items.sort_by(|a, b| a.session_id.cmp(&b.session_id));
        items
    }

    pub async fn suspend_session(
        &self,
        session_id: &str,
        reason: impl Into<String>,
        trigger: impl Into<String>,
        action_type: Option<String>,
    ) -> Option<DeskbridEvent> {
        if !self.config.enabled {
            return None;
        }

        let mut inner = self.inner.lock().await;
        if inner.suspended.contains_key(session_id) {
            return None;
        }

        let suspension = SessionSuspension {
            session_id: session_id.to_string(),
            reason: reason.into(),
            trigger: trigger.into(),
            action_type,
            suspended_at: now_secs(),
        };
        inner
            .suspended
            .insert(session_id.to_string(), suspension.clone());
        Some(DeskbridEvent::AgentSuspended {
            session_id: suspension.session_id,
            reason: suspension.reason,
            trigger: suspension.trigger,
            action_type: suspension.action_type,
            timestamp: suspension.suspended_at,
        })
    }

    pub async fn resume_session(&self, session_id: &str) -> Option<DeskbridEvent> {
        let mut inner = self.inner.lock().await;
        inner.suspended.remove(session_id)?;
        Some(DeskbridEvent::AgentResumed {
            session_id: session_id.to_string(),
            timestamp: now_secs(),
        })
    }

    pub async fn record_action(&self, session_id: &str, action: &Action) -> Option<DeskbridEvent> {
        if !self.config.enabled || !self.config.suspend_actions {
            return None;
        }

        if let Some(reason) = dangerous_process_reason(action) {
            return self
                .suspend_session(
                    session_id,
                    reason,
                    "dangerous_process_command",
                    Some(action.action_type().to_string()),
                )
                .await;
        }

        let (limit, window_ms, reason) = burst_limit(action.action_type())?;

        let now = now_ms();
        let key = (session_id.to_string(), action.action_type().to_string());
        let mut inner = self.inner.lock().await;
        let window = inner.action_windows.entry(key).or_default();
        while window
            .front()
            .is_some_and(|seen| now.saturating_sub(*seen) > window_ms)
        {
            window.pop_front();
        }
        window.push_back(now);
        if window.len() <= limit {
            return None;
        }
        if inner.suspended.contains_key(session_id) {
            return None;
        }

        let suspension = SessionSuspension {
            session_id: session_id.to_string(),
            reason: reason.to_string(),
            trigger: "suspicious_action_pattern".to_string(),
            action_type: Some(action.action_type().to_string()),
            suspended_at: now_secs(),
        };
        inner
            .suspended
            .insert(session_id.to_string(), suspension.clone());
        Some(DeskbridEvent::AgentSuspended {
            session_id: suspension.session_id,
            reason: suspension.reason,
            trigger: suspension.trigger,
            action_type: suspension.action_type,
            timestamp: suspension.suspended_at,
        })
    }

    pub fn suspend_on_heartbeat_timeout(&self) -> bool {
        self.config.enabled && self.config.suspend_on_heartbeat_timeout
    }
}

fn burst_limit(action_type: &str) -> Option<(usize, u64, &'static str)> {
    match action_type {
        "windows.focus" => Some((10, 1_000, "more than 10 windows.focus actions in 1 second")),
        "files.delete" => Some((5, 10_000, "more than 5 files.delete actions in 10 seconds")),
        _ => None,
    }
}

fn dangerous_process_reason(action: &Action) -> Option<String> {
    let Action::ProcessStart { command, .. } = action else {
        return None;
    };
    let joined = command.join(" ");
    let lowered = joined.to_ascii_lowercase();
    let dangerous = (command.first().is_some_and(|cmd| cmd == "rm")
        && command.iter().any(|arg| arg.contains("rf")))
        || lowered.contains("rm -rf /")
        || lowered.contains(":(){:|:&};:")
        || lowered.contains("mkfs.")
        || lowered.contains("dd if=");

    if dangerous {
        Some(format!("dangerous process command blocked: {joined}"))
    } else {
        None
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn suspends_after_windows_focus_burst() {
        let store = AutoSuspendStore::new(AutoSuspendConfig::default());
        for _ in 0..10 {
            assert!(
                store
                    .record_action("agent", &Action::WindowsFocus("0x1".into()))
                    .await
                    .is_none()
            );
        }
        assert!(
            store
                .record_action("agent", &Action::WindowsFocus("0x1".into()))
                .await
                .is_some()
        );
        assert!(store.is_suspended("agent").await.is_some());
    }
}
