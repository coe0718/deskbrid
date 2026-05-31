// Rules types for the event-driven rules engine (#83).
// EventTrigger, RuleCondition, and Rule are serializable so they can
// be persisted to the database and sent over the protocol.

use serde::{Deserialize, Serialize};

/// What kind of event fires a rule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum EventTrigger {
    #[serde(rename = "clipboard.changed")]
    ClipboardChanged,
    #[serde(rename = "window.opened")]
    WindowOpened {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        app_id: Option<String>,
    },
    #[serde(rename = "window.closed")]
    WindowClosed {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        app_id: Option<String>,
    },
    #[serde(rename = "window.focused")]
    WindowFocused {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        app_id: Option<String>,
    },
    #[serde(rename = "session.locked")]
    SessionLocked,
    #[serde(rename = "session.unlocked")]
    SessionUnlocked,
    #[serde(rename = "idle.started")]
    IdleStarted,
    #[serde(rename = "idle.ended")]
    IdleEnded,
    #[serde(rename = "file.changed")]
    FileChanged { path: String },
    #[serde(rename = "time.range")]
    TimeRange {
        start_hour: u8,
        end_hour: u8,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        days: Vec<u8>,
    },
    #[serde(rename = "presence.changed")]
    PresenceChanged { to: String },
}

/// Optional extra condition that must also hold for the rule to fire.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum RuleCondition {
    /// Check that a session variable matches a value.
    #[serde(rename = "var.equals")]
    VarEquals { name: String, value: String },
    /// Check that a session variable exists.
    #[serde(rename = "var.exists")]
    VarExists { name: String },
}

/// A single user-configured rule: when trigger + condition match, the action fires.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub trigger: EventTrigger,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<RuleCondition>,
    /// The action type string (e.g. "notification.send", "screenshot").
    pub action_type: String,
    /// Optional action parameters as a JSON object.
    #[serde(default)]
    pub action_params: serde_json::Value,
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_fires: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cooldown_ms: Option<u64>,
}
