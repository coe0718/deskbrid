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
    tools_a11y, tools_audio, tools_bluetooth, tools_browser, tools_clipboard, tools_files,
    tools_input, tools_media, tools_misc, tools_monitors, tools_network, tools_notifications,
    tools_screenshot, tools_services, tools_system, tools_terminal, tools_windows,
};
use anyhow::Context;
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
    Json(rt.block_on(async { f.await.unwrap_or_else(|e| json!({"error": e.to_string()})) }))
}

fn execute(state: Arc<DaemonState>, rt: &Handle, action: &str, args: Value) -> Json<Value> {
    let action = action.to_string();
    let rt = rt.clone();
    Json(rt.block_on(async {
        do_execute_with(&state, &action, args)
            .await
            .unwrap_or_else(|e| json!({"error": e.to_string()}))
    }))
}

#[tool_router(server_handler)]
impl McpServer {
    tools_windows!();
    tools_screenshot!();
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
/// Self-contained: creates its own daemon state and backend.
pub async fn run_mcp_tcp_on_port(port: u16) -> anyhow::Result<()> {
    let event_tx = tokio::sync::broadcast::channel(256).0;
    let state = Arc::new(crate::DaemonState::new());
    let backend = crate::backend::create_backend(event_tx)
        .await
        .context("no desktop backend detected")?;
    *state.backend.write().await = Some(backend);
    run_mcp_tcp(state, port).await
}

/// Run the MCP server over TCP transport (for `deskbrid daemon --mcp-port`).
/// Uses rmcp's stream transport — same tool surface as stdio mode.
pub async fn run_mcp_tcp(state: Arc<DaemonState>, port: u16) -> anyhow::Result<()> {
    use rmcp::service::serve_server;
    use tokio::net::TcpListener;

    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("Deskbrid MCP (rmcp) TCP server listening on {addr}");

    loop {
        let (stream, peer) = listener.accept().await?;
        let state = state.clone();
        tokio::spawn(async move {
            let server = McpServer::new(state);
            if let Err(e) = serve_server(server, stream).await {
                tracing::error!("MCP connection error from {peer}: {e}");
            }
        });
    }
}
