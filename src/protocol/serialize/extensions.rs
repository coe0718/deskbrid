use crate::protocol::Action;
use serde_json::{Value, json};

pub fn serialize_confirmation(action: &Action, id: &str) -> Value {
    match action {
        Action::ConfirmAction { id: confirm_id } => json!({
            "type": "confirmation.confirm",
            "id": id,
            "confirm_id": confirm_id,
        }),
        Action::DenyAction { id: confirm_id } => json!({
            "type": "confirmation.deny",
            "id": id,
            "confirm_id": confirm_id,
        }),
        Action::ConfirmationList => json!({
            "type": "confirmation.list",
            "id": id,
        }),
        _ => serde_json::json!({"error": "not a confirmation action"}),
    }
}

pub fn serialize_agent(action: &Action, id: &str) -> Value {
    match action {
        Action::AgentMessage {
            to_session,
            subject,
            body,
            ttl_ms,
            reply_to,
        } => {
            let mut msg = json!({
                "type": "agent.message",
                "id": id,
                "to_session": to_session,
                "subject": subject,
                "body": body,
            });
            if let Some(ttl) = ttl_ms {
                msg["ttl_ms"] = json!(ttl);
            }
            if let Some(reply) = reply_to {
                msg["reply_to"] = json!(reply);
            }
            msg
        }
        Action::AgentBroadcast {
            subject,
            body,
            exclude_self,
        } => {
            let mut msg = json!({
                "type": "agent.broadcast",
                "id": id,
                "subject": subject,
                "body": body,
            });
            if let Some(ex) = exclude_self {
                msg["exclude_self"] = json!(ex);
            }
            msg
        }
        Action::AgentMailbox => json!({
            "type": "agent.mailbox",
            "id": id,
        }),
        Action::AgentRegister {
            name,
            agent_type,
            capabilities,
            metadata,
            heartbeat_interval_ms,
        } => {
            let mut msg = json!({
                "type": "agent.register",
                "id": id,
                "name": name,
                "capabilities": capabilities,
            });
            if let Some(agent_type) = agent_type {
                msg["agent_type"] = json!(agent_type);
            }
            if let Some(metadata) = metadata {
                msg["metadata"] = metadata.clone();
            }
            if let Some(heartbeat_interval_ms) = heartbeat_interval_ms {
                msg["heartbeat_interval_ms"] = json!(heartbeat_interval_ms);
            }
            msg
        }
        Action::AgentList => json!({
            "type": "agent.list",
            "id": id,
        }),
        Action::AgentGet { name } => json!({
            "type": "agent.get",
            "id": id,
            "name": name,
        }),
        Action::AgentHeartbeat { name } => json!({
            "type": "agent.heartbeat",
            "id": id,
            "name": name,
        }),
        _ => serde_json::json!({"error": "not an agent action"}),
    }
}

pub fn serialize_lock(action: &Action, id: &str) -> Value {
    match action {
        Action::LockAcquire {
            resource,
            holder,
            ttl_ms,
            wait_ms,
            force,
        } => {
            let mut msg = json!({
                "type": "lock.acquire",
                "id": id,
                "resource": resource,
                "force": force,
            });
            if let Some(holder) = holder {
                msg["holder"] = json!(holder);
            }
            if let Some(ttl_ms) = ttl_ms {
                msg["ttl_ms"] = json!(ttl_ms);
            }
            if let Some(wait_ms) = wait_ms {
                msg["wait_ms"] = json!(wait_ms);
            }
            msg
        }
        Action::LockRelease { resource, token } => json!({
            "type": "lock.release",
            "id": id,
            "resource": resource,
            "token": token,
        }),
        Action::LockList => json!({
            "type": "lock.list",
            "id": id,
        }),
        _ => serde_json::json!({"error": "not a lock action"}),
    }
}

pub fn serialize_search(action: &Action, id: &str) -> Value {
    match action {
        Action::UnifiedSearch {
            query,
            categories,
            limit,
        } => {
            let mut msg = json!({
                "type": "search.query",
                "id": id,
                "query": query,
            });
            if let Some(cats) = categories {
                msg["categories"] = json!(cats);
            }
            if let Some(lim) = limit {
                msg["limit"] = json!(lim);
            }
            msg
        }
        Action::UnifiedIndex => json!({
            "type": "search.index",
            "id": id,
        }),
        _ => serde_json::json!({"error": "not a search action"}),
    }
}

pub fn serialize_secrets(action: &Action, id: &str) -> Value {
    match action {
        Action::SecretsListCollections => json!({
            "type": "secrets.list_collections",
            "id": id,
        }),
        Action::SecretsGetSecret { attributes } => {
            let attrs: serde_json::Map<String, Value> = attributes
                .iter()
                .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                .collect();
            json!({
                "type": "secrets.get_secret",
                "id": id,
                "attributes": attrs,
            })
        }
        Action::SecretsStoreSecret {
            attributes,
            secret,
            label,
            collection,
        } => {
            let attrs: serde_json::Map<String, Value> = attributes
                .iter()
                .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                .collect();
            let mut msg = json!({
                "type": "secrets.store_secret",
                "id": id,
                "attributes": attrs,
                "secret": secret,
            });
            if let Some(l) = label {
                msg["label"] = json!(l);
            }
            if let Some(c) = collection {
                msg["collection"] = json!(c);
            }
            msg
        }
        _ => serde_json::json!({"error": "not a secrets action"}),
    }
}
