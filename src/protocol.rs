//! Protocol types — JSON message definitions for the deskbrid protocol.
//!
//! These map 1:1 to the messages defined in PROTOCOL.md.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Messages the client sends to the daemon.
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "subscribe")]
    Subscribe { events: Vec<String> },
    #[serde(rename = "unsubscribe")]
    Unsubscribe { events: Vec<String> },
    #[serde(rename = "action")]
    Action {
        #[serde(default = "uuid_v4")]
        id: String,
        action: String,
        #[serde(default)]
        params: serde_json::Value,
    },
}

fn uuid_v4() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Messages the daemon sends to clients.
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "hello")]
    Hello { version: &'static str, pid: u32 },
    #[serde(rename = "event")]
    Event {
        event: String,
        data: serde_json::Value,
    },
    #[serde(rename = "result")]
    Result {
        id: String,
        ok: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
}

/// Per-connection session state.
#[derive(Debug, Default)]
pub struct Session {
    subscriptions: HashSet<String>,
}

impl Session {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe(&mut self, event: &str) {
        self.subscriptions.insert(event.to_string());
    }

    pub fn unsubscribe(&mut self, event: &str) {
        self.subscriptions.remove(event);
    }

    pub fn is_subscribed(&self, event: &str) -> bool {
        self.subscriptions.contains(event)
    }
}
