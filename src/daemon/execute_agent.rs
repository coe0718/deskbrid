use crate::protocol::Action;
use serde_json::{Value, json};
use tokio::sync::Mutex;

/// In-memory agent message store with TTL tracking.
pub struct AgentMailboxStore {
    messages: Mutex<Vec<StoredMessage>>,
    next_id: Mutex<u64>,
}

#[derive(Clone, serde::Serialize)]
pub struct StoredMessage {
    pub id: u64,
    pub from_session: String,
    pub to_session: Option<String>,
    pub subject: String,
    pub body: Value,
    pub reply_to: Option<String>,
    pub received_at: u64,
    pub ttl_ms: Option<u64>,
}

impl AgentMailboxStore {
    pub fn new() -> Self {
        Self {
            messages: Mutex::new(Vec::new()),
            next_id: Mutex::new(1),
        }
    }

    pub async fn store(&self, msg: StoredMessage) -> u64 {
        let id = {
            let mut next = self.next_id.lock().await;
            let id = *next;
            *next += 1;
            id
        };
        let mut msgs = self.messages.lock().await;
        // Prune expired before push
        let now_ms = now_ms();
        msgs.retain(|m| !is_expired(m, now_ms));
        msgs.push(StoredMessage { id, ..msg });
        let len = msgs.len();
        if len > 1000 {
            msgs.drain(0..len - 1000);
        }
        id
    }

    pub async fn get_for(&self, session: &str) -> Vec<StoredMessage> {
        let mut msgs = self.messages.lock().await;
        let now_ms = now_ms();
        // Purge expired first
        msgs.retain(|m| !is_expired(m, now_ms));
        msgs.iter()
            .filter(|m| m.to_session.as_deref() == Some(session) || m.to_session.is_none())
            .cloned()
            .collect()
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn is_expired(msg: &StoredMessage, now: u64) -> bool {
    match msg.ttl_ms {
        Some(ttl) => now.saturating_sub(msg.received_at) >= ttl,
        None => false,
    }
}

/// Execute agent messaging actions.
pub async fn execute_agent(
    action: Action,
    state: &crate::DaemonState,
    session_id: &str,
    peer_uid: u32,
) -> anyhow::Result<Value> {
    let now_ms = now_ms();

    match action {
        Action::AgentMessage {
            to_session,
            subject,
            body,
            ttl_ms,
            reply_to,
        } => {
            let msg = StoredMessage {
                id: 0,
                from_session: session_id.to_string(),
                to_session: Some(to_session.clone()),
                subject,
                body,
                reply_to,
                received_at: now_ms,
                ttl_ms,
            };
            let id = state.agent_mailbox.store(msg).await;
            Ok(json!({"status": "sent", "id": id, "to": to_session}))
        }
        Action::AgentBroadcast {
            subject,
            body,
            exclude_self,
        } => {
            let msg = StoredMessage {
                id: 0,
                from_session: session_id.to_string(),
                to_session: None,
                subject,
                body,
                reply_to: None,
                received_at: now_ms,
                ttl_ms: None,
            };
            let id = state.agent_mailbox.store(msg).await;
            Ok(
                json!({"status": "broadcast", "id": id, "exclude_self": exclude_self.unwrap_or(false)}),
            )
        }
        Action::AgentMailbox => {
            let messages = state.agent_mailbox.get_for(session_id).await;
            Ok(json!({"messages": messages, "count": messages.len()}))
        }
        Action::AgentRegister {
            name,
            agent_type,
            capabilities,
            metadata,
            heartbeat_interval_ms,
        } => {
            let (record, inserted) = state
                .agent_registry
                .register(
                    name,
                    agent_type,
                    capabilities,
                    metadata,
                    heartbeat_interval_ms,
                    session_id,
                    peer_uid,
                )
                .await;
            if inserted {
                let _ = state
                    .event_tx
                    .send(crate::protocol::DeskbridEvent::AgentConnected {
                        name: record.name.clone(),
                        session_id: record.session_id.clone(),
                        uid: record.uid,
                        timestamp: crate::daemon::agent_registry::now_secs(),
                    });
            }
            Ok(json!({"registered": true, "agent": agent_to_json(state, record).await}))
        }
        Action::AgentList => {
            let records = state.agent_registry.list().await;
            let mut agents = Vec::with_capacity(records.len());
            for record in records {
                agents.push(agent_to_json(state, record).await);
            }
            let count = agents.len();
            Ok(json!({"agents": agents, "count": count}))
        }
        Action::AgentGet { name } => match state.agent_registry.get(&name).await {
            Some(record) => Ok(json!({"agent": agent_to_json(state, record).await, "found": true})),
            None => Ok(json!({"agent": null, "found": false})),
        },
        Action::AgentHeartbeat { name } => {
            let record = state.agent_registry.heartbeat(&name).await?;
            Ok(json!({"ok": true, "agent": agent_to_json(state, record).await}))
        }
        _ => anyhow::bail!("internal dispatch error: not an agent action"),
    }
}

pub(crate) fn is_agent_action(action: &Action) -> bool {
    matches!(
        action,
        Action::AgentMessage { .. }
            | Action::AgentBroadcast { .. }
            | Action::AgentMailbox
            | Action::AgentRegister { .. }
            | Action::AgentList
            | Action::AgentGet { .. }
            | Action::AgentHeartbeat { .. }
    )
}

async fn agent_to_json(
    state: &crate::DaemonState,
    record: crate::daemon::agent_registry::AgentRecord,
) -> Value {
    let locked_resources = state
        .locks
        .resources_for_holders([record.name.clone(), record.session_id.clone()])
        .await;
    let mut value = serde_json::to_value(record).unwrap_or_else(|_| json!({}));
    value["locked_resources"] = json!(locked_resources);
    value
}
