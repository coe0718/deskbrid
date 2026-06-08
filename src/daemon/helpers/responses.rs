//! JSON response builders for daemon protocol responses.

pub fn ok_response(id: &str, seq: u64) -> serde_json::Value {
    serde_json::json!({"type": "response", "id": id, "seq": seq, "status": "ok", "data": {}})
}

pub fn not_supported_response(request_id: &str, msg: &str, seq: u64) -> serde_json::Value {
    serde_json::json!({
        "type": "response", "id": request_id, "seq": seq, "status": "error",
        "error": { "code": "NOT_SUPPORTED", "message": msg }
    })
}

pub fn permission_denied_response(
    request_id: &str,
    action_type: &str,
    seq: u64,
) -> serde_json::Value {
    serde_json::json!({
        "type": "response", "id": request_id, "seq": seq, "status": "error",
        "error": { "code": "PERMISSION_DENIED", "message": format!("action not permitted: {action_type} requires explicit permission — add '{action_type}' to your permissions.toml") }
    })
}
