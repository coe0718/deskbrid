use crate::protocol::DeskbridEvent;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use tokio::sync::Mutex;

const HEARTBEAT_TIMEOUT_MULTIPLIER: u64 = 3;

#[derive(Debug, Clone, Serialize)]
pub struct AgentRecord {
    pub name: String,
    pub agent_type: Option<String>,
    pub capabilities: Vec<String>,
    pub metadata: Value,
    pub session_id: String,
    pub uid: u32,
    pub connected_at: u64,
    pub last_seen: u64,
    pub last_action: Option<String>,
    pub heartbeat_interval_ms: Option<u64>,
    pub heartbeat_timed_out: bool,
    pub subscriptions: usize,
    pub terminals: Vec<String>,
    #[serde(skip_serializing)]
    timeout_emitted: bool,
}

#[derive(Default)]
struct AgentRegistryInner {
    agents: HashMap<String, AgentRecord>,
    session_counts: HashMap<String, u32>,
}

#[derive(Default)]
pub struct AgentRegistry {
    inner: Mutex<AgentRegistryInner>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn connect_session(&self, session_id: &str, uid: u32) -> Option<DeskbridEvent> {
        let mut inner = self.inner.lock().await;
        let count = inner
            .session_counts
            .entry(session_id.to_string())
            .or_insert(0);
        *count += 1;

        let now = now_ms();
        let existed = inner.agents.contains_key(session_id);
        let record = inner
            .agents
            .entry(session_id.to_string())
            .or_insert_with(|| AgentRecord {
                name: session_id.to_string(),
                agent_type: Some("session".to_string()),
                capabilities: Vec::new(),
                metadata: json!({"auto_registered": true}),
                session_id: session_id.to_string(),
                uid,
                connected_at: now,
                last_seen: now,
                last_action: None,
                heartbeat_interval_ms: None,
                heartbeat_timed_out: false,
                subscriptions: 0,
                terminals: Vec::new(),
                timeout_emitted: false,
            });
        record.uid = uid;
        record.last_seen = now;

        if existed {
            None
        } else {
            Some(DeskbridEvent::AgentConnected {
                name: record.name.clone(),
                session_id: record.session_id.clone(),
                uid: record.uid,
                timestamp: now_secs(),
            })
        }
    }

    pub async fn disconnect_session(&self, session_id: &str) -> Vec<AgentRecord> {
        let mut inner = self.inner.lock().await;
        if let Some(count) = inner.session_counts.get_mut(session_id)
            && *count > 1
        {
            *count -= 1;
            return Vec::new();
        }
        inner.session_counts.remove(session_id);

        let names: Vec<String> = inner
            .agents
            .iter()
            .filter(|(_, record)| record.session_id == session_id)
            .map(|(name, _)| name.clone())
            .collect();
        names
            .into_iter()
            .filter_map(|name| inner.agents.remove(&name))
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn register(
        &self,
        name: String,
        agent_type: Option<String>,
        capabilities: Vec<String>,
        metadata: Option<Value>,
        heartbeat_interval_ms: Option<u64>,
        session_id: &str,
        uid: u32,
    ) -> (AgentRecord, bool) {
        let mut inner = self.inner.lock().await;
        let now = now_ms();
        let previous = inner.agents.get(&name).cloned();
        let inserted = previous.is_none();
        let connected_at = previous
            .as_ref()
            .map(|record| record.connected_at)
            .unwrap_or(now);
        let record = AgentRecord {
            name: name.clone(),
            agent_type,
            capabilities,
            metadata: metadata.unwrap_or_else(|| json!({})),
            session_id: session_id.to_string(),
            uid,
            connected_at,
            last_seen: now,
            last_action: previous
                .as_ref()
                .and_then(|record| record.last_action.clone()),
            heartbeat_interval_ms,
            heartbeat_timed_out: false,
            subscriptions: previous
                .as_ref()
                .map(|record| record.subscriptions)
                .unwrap_or_default(),
            terminals: previous.map(|record| record.terminals).unwrap_or_default(),
            timeout_emitted: false,
        };
        inner.agents.insert(name, record.clone());
        (record, inserted)
    }

    pub async fn heartbeat(&self, name: &str) -> anyhow::Result<AgentRecord> {
        let mut inner = self.inner.lock().await;
        let record = inner
            .agents
            .get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("agent '{}' is not registered", name))?;
        record.last_seen = now_ms();
        record.heartbeat_timed_out = false;
        record.timeout_emitted = false;
        Ok(record.clone())
    }

    pub async fn get(&self, name: &str) -> Option<AgentRecord> {
        let mut inner = self.inner.lock().await;
        refresh_heartbeat_status(&mut inner);
        inner.agents.get(name).cloned()
    }

    pub async fn list(&self) -> Vec<AgentRecord> {
        let mut inner = self.inner.lock().await;
        refresh_heartbeat_status(&mut inner);
        let mut records: Vec<_> = inner.agents.values().cloned().collect();
        records.sort_by(|a, b| a.name.cmp(&b.name));
        records
    }

    pub async fn record_action(&self, session_id: &str, action_type: &str) {
        let mut inner = self.inner.lock().await;
        let now = now_ms();
        for record in inner.agents.values_mut() {
            if record.session_id == session_id {
                record.last_seen = now;
                record.last_action = Some(action_type.to_string());
            }
        }
    }

    pub async fn set_subscriptions(&self, session_id: &str, count: usize) {
        let mut inner = self.inner.lock().await;
        for record in inner.agents.values_mut() {
            if record.session_id == session_id {
                record.subscriptions = count;
            }
        }
    }

    pub async fn heartbeat_timeout_events(&self) -> Vec<DeskbridEvent> {
        let mut inner = self.inner.lock().await;
        let now = now_ms();
        let mut events = Vec::new();
        for record in inner.agents.values_mut() {
            let timed_out = heartbeat_timed_out(record, now);
            record.heartbeat_timed_out = timed_out;
            if timed_out && !record.timeout_emitted {
                record.timeout_emitted = true;
                events.push(DeskbridEvent::AgentHeartbeatTimeout {
                    name: record.name.clone(),
                    session_id: record.session_id.clone(),
                    last_seen: record.last_seen,
                    timestamp: now_secs(),
                });
            }
        }
        events
    }
}

pub fn spawn_heartbeat_sweeper(state: std::sync::Arc<crate::DaemonState>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            for event in state.agent_registry.heartbeat_timeout_events().await {
                let _ = state.event_tx.send(event);
            }
        }
    });
}

fn refresh_heartbeat_status(inner: &mut AgentRegistryInner) {
    let now = now_ms();
    for record in inner.agents.values_mut() {
        record.heartbeat_timed_out = heartbeat_timed_out(record, now);
    }
}

fn heartbeat_timed_out(record: &AgentRecord, now: u64) -> bool {
    match record.heartbeat_interval_ms {
        Some(interval) if interval > 0 => {
            now.saturating_sub(record.last_seen)
                > interval.saturating_mul(HEARTBEAT_TIMEOUT_MULTIPLIER)
        }
        _ => false,
    }
}

pub(crate) fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub(crate) fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
