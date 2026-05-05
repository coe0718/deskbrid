//! deskbrid — The HAL your Linux desktop agents are missing.
//!
//! A standalone daemon that wraps Wayland protocols, DBus APIs, and PipeWire
//! into a JSON-over-Unix-socket protocol for agent-native desktop control.

pub mod backend;
pub mod capture;
pub mod cli;
pub mod clipboard;
pub mod config;
pub mod dbus;
pub mod events;
pub mod input;
pub mod protocol;

use anyhow::{anyhow, Context, Result};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::watch;
use tracing::{debug, info, warn};

pub use backend::types::{MonitorInfo, WindowInfo};

type ClipboardMonitor = clipboard::Monitor;

/// Default socket path: $XDG_RUNTIME_DIR/deskbrid/socket
pub fn default_socket_path() -> PathBuf {
    let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/run/user/1000".to_string());
    PathBuf::from(runtime).join("deskbrid").join("socket")
}

/// Version of the running daemon
pub const VERSION: &str = "0.1.0";

struct DaemonState {
    backend: Box<dyn backend::DesktopBackend>,
    clipboard_monitor: Option<ClipboardMonitor>,
    event_bus: events::EventBus,
}

impl DaemonState {
    fn capabilities(&self) -> Vec<&'static str> {
        let mut capabilities = self.backend.capabilities().to_vec();
        if self.clipboard_monitor.is_some() {
            capabilities.push("clipboard");
        }
        capabilities
    }
}

/// Main daemon entry point.
pub async fn run(config: config::Config, shutdown: watch::Receiver<bool>) -> Result<()> {
    let socket_path = config.socket_path.unwrap_or_else(default_socket_path);

    if let Some(parent) = socket_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("creating socket dir {}", parent.display()))?;
    }

    let _ = tokio::fs::remove_file(&socket_path).await;

    let listener = UnixListener::bind(&socket_path)
        .with_context(|| format!("binding socket {}", socket_path.display()))?;
    fs::set_permissions(&socket_path, fs::Permissions::from_mode(0o600))
        .with_context(|| format!("setting socket permissions on {}", socket_path.display()))?;
    info!("listening on {}", socket_path.display());

    let event_bus = events::EventBus::new();
    let clipboard_monitor = match clipboard::Monitor::new(event_bus.clone(), shutdown.clone()).await
    {
        Ok(monitor) => Some(monitor),
        Err(error) => {
            warn!("clipboard unavailable: {error:#}");
            None
        }
    };
    let backend = backend::create_backend(event_bus.clone(), shutdown.clone()).await?;

    let state = Arc::new(DaemonState {
        backend,
        clipboard_monitor,
        event_bus: event_bus.clone(),
    });

    let mut shutdown = shutdown;
    loop {
        tokio::select! {
            _ = shutdown.changed() => break,
            accept_result = listener.accept() => {
                let (stream, addr) = accept_result.context("accepting client")?;
                let client_state = Arc::clone(&state);
                let client_shutdown = shutdown.clone();

                tokio::spawn(async move {
                    debug!("client connected: {:?}", addr);
                    if let Err(error) = handle_client(stream, client_state, client_shutdown).await {
                        warn!("client error: {error:#}");
                    }
                });
            }
        }
    }

    Ok(())
}

