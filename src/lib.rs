//! deskbrid — The HAL your Linux desktop agents are missing.
//!
//! A standalone daemon that wraps Wayland protocols, DBus APIs, and PipeWire
//! into a JSON-over-Unix-socket protocol for agent-native desktop control.

pub mod capture;
pub mod cli;
pub mod clipboard;
pub mod config;
pub mod dbus;
pub mod events;
pub mod input;
pub mod protocol;

use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::watch;
use tracing::{debug, info, warn};

/// Default socket path: $XDG_RUNTIME_DIR/deskbrid/socket
pub fn default_socket_path() -> PathBuf {
    let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/run/user/1000".to_string());
    PathBuf::from(runtime).join("deskbrid").join("socket")
}

/// Version of the running daemon
pub const VERSION: &str = "0.1.0";

#[derive(Clone, Default)]
struct RuntimeState {
    input_session: Option<input::InputSession>,
    clipboard_monitor: Option<clipboard::Monitor>,
    dbus_hub: Option<dbus::Hub>,
}

impl RuntimeState {
    fn capabilities(&self) -> Vec<&'static str> {
        let mut capabilities = vec!["screenshot", "screencast"];
        if self.dbus_hub.is_some() {
            capabilities.extend(["window", "notifications", "display", "idle"]);
        }
        if self.input_session.is_some() {
            capabilities.push("inject");
        }
        if self.clipboard_monitor.is_some() {
            capabilities.push("clipboard");
        }
        capabilities.push("audio");
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
    info!("listening on {}", socket_path.display());

    let event_bus = events::EventBus::new();
    let input_session = match input::InputSession::new().await {
        Ok(session) => Some(session),
        Err(error) => {
            warn!("input injection unavailable: {error:#}");
            None
        }
    };
    let clipboard_monitor = match clipboard::Monitor::new(event_bus.clone(), shutdown.clone()).await
    {
        Ok(monitor) => Some(monitor),
        Err(error) => {
            warn!("clipboard unavailable: {error:#}");
            None
        }
    };
    let dbus_hub = match dbus::Hub::new(event_bus.clone()).await {
        Ok(hub) => Some(hub),
        Err(error) => {
            warn!("dbus hub unavailable: {error:#}");
            None
        }
    };

    let state = RuntimeState {
        input_session,
        clipboard_monitor,
        dbus_hub,
    };

    if let Some(dbus_hub) = state.dbus_hub.clone() {
        tokio::spawn(
            dbus_hub
                .clone()
                .watch_windows(event_bus.clone(), shutdown.clone()),
        );
        tokio::spawn(
            dbus_hub
                .clone()
                .watch_notifications(event_bus.clone(), shutdown.clone()),
        );
        tokio::spawn(dbus_hub.watch_idle(event_bus.clone(), shutdown.clone()));
    }

    let mut shutdown = shutdown;
    loop {
        tokio::select! {
            _ = shutdown.changed() => break,
            accept_result = listener.accept() => {
                let (stream, addr) = accept_result.context("accepting client")?;
                let eb = event_bus.clone();
                let client_state = state.clone();
                let client_shutdown = shutdown.clone();

                tokio::spawn(async move {
                    debug!("client connected: {:?}", addr);
                    if let Err(error) = handle_client(stream, eb, client_state, client_shutdown).await {
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
    event_bus: events::EventBus,
    state: RuntimeState,
    mut shutdown: watch::Receiver<bool>,
) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    let mut subscriptions = protocol::Session::new();
    let mut events_rx = event_bus.subscribe();

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
    state: &RuntimeState,
) -> Result<serde_json::Value> {
    match action {
        "inject:type" => {
            let text = params
                .get("text")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| anyhow!("missing 'text' param"))?;
            require_subsystem(&state.input_session, "inject")?
                .type_text(text)
                .await?;
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
            require_subsystem(&state.input_session, "inject")?
                .send_keys(&keys)
                .await?;
            Ok(serde_json::json!({}))
        }
        "inject:mouse" => {
            require_subsystem(&state.input_session, "inject")?
                .mouse_action(&params)
                .await?;
            Ok(serde_json::json!({}))
        }
        "window:list" => {
            let windows = require_subsystem(&state.dbus_hub, "window")?
                .list_windows()
                .await?;
            Ok(serde_json::json!({ "windows": windows }))
        }
        "display:list" => {
            let monitors = require_subsystem(&state.dbus_hub, "display")?
                .list_monitors()
                .await?;
            Ok(serde_json::json!({ "monitors": monitors }))
        }
        "window:focus" => {
            let app_id = params.get("app_id").and_then(serde_json::Value::as_str);
            let title = params.get("title").and_then(serde_json::Value::as_str);
            let exact = params
                .get("exact")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            require_subsystem(&state.dbus_hub, "window")?
                .focus_window(app_id, title, exact)
                .await?;
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
            let id = require_subsystem(&state.dbus_hub, "notifications")?
                .send_notification(summary, body, urgency)
                .await?;
            Ok(serde_json::json!({ "id": id }))
        }
        "screenshot" => {
            let monitor = params
                .get("monitor")
                .and_then(serde_json::Value::as_u64)
                .map(|value| value as u32);
            let path = capture::screenshot(monitor).await?;
            Ok(serde_json::json!({ "path": path }))
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
            let node_id = capture::start_screencast(monitor, framerate).await?;
            Ok(serde_json::json!({ "node_id": node_id }))
        }
        "screencast:stop" => {
            let node_id = params
                .get("node_id")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| anyhow!("missing 'node_id' param"))?
                as u32;
            capture::stop_screencast(node_id).await?;
            Ok(serde_json::json!({}))
        }
        "info" => Ok(serde_json::json!({
            "deskbrid_version": VERSION,
            "desktop": "GNOME",
            "session_type": "wayland",
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
