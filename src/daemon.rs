use crate::protocol;
use crate::DaemonState;
use anyhow::Context;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tracing::{debug, error, info, warn};

const SOCKET_PATH: &str = "/run/user/1000/deskbrid.sock";

pub async fn run() -> anyhow::Result<()> {
    let _ = tokio::fs::remove_file(SOCKET_PATH).await;

    if let Some(parent) = std::path::Path::new(SOCKET_PATH).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let listener = UnixListener::bind(SOCKET_PATH)
        .context("failed to bind Unix socket")?;

    info!("Deskbrid daemon listening on {}", SOCKET_PATH);

    let state = Arc::new(DaemonState::new());

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                debug!("New connection from {:?}", addr);
                let state = Arc::clone(&state);
                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream, &state).await {
                        error!("Client error: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("Accept error: {}", e);
            }
        }
    }
}

async fn handle_client(stream: UnixStream, _state: &DaemonState) -> anyhow::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    let mut seq: u64 = 0;

    let connected = serde_json::json!({
        "type": "connected",
        "id": "server",
        "seq": 0,
        "data": {
            "version": "2.0.0",
            "protocol": "deskbrid-v2"
        }
    });
    writer
        .write_all(format!("{}\n", serde_json::to_string(&connected)?).as_bytes())
        .await?;

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break;
        }

        seq += 1;

        if line.trim().is_empty() {
            continue;
        }

        let msg_type = match extract_type(&line) {
            Some(t) => t,
            None => {
                warn!("Unparseable message: {}", line.trim());
                continue;
            }
        };

        let response = match msg_type.as_str() {
            "ping" => {
                let id = extract_field(&line, "id");
                serde_json::json!({"type": "pong", "id": id, "seq": seq})
            }

            "disconnect" => {
                let id = extract_field(&line, "id");
                let resp = serde_json::json!({"type": "disconnected", "id": id, "seq": seq});
                writer
                    .write_all(format!("{}\n", serde_json::to_string(&resp)?).as_bytes())
                    .await?;
                break;
            }

            _ => {
                let id = extract_field(&line, "id").unwrap_or_else(|| "?".to_string());
                serde_json::json!({
                    "type": "response",
                    "id": id,
                    "seq": seq,
                    "status": "error",
                    "error": {
                        "code": "NOT_SUPPORTED",
                        "message": format!("action '{}' not yet implemented", msg_type)
                    }
                })
            }
        };

        writer
            .write_all(format!("{}\n", serde_json::to_string(&response)?).as_bytes())
            .await?;
    }

    info!("Client disconnected");
    Ok(())
}

fn extract_type(line: &str) -> Option<String> {
    let start = line.find("\"type\"")?;
    let after_key = &line[start..];
    let colon = after_key.find(':')?;
    let value_start = after_key[colon + 1..].trim();
    if value_start.starts_with('"') {
        let end = value_start[1..].find('"')?;
        Some(value_start[1..=end].to_string())
    } else {
        None
    }
}

fn extract_field(line: &str, field: &str) -> Option<String> {
    let search = format!("\"{}\"", field);
    let start = line.find(&search)?;
    let after_key = &line[start + search.len()..];
    let colon = after_key.find(':')?;
    let value_start = after_key[colon + 1..].trim();
    if value_start.starts_with('"') {
        let end = value_start[1..].find('"')?;
        Some(value_start[1..=end].to_string())
    } else {
        None
    }
}
