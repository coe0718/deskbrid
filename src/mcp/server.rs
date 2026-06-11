//! MCP server — rmcp-based stdio server for `deskbrid mcp`.
//!
//! Each tool is a thin `#[tool]` wrapper delegating to async helpers.
//! Tool implementations are split across 17 `tools_*.rs` files,
//! each defining a `#[macro_export]` macro called from the impl block below.
//!
//! Note: `block` and `execute` helpers are used by tool macros in tools_*.rs
//! but the compiler can't see the cross-file usage, hence the allow.

#![allow(dead_code)]

use super::helpers::*;
use super::types::*;
use crate::DaemonState;
use crate::{
    tools_a11y, tools_agent, tools_audio, tools_bluetooth, tools_browser, tools_clipboard,
    tools_confirmation, tools_desktop, tools_files, tools_input, tools_media, tools_misc,
    tools_monitors, tools_network, tools_notifications, tools_portal, tools_screencast,
    tools_screenshot, tools_search, tools_secrets, tools_services, tools_system, tools_terminal,
    tools_windows,
};
// Types used by macro expansions (defined in tool modules, used in #[tool] signatures)
use crate::mcp::tools_agent::{BroadcastArgs, SendMessageArgs};
use crate::mcp::tools_confirmation::ConfirmActionArgs;
use crate::mcp::tools_search::SearchArgs;
use crate::mcp::tools_secrets::{SecretsGetArgs, SecretsStoreArgs};
use rmcp::{
    handler::server::wrapper::{Json, Parameters},
    tool, tool_router,
};
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::runtime::Handle;

#[derive(Clone)]
pub struct McpServer {
    state: Arc<DaemonState>,
    rt: Handle,
}

impl McpServer {
    pub fn new(state: Arc<DaemonState>) -> Self {
        Self {
            state,
            rt: Handle::current(),
        }
    }
}

fn block<F: std::future::Future<Output = anyhow::Result<Value>>>(rt: &Handle, f: F) -> Json<Value> {
    Json(
        tokio::task::block_in_place(|| rt.block_on(f))
            .unwrap_or_else(|e| json!({"error": e.to_string()})),
    )
}

/// Like `block` but doesn't require a desktop backend — for state-only actions
/// (confirmation, agent messaging, etc.) that only need DaemonState.
fn block_state<F>(
    rt: &Handle,
    state: &Arc<DaemonState>,
    f: impl FnOnce(Arc<DaemonState>) -> F,
) -> Json<Value>
where
    F: std::future::Future<Output = anyhow::Result<Value>> + Send + 'static,
{
    let state = state.clone();
    let rt = rt.clone();
    Json(tokio::task::block_in_place(move || {
        rt.block_on(async {
            f(state)
                .await
                .unwrap_or_else(|e| json!({"error": e.to_string()}))
        })
    }))
}

fn execute(state: Arc<DaemonState>, rt: &Handle, action: &str, args: Value) -> Json<Value> {
    let action = action.to_string();
    let rt = rt.clone();
    Json(tokio::task::block_in_place(move || {
        rt.block_on(async {
            match do_execute_with(&state, &action, args).await {
                Ok(v) => v,
                Err(e) if e.to_string().contains("no backend") => {
                    json!({"headless": true, "note": "Running in Docker/headless mode — no desktop backend available"})
                }
                Err(e) => json!({"error": e.to_string()}),
            }
        })
    }))
}

#[tool_router(server_handler)]
impl McpServer {
    tools_windows!();
    tools_screenshot!();
    tools_screencast!();
    tools_input!();
    tools_clipboard!();
    tools_a11y!();
    tools_system!();
    tools_network!();
    tools_bluetooth!();
    tools_services!();
    tools_audio!();
    tools_files!();
    tools_terminal!();
    tools_browser!();
    tools_media!();
    tools_monitors!();
    tools_notifications!();
    tools_misc!();
    tools_portal!();
    tools_desktop!();
    tools_confirmation!();
    tools_agent!();
    tools_search!();
    tools_secrets!();
}

/// Run the MCP server over stdio transport (for `deskbrid mcp`).
pub async fn run_mcp(state: Arc<DaemonState>) -> anyhow::Result<()> {
    use rmcp::{service::serve_server, transport::stdio};
    serve_server(McpServer::new(state), stdio())
        .await?
        .waiting()
        .await?;
    Ok(())
}

/// Run the MCP server over TCP transport (for `deskbrid daemon --mcp-port`).
/// Requires bearer-token auth before the MCP handshake — same pattern as the TCP
/// control listener. Without a valid token the connection is rejected.
pub async fn run_mcp_tcp(state: Arc<DaemonState>, port: u16, token: String) -> anyhow::Result<()> {
    use crate::daemon::tcp::constant_time_eq;
    use rmcp::service::serve_server;
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
    use tokio::net::TcpListener;

    const MAX_AUTH_LINE: u64 = 4096;

    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("Deskbrid MCP (rmcp) TCP server listening on {addr} (token auth)");

    loop {
        let (stream, peer) = listener.accept().await?;
        let state = state.clone();
        let token = token.clone();
        tokio::spawn(async move {
            // ── Bearer token auth ──
            let (reader, writer) = tokio::io::split(stream);
            let mut limited = reader.take(MAX_AUTH_LINE + 1);
            let mut buf_reader = BufReader::new(&mut limited);
            let mut auth_line = String::new();
            if let Err(e) = buf_reader.read_line(&mut auth_line).await {
                tracing::error!("MCP auth read error from {peer}: {e}");
                return;
            }
            drop(buf_reader);
            let reader = limited.into_inner();

            if auth_line.len() > MAX_AUTH_LINE as usize {
                tracing::warn!(
                    "MCP auth message too large ({} bytes) from {peer} — rejecting",
                    auth_line.len()
                );
                return;
            }

            if auth_line.trim().is_empty() {
                tracing::warn!("MCP client {peer} sent empty auth — rejecting");
                return;
            }

            let auth: serde_json::Value = match serde_json::from_str(&auth_line) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("MCP client {peer} sent invalid JSON auth: {e}");
                    return;
                }
            };

            if auth.get("type") != Some(&serde_json::Value::String("auth".into())) {
                tracing::warn!("MCP client {peer} sent non-auth first message");
                return;
            }

            let provided = auth.get("token").and_then(|v| v.as_str()).unwrap_or("");
            if !constant_time_eq(provided, &token) {
                tracing::warn!("MCP client {peer} sent invalid token");
                return;
            }

            // ── Auth passed — reassemble and serve ──
            let stream = reader.unsplit(writer);
            let server = McpServer::new(state);
            if let Err(e) = serve_server(server, stream).await {
                tracing::error!("MCP connection error from {peer}: {e}");
            }
        });
    }
}
