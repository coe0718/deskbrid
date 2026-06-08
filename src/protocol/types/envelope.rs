//! Protocol envelope, request options, error types, keyboard layouts, macro types.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Envelope {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorBody>,
}

#[derive(Debug, Clone, Default)]
pub struct RequestOptions {
    pub dry_run: bool,
    pub timeout_ms: Option<u64>,
    pub require_confirmation: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeyboardLayout {
    pub index: u32,
    pub name: String,
    pub variant: Option<String>,
    pub display_name: Option<String>,
}

// ─── Macro Recording & Replay ──────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RecordedAction {
    pub seq: u64,
    pub timestamp: u64,
    pub elapsed_ms: u64,
    pub action_type: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MacroSummary {
    pub name: String,
    pub description: Option<String>,
    pub action_count: usize,
    pub total_duration_ms: u64,
    pub created_at: u64,
}
