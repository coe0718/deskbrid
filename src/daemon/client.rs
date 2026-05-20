use crate::permissions::socket_peer_uid;
use crate::protocol::Action;
use crate::{ConnectionState, DaemonState};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tracing::{info, warn};

use super::dispatch::dispatch_action;
use super::helpers::ok_response;

pub async fn handle_client(stream: UnixStream, state: &DaemonState) -> anyhow::Result<()> {
    let peer_uid = socket_peer_uid(&stream).unwrap_or(u32::MAX);
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut conn = ConnectionState::default();
    let mut line = String::new();
    let mut seq: u64 = 0;

    let connected = serde_json::json!({
        "type": "connected",
        "id": "server",
        "seq": 0,
        "data": { "version": "0.6.0", "protocol": "deskbrid-v2", "uid": peer_uid }
    });
    writer
        .write_all(format!("{}\n", serde_json::to_string(&connected)?).as_bytes())
        .await?;

    // Spawn event forwarder: reads from broadcast and pushes matching events to this client
    let mut event_rx = state.event_tx.subscribe();
    let (event_tx, mut event_rx_forward) = tokio::sync::mpsc::unbounded_channel::<String>();

    tokio::spawn(async move {
        loop {
            match event_rx.recv().await {
                Ok(evt) => {
                    if let Ok(json) = serde_json::to_string(&evt) {
                        let _ = event_tx.send(json);
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    loop {
        tokio::select! {
            // Check for pending events to forward
            event_msg = event_rx_forward.recv() => {
                if let Some(msg) = event_msg {
                    // Parse the event to get its type for subscription matching
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&msg) {
                        let event_type = parsed["event"].as_str().unwrap_or("");
                        if event_matches_any(&conn.subscriptions, event_type) {
                            let envelope = serde_json::json!({
                                "type": "event",
                                "id": event_type,
                                "data": parsed
                            });
                            if let Ok(out) = serde_json::to_string(&envelope) {
                                let _ = writer.write_all(format!("{}\n", out).as_bytes()).await;
                            }
                        }
                    }
                }
            }

            // Read next client command
            result = reader.read_line(&mut line) => {
                let n = result?;
                if n == 0 {
                    break;
                }

                seq += 1;
                if line.trim().is_empty() {
                    line.clear();
                    continue;
                }

                let action = match Action::from_json(&line) {
                    Ok((_, action)) => action,
                    Err(e) => {
                        warn!("Failed to parse message: {}", e);
                        let err = serde_json::json!({
                            "type": "response", "id": "?", "seq": seq, "status": "error",
                            "error": { "code": "INVALID_PARAMS", "message": format!("{}", e) }
                        });
                        writer
                            .write_all(format!("{}\n", serde_json::to_string(&err)?).as_bytes())
                            .await?;
                        line.clear();
                        continue;
                    }
                };
                line.clear();

                match action {
                    Action::Disconnect => {
                        let resp = serde_json::json!({"type": "disconnected", "id": "dc", "seq": seq});
                        writer
                            .write_all(format!("{}\n", serde_json::to_string(&resp)?).as_bytes())
                            .await?;
                        break;
                    }

                    Action::Ping => {
                        let resp = serde_json::json!({"type": "pong", "id": "ping", "seq": seq});
                        writer
                            .write_all(format!("{}\n", serde_json::to_string(&resp)?).as_bytes())
                            .await?;
                    }

                    Action::Subscribe { events } => {
                        for evt in &events {
                            conn.subscriptions.insert(evt.clone());
                        }
                        let resp = ok_response("subscribe", seq);
                        writer
                            .write_all(format!("{}\n", serde_json::to_string(&resp)?).as_bytes())
                            .await?;
                    }

                    Action::Unsubscribe { events } => {
                        for evt in &events {
                            conn.subscriptions.remove(evt);
                        }
                        let resp = ok_response("unsubscribe", seq);
                        writer
                            .write_all(format!("{}\n", serde_json::to_string(&resp)?).as_bytes())
                            .await?;
                    }

                    // Files — track watched paths locally
                    Action::FilesWatch { ref path, .. } => {
                        conn.watched_paths.insert(path.clone());
                        let resp = dispatch_action(action, state, peer_uid, seq).await;
                        writer
                            .write_all(format!("{}\n", serde_json::to_string(&resp)?).as_bytes())
                            .await?;
                    }
                    Action::FilesUnwatch { ref path } => {
                        conn.watched_paths.remove(path);
                        let resp = dispatch_action(action, state, peer_uid, seq).await;
                        writer
                            .write_all(format!("{}\n", serde_json::to_string(&resp)?).as_bytes())
                            .await?;
                    }

                    _ => {
                        let resp = dispatch_action(action, state, peer_uid, seq).await;
                        writer
                            .write_all(format!("{}\n", serde_json::to_string(&resp)?).as_bytes())
                            .await?;
                    }
                }
            }
        }
    }

    info!("Client disconnected");
    Ok(())
}

/// Check if an event type matches any subscription glob pattern.
/// Simple prefix/wildcard matching: "file.*" matches "file.created", "file.*" matches "file.*", etc.
pub fn event_matches_any(
    subscriptions: &std::collections::HashSet<String>,
    event_type: &str,
) -> bool {
    for sub in subscriptions {
        if sub == event_type {
            return true;
        }
        // Glob-style: "file.*" matches "file.created"
        if let Some(prefix) = sub.strip_suffix(".*")
            && event_type.starts_with(prefix)
            && event_type[prefix.len()..].starts_with('.')
        {
            return true;
        }
        // Glob-style: "*" matches everything
        if sub == "*" {
            return true;
        }
    }
    false
}