async fn handle_client(
    stream: UnixStream,
    state: Arc<DaemonState>,
    mut shutdown: watch::Receiver<bool>,
) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    let mut subscriptions = protocol::Session::new();
    let mut events_rx = state.event_bus.subscribe();

    write_server_message(
        &mut writer,
        &protocol::ServerMessage::Hello {
            version: VERSION,
            pid: std::process::id(),
        },
    )
    .await?;

    loop {
        tokio::select! {
            read_result = reader.read_line(&mut line) => {
                let bytes = read_result.context("reading client message")?;
                if bytes == 0 {
                    break;
                }

                if bytes > 1024 * 1024 {
                    return Err(anyhow!("client message exceeds 1 MiB"));
                }

                let parsed = serde_json::from_str::<protocol::ClientMessage>(&line).map_err(|error| {
                    anyhow!("invalid client message: {error}")
                });
                line.clear();

                match parsed {
                    Ok(protocol::ClientMessage::Subscribe { events }) => {
                        for event in events {
                            subscriptions.subscribe(&event);
                        }
                        let response = protocol::ServerMessage::Result {
                            id: uuid::Uuid::new_v4().to_string(),
                            ok: true,
                            data: Some(serde_json::json!({})),
                            error: None,
                            message: None,
                        };
                        write_server_message(&mut writer, &response).await?;
                    }
                    Ok(protocol::ClientMessage::Unsubscribe { events }) => {
                        for event in events {
                            subscriptions.unsubscribe(&event);
                        }
                        let response = protocol::ServerMessage::Result {
                            id: uuid::Uuid::new_v4().to_string(),
                            ok: true,
                            data: Some(serde_json::json!({})),
                            error: None,
                            message: None,
                        };
                        write_server_message(&mut writer, &response).await?;
                    }
                    Ok(protocol::ClientMessage::Action { id, action, params }) => {
                        let response = match dispatch_action(&action, params, &state).await {
                            Ok(data) => protocol::ServerMessage::Result {
                                id,
                                ok: true,
                                data: Some(data),
                                error: None,
                                message: None,
                            },
                            Err(error) => {
                                let (code, message) = action_error(&error);
                                protocol::ServerMessage::Result {
                                    id,
                                    ok: false,
                                    data: None,
                                    error: Some(code.to_string()),
                                    message: Some(message),
                                }
                            }
                        };
                        write_server_message(&mut writer, &response).await?;
                    }
                    Err(error) => {
                        let response = protocol::ServerMessage::Result {
                            id: uuid::Uuid::new_v4().to_string(),
                            ok: false,
                            data: None,
                            error: Some("invalid_params".to_string()),
                            message: Some(error.to_string()),
                        };
                        write_server_message(&mut writer, &response).await?;
                    }
                }
            }
            event = events_rx.recv() => {
                match event {
                    Ok(event) if subscriptions.is_subscribed(&event.event_type) => {
                        let message = protocol::ServerMessage::Event {
                            event: event.event_type,
                            data: event.data,
                        };
                        write_server_message(&mut writer, &message).await?;
                    }
                    Ok(_) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!("client event stream lagged by {skipped} messages");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            _ = shutdown.changed() => break,
        }
    }

    Ok(())
}

async fn write_server_message(
    writer: &mut tokio::net::unix::OwnedWriteHalf,
    message: &protocol::ServerMessage,
) -> Result<()> {
    let encoded = serde_json::to_string(message).context("serializing server message")?;
    writer
        .write_all(encoded.as_bytes())
        .await
        .context("writing server message")?;
    writer.write_all(b"\n").await.context("writing newline")?;
    Ok(())
}

async fn dispatch_action(
    action: &str,
    params: serde_json::Value,
    state: &DaemonState,
) -> Result<serde_json::Value> {
    match action {
        "inject:type" => {
            let text = params
                .get("text")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| anyhow!("missing 'text' param"))?;
            let input = state.backend.create_input_session().await?;
            input.type_text(text).await?;
            Ok(serde_json::json!({}))
        }
        "inject:key" => {
            let keys: Vec<String> = serde_json::from_value(
                params
                    .get("keys")
                    .cloned()
                    .ok_or_else(|| anyhow!("missing 'keys' param"))?,
            )
            .context("invalid 'keys' param")?;
            let input = state.backend.create_input_session().await?;
            input.send_keys(&keys).await?;
            Ok(serde_json::json!({}))
        }
        "inject:mouse" => {
            let input = state.backend.create_input_session().await?;
            input.mouse_action(&params).await?;
            Ok(serde_json::json!({}))
        }
        "window:list" => {
            let windows = state.backend.list_windows().await?;
            Ok(serde_json::json!({ "windows": windows }))
        }
        "window:focused" => {
            let window = state.backend.focused_window().await?;
            Ok(serde_json::json!({ "window": window }))
        }
        "display:list" => {
            let monitors = state.backend.list_displays().await?;
            Ok(serde_json::json!({ "monitors": monitors }))
        }
        "window:focus" => {
            let app_id = params.get("app_id").and_then(serde_json::Value::as_str);
            let title = params.get("title").and_then(serde_json::Value::as_str);
            let exact = params
                .get("exact")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            state.backend.focus_window(app_id, title, exact).await?;
            Ok(serde_json::json!({}))
        }
        "clipboard:read" => {
            let text = require_subsystem(&state.clipboard_monitor, "clipboard")?
                .read()
                .await?;
            Ok(serde_json::json!({
                "text": text,
                "mime_types": ["text/plain"],
            }))
        }
        "clipboard:write" => {
            let text = params
                .get("text")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| anyhow!("missing 'text' param"))?;
            require_subsystem(&state.clipboard_monitor, "clipboard")?
                .write(text)
                .await?;
            Ok(serde_json::json!({}))
        }
        "notification:send" => {
            let summary = params
                .get("summary")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| anyhow!("missing 'summary' param"))?;
            let body = params
                .get("body")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let urgency = params
                .get("urgency")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("normal");
            let id = state
                .backend
                .send_notification(summary, body, urgency)
                .await?;
            Ok(serde_json::json!({ "id": id }))
        }
        "screenshot" => {
            let monitor = params
                .get("monitor")
                .and_then(serde_json::Value::as_u64)
                .map(|value| value as u32);
            state.backend.screenshot(monitor).await
        }
        "screencast:start" => {
            let monitor = params
                .get("monitor")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0) as u32;
            let framerate = params
                .get("framerate")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(15) as u32;
            state.backend.start_screencast(monitor, framerate).await
        }
        "screencast:stop" => {
            let node_id = params
                .get("node_id")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| anyhow!("missing 'node_id' param"))?
                as u32;
            state.backend.stop_screencast(node_id).await?;
            Ok(serde_json::json!({}))
        }
        "info" => Ok(serde_json::json!({
            "deskbrid_version": VERSION,
            "desktop": state.backend.desktop_name(),
            "session_type": std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".to_string()),
            "capabilities": state.capabilities()
        })),
        other => Err(anyhow!("unknown action: {other}")),
    }
}

fn action_error(error: &anyhow::Error) -> (&'static str, String) {
    let message = error.to_string();
    if let Some(capability) = message.strip_prefix("not_supported: ") {
        ("not_supported", capability.to_string())
    } else if message.starts_with("missing ")
        || message.starts_with("invalid ")
        || message.starts_with("unknown ")
        || message.starts_with("unsupported ")
    {
        ("invalid_params", message)
    } else {
        ("internal_error", message)
    }
}

fn require_subsystem<'a, T>(subsystem: &'a Option<T>, capability: &str) -> Result<&'a T> {
    subsystem
        .as_ref()
        .ok_or_else(|| anyhow!("not_supported: {capability} capability unavailable"))
}
