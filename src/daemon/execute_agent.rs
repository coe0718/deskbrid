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
        _ => anyhow::bail!("internal dispatch error: not an agent action"),
    }
}
