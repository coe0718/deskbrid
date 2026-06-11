use crate::permissions::socket_peer_uid;
use crate::protocol::Action;
use crate::{ConnectionState, DaemonState};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tracing::{info, warn};

use super::dispatch::dispatch_action_with_options;
use super::helpers::ok_response;

/// Handle a Unix socket client (SO_PEERCRED auth).
pub async fn handle_client(stream: UnixStream, state: &DaemonState) -> anyhow::Result<()> {
    let peer_uid = socket_peer_uid(&stream)
        .ok_or_else(|| anyhow::anyhow!("failed to determine peer UID — connection rejected"))?;
    let (reader, writer) = stream.into_split();
    handle_client_generic(BufReader::new(reader), writer, peer_uid, state).await
}

/// Handle a TCP client (pre-authenticated, caller provides effective UID).
pub async fn handle_client_tcp<S: AsyncRead + AsyncWrite + Unpin>(
    stream: S,
    effective_uid: u32,
    state: &DaemonState,
) -> anyhow::Result<()> {
    let (reader, writer) = tokio::io::split(stream);
    handle_client_generic(BufReader::new(reader), writer, effective_uid, state).await
}

/// Transport-agnostic client handler. Works over any AsyncRead + AsyncWrite.
async fn handle_client_generic<R, W>(
    mut reader: BufReader<R>,
    mut writer: W,
    peer_uid: u32,
    state: &DaemonState,
) -> anyhow::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut conn = ConnectionState::default();
    let mut line = String::new();
    let mut seq: u64 = 0;

    let connected = serde_json::json!({
        "type": "connected",
        "id": "server",
        "seq": 0,
        "data": { "version": env!("CARGO_PKG_VERSION"), "protocol": "deskbrid-v2", "uid": peer_uid, "session": conn.session_id }
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
                                let _ = writer.write_all(format!("{out}\n").as_bytes()).await;
                            }
                        }
                    }
                }
            }

            // Read next client command (capped at 10MB to prevent memory exhaustion)
            result = read_line_limited(&mut reader, &mut line) => {
                let n = result?;
                if n == 0 {
                    break;
                }

                seq += 1;
                if line.trim().is_empty() {
                    line.clear();
                    continue;
                }

                // Handle "connect" message — join a named session
                if let Ok(raw) = serde_json::from_str::<serde_json::Value>(&line)
                    && raw["type"].as_str() == Some("connect")
                {
                    if let Some(session_name) = raw["session"].as_str() {
                        let name = session_name.to_string();
                        // Create session if it doesn't exist
                        state.sessions.entry(name.clone()).or_insert_with(|| {
                            crate::SessionData::new(name.clone())
                        });
                        conn.session_id = name;
                            let resp = serde_json::json!({
                                "type": "response",
                                "id": raw["id"].as_str().unwrap_or("?"),
                                "seq": seq,
                                "status": "ok",
                                "data": { "session": conn.session_id }
                            });
                            writer
                                .write_all(format!("{}\n", serde_json::to_string(&resp)?).as_bytes())
                                .await?;
                        } else {
                            let err = serde_json::json!({
                                "type": "response", "id": "?", "seq": seq, "status": "error",
                                "error": { "code": "INVALID_PARAMS", "message": "connect requires 'session' field" }
                            });
                            writer
                                .write_all(format!("{}\n", serde_json::to_string(&err)?).as_bytes())
                                .await?;
                        }
                        line.clear();
                        continue;
                }

                let (request_id, action, options) = match Action::from_json_with_options(&line) {
                    Ok((id, action, options)) => (id, action, options),
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
                        let resp = ok_response(&request_id, seq);
                        writer
                            .write_all(format!("{}\n", serde_json::to_string(&resp)?).as_bytes())
                            .await?;
                    }

                    Action::Unsubscribe { events } => {
                        for evt in &events {
                            conn.subscriptions.remove(evt);
                        }
                        let resp = ok_response(&request_id, seq);
                        writer
                            .write_all(format!("{}\n", serde_json::to_string(&resp)?).as_bytes())
                            .await?;
                    }

                    // Files — track watched paths locally
                    // Session switch updates the connection's active session so var
                    // ops resolve against the correct session after switching.
                    Action::SessionSwitch { ref name } => {
                        conn.session_id = name.clone();
                        let resp = dispatch_action_with_options(&request_id, action, state, peer_uid, seq, options, &conn.session_id).await;
                        writer
                            .write_all(format!("{}\n", serde_json::to_string(&resp)?).as_bytes())
                            .await?;
                    }

                    Action::FilesWatch { ref path, .. } => {
                        conn.watched_paths.insert(path.clone());
                        let resp = dispatch_action_with_options(&request_id, action, state, peer_uid, seq, options, &conn.session_id).await;
                        writer
                            .write_all(format!("{}\n", serde_json::to_string(&resp)?).as_bytes())
                            .await?;
                    }
                    Action::FilesUnwatch { ref path } => {
                        conn.watched_paths.remove(path);
                        let resp = dispatch_action_with_options(&request_id, action, state, peer_uid, seq, options, &conn.session_id).await;
                        writer
                            .write_all(format!("{}\n", serde_json::to_string(&resp)?).as_bytes())
                            .await?;
                    }

                    _ => {
                        let resp = dispatch_action_with_options(&request_id, action, state, peer_uid, seq, options, &conn.session_id).await;
                        writer
                            .write_all(format!("{}\n", serde_json::to_string(&resp)?).as_bytes())
                            .await?;
                    }
                }
            }
        }
    }

    info!("Client disconnected (uid={})", peer_uid);

    // Clean up rate limit buckets and other per-peer state to prevent
    // unbounded memory growth from accumulated disconnected clients.
    state.rate_limit_store.remove_peer(peer_uid);

    Ok(())
}

/// Read a line from a buffered reader with a 10MB cap to prevent memory exhaustion.
/// If the line exceeds MAX_BYTES without a newline, returns an error rather than
/// silently delivering partial chunks.
async fn read_line_limited<R: AsyncRead + Unpin>(
    reader: &mut BufReader<R>,
    buf: &mut String,
) -> std::io::Result<usize> {
    const MAX_BYTES: u64 = 10 * 1024 * 1024;
    let mut limited = reader.take(MAX_BYTES);
    let n = limited.read_line(buf).await?;
    // If we read exactly MAX_BYTES and don't end with a newline, the line was truncated
    if n as u64 == MAX_BYTES && !buf.ends_with('\n') {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("line exceeds {MAX_BYTES} byte limit"),
        ));
    }
    Ok(n)
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
