use crate::DaemonState;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tracing::{debug, error, info, warn};

use super::client::handle_client_tcp;

/// Synthetic UID assigned to TCP connections. Permissions for TCP clients
/// are configured under `[permissions."uid:4294967294"]` in permissions.toml.
pub const TCP_EFFECTIVE_UID: u32 = 0xFFFF_FFFE;

/// Start a TCP listener on the given bind address. Authenticates with bearer token
/// before handing off to the standard client handler.
pub async fn run_tcp_listener(
    bind: String,
    token: String,
    state: Arc<DaemonState>,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(&bind).await?;
    info!("Deskbrid TCP listener on {} (token auth)", bind);

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                debug!("TCP connection from {}", addr);
                let state = Arc::clone(&state);
                let token = token.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_tcp_connection(stream, &token, &state).await {
                        error!("TCP client error ({}): {}", addr, e);
                    }
                });
            }
            Err(e) => {
                error!("TCP accept error: {}", e);
            }
        }
    }
}

/// Authenticate a TCP connection then hand off to the generic client handler.
async fn handle_tcp_connection(
    stream: tokio::net::TcpStream,
    token: &str,
    state: &DaemonState,
) -> anyhow::Result<()> {
    let (reader, mut writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(reader);

    // Read auth message
    let mut auth_line = String::new();
    reader.read_line(&mut auth_line).await?;

    // Reject oversized auth messages
    if auth_line.len() > 4096 {
        warn!(
            "TCP auth message too large ({} bytes) — rejecting",
            auth_line.len()
        );
        let err = serde_json::json!({
            "type": "error", "id": "auth", "status": "error",
            "error": { "code": "INVALID_PARAMS", "message": "Auth message too large (max 4KB)" }
        });
        let _ = writer
            .write_all(format!("{}\n", serde_json::to_string(&err)?).as_bytes())
            .await;
        return Ok(());
    }

    if auth_line.trim().is_empty() {
        warn!("TCP client sent empty auth message — rejecting");
        let err = serde_json::json!({
            "type": "error",
            "id": "auth",
            "status": "error",
            "error": {
                "code": "UNAUTHORIZED",
                "message": "Authentication required. Send {\"type\":\"auth\",\"token\":\"...\"}"
            }
        });
        let _ = writer
            .write_all(format!("{}\n", serde_json::to_string(&err)?).as_bytes())
            .await;
        return Ok(());
    }

    let auth: serde_json::Value = match serde_json::from_str(&auth_line) {
        Ok(v) => v,
        Err(e) => {
            warn!("TCP client sent invalid JSON auth: {}", e);
            let err = serde_json::json!({
                "type": "error", "id": "auth", "status": "error",
                "error": {
                    "code": "INVALID_PARAMS",
                    "message": format!("Invalid auth JSON: {}", e)
                }
            });
            let _ = writer
                .write_all(format!("{}\n", serde_json::to_string(&err)?).as_bytes())
                .await;
            return Ok(());
        }
    };

    if auth.get("type") != Some(&serde_json::Value::String("auth".into())) {
        warn!("TCP client sent non-auth first message");
        let err = serde_json::json!({
            "type": "error", "id": "auth", "status": "error",
            "error": {
                "code": "UNAUTHORIZED",
                "message": "First message must be {\"type\":\"auth\",\"token\":\"...\"}"
            }
        });
        let _ = writer
            .write_all(format!("{}\n", serde_json::to_string(&err)?).as_bytes())
            .await;
        return Ok(());
    }

    let provided = auth.get("token").and_then(|v| v.as_str()).unwrap_or("");

    if provided != token {
        warn!("TCP client sent invalid token");
        let err = serde_json::json!({
            "type": "error", "id": "auth", "status": "error",
            "error": { "code": "UNAUTHORIZED", "message": "Invalid token" }
        });
        let _ = writer
            .write_all(format!("{}\n", serde_json::to_string(&err)?).as_bytes())
            .await;
        return Ok(());
    }

    // Reassemble stream from split halves for the generic handler
    let stream = reader.into_inner().unsplit(writer);
    handle_client_tcp(stream, TCP_EFFECTIVE_UID, state).await
}

/// Generate a random 32-character hex token suitable for TCP auth.
pub fn generate_token() -> String {
    uuid::Uuid::new_v4().to_string().replace('-', "")
}
