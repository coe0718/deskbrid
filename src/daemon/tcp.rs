use crate::DaemonState;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::{debug, error, info, warn};

use super::client::handle_client_tcp;

/// Synthetic UID assigned to TCP connections. Permissions for TCP clients
/// are configured under `[permissions."uid:4294967294"]` in permissions.toml.
pub const TCP_EFFECTIVE_UID: u32 = 0xFFFF_FFFE;

/// Max size of an auth message line (4KB).
const MAX_AUTH_LINE: u64 = 4096;

/// Read one newline-terminated line without buffering bytes after the newline.
/// This preserves pipelined messages sent immediately after the auth line.
pub(crate) async fn read_limited_line<R: AsyncRead + Unpin>(
    reader: &mut R,
    max_bytes: usize,
) -> std::io::Result<String> {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        let n = reader.read(&mut byte).await?;
        if n == 0 {
            break;
        }
        buf.push(byte[0]);
        if buf.len() > max_bytes {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("line exceeds {max_bytes} byte limit"),
            ));
        }
        if byte[0] == b'\n' {
            break;
        }
    }
    String::from_utf8(buf).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

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
    let (mut reader, mut writer) = tokio::io::split(stream);
    let auth_line = match read_limited_line(&mut reader, MAX_AUTH_LINE as usize).await {
        Ok(line) => line,
        Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {
            warn!("TCP auth message invalid or too large: {}", e);
            let err = serde_json::json!({
                "type": "error", "id": "auth", "status": "error",
                "error": { "code": "INVALID_PARAMS", "message": "Auth message too large or invalid (max 4KB)" }
            });
            let _ = writer
                .write_all(format!("{}\n", serde_json::to_string(&err)?).as_bytes())
                .await;
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    };

    // Reject oversized auth messages
    if auth_line.len() > MAX_AUTH_LINE as usize {
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

    // Constant-time token comparison — no short-circuiting per-character
    if !constant_time_eq(provided, token) {
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
    let stream = reader.unsplit(writer);
    handle_client_tcp(stream, TCP_EFFECTIVE_UID, state).await
}

/// Generate a random 32-character hex token suitable for TCP auth.
pub fn generate_token() -> String {
    uuid::Uuid::new_v4().to_string().replace('-', "")
}

/// Constant-time string comparison. Resistant to timing side-channel attacks
/// because it compares all bytes regardless of where a mismatch occurs.
pub(crate) fn constant_time_eq(a: &str, b: &str) -> bool {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    // Length check: different lengths → not equal (minor length leak, but
    // token length is already visible in the JSON message size).
    if a_bytes.len() != b_bytes.len() {
        return false;
    }
    // XOR all bytes — no short-circuit, every byte pair is compared
    a_bytes
        .iter()
        .zip(b_bytes.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;

    #[tokio::test]
    async fn read_limited_line_preserves_pipelined_bytes() {
        let (mut client, mut server) = tokio::io::duplex(64);
        let expected = br#"{"type":"ping"}"#;

        client.write_all(b"auth\n").await.unwrap();
        client.write_all(expected).await.unwrap();
        client.write_all(b"\n").await.unwrap();

        let line = read_limited_line(&mut server, 16).await.unwrap();
        assert_eq!(line, "auth\n");

        let mut rest = vec![0; expected.len() + 1];
        server.read_exact(&mut rest).await.unwrap();
        assert_eq!(rest, [expected.as_slice(), b"\n"].concat());
    }

    #[tokio::test]
    async fn read_limited_line_rejects_oversized_line() {
        let (mut client, mut server) = tokio::io::duplex(64);

        client.write_all(b"abcdef\n").await.unwrap();
        let err = read_limited_line(&mut server, 4).await.unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }
}
