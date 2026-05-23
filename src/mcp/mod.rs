//! MCP (Model Context Protocol) server mode for Deskbrid.
//!
//! Implements a minimal JSON-RPC 2.0 MCP server over stdin/stdout (for `deskbrid mcp`)
//! and TCP (for `deskbrid daemon --mcp-port <PORT>`).
//!
//! Zero external MCP dependencies — pure tokio + serde_json.

pub mod tools;

mod helpers;
mod tool_list;
mod types;

pub mod server;

use anyhow::Context;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::DaemonState;

/// Run the MCP stdio server (for `deskbrid mcp`).
pub async fn run_mcp_server() -> anyhow::Result<()> {
    let event_tx = tokio::sync::broadcast::channel(256).0;
    let state = Arc::new(DaemonState::new());
    let backend = crate::backend::create_backend(event_tx)
        .await
        .context("no desktop backend detected")?;
    *state.backend.write().await = Some(backend);

    tracing::info!("Deskbrid MCP server starting on stdio");
    serve_stdio(state).await
}

/// Run the MCP TCP server (for `deskbrid daemon --mcp-port <PORT>`).
pub async fn run_mcp_tcp(port: u16) -> anyhow::Result<()> {
    use tokio::net::TcpListener;
    let event_tx = tokio::sync::broadcast::channel(256).0;
    let state = Arc::new(DaemonState::new());
    let backend = crate::backend::create_backend(event_tx)
        .await
        .context("no desktop backend detected")?;
    *state.backend.write().await = Some(backend);

    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr)
        .await
        .with_context(|| format!("failed to bind MCP TCP on {addr}"))?;
    tracing::info!("Deskbrid MCP TCP server listening on {addr}");

    loop {
        let (stream, peer) = listener.accept().await?;
        let state = state.clone();
        tokio::spawn(async move {
            let (reader, mut writer) = tokio::io::split(stream);
            if let Err(e) = serve_on_stream(state, reader, &mut writer).await {
                tracing::error!("MCP connection error from {peer}: {e}");
            }
        });
    }
}

async fn serve_stdio(state: Arc<DaemonState>) -> anyhow::Result<()> {
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let reader = BufReader::new(stdin);
    serve_on_stream(state, reader, &mut stdout).await
}

async fn serve_on_stream(
    state: Arc<DaemonState>,
    reader: impl tokio::io::AsyncRead + Unpin,
    writer: &mut (impl tokio::io::AsyncWrite + Unpin),
) -> anyhow::Result<()> {
    let mut lines = BufReader::new(reader).lines();
    let mut initialized = false;

    while let Ok(Some(line)) = lines.next_line().await {
        let request: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let method = request["method"].as_str().unwrap_or("");
        let id = request.get("id").cloned();

        let response = match method {
            "initialize" => {
                initialized = true;
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "serverInfo": {
                            "name": "deskbrid",
                            "version": env!("CARGO_PKG_VERSION")
                        },
                        "capabilities": {
                            "tools": {}
                        }
                    }
                })
            }
            "notifications/initialized" => {
                continue; // no response
            }
            "tools/list" if initialized => {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "tools": tools::list_tools()
                    }
                })
            }
            "tools/call" if initialized => {
                let tool_name = request["params"]["name"].as_str().unwrap_or("");
                let args = request["params"]
                    .get("arguments")
                    .cloned()
                    .unwrap_or(json!({}));
                match tools::call_tool(&state, tool_name, &args).await {
                    Ok(content) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [{"type": "text", "text": content}]
                        }
                    }),
                    Err(e) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32000,
                            "message": e.to_string()
                        }
                    }),
                }
            }
            _ => {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32601,
                        "message": format!("Method not found: {method}")
                    }
                })
            }
        };

        let mut response_str = serde_json::to_string(&response)?;
        response_str.push('\n');
        writer.write_all(response_str.as_bytes()).await?;
        writer.flush().await?;
    }

    Ok(())
}
