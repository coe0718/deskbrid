use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use serde_json::Value;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

#[derive(Debug, Parser)]
#[command(name = "deskbrid", about = "deskbrid daemon and client CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Daemon,
    Subscribe {
        events: Vec<String>,
    },
    Action {
        action: String,
        params_json: Option<String>,
    },
    Info,
}

pub async fn run(command: Option<Command>, socket_path: PathBuf) -> Result<()> {
    match command.unwrap_or(Command::Daemon) {
        Command::Daemon => run_daemon(socket_path).await,
        Command::Subscribe { events } => run_subscribe(&socket_path, events).await,
        Command::Action {
            action,
            params_json,
        } => run_action(&socket_path, &action, params_json.as_deref()).await,
        Command::Info => run_info(&socket_path).await,
    }
}

async fn run_daemon(socket_path: PathBuf) -> Result<()> {
    let mut config = crate::config::Config::from_env();
    config.socket_path = Some(socket_path.clone());

    tracing_subscriber::fmt()
        .with_env_filter(&config.log_level)
        .init();

    tracing::info!("Starting deskbrid v{}", crate::VERSION);

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let mut daemon = tokio::spawn(crate::run(config, shutdown_rx));

    tokio::select! {
        result = &mut daemon => {
            match result {
                Ok(result) => result?,
                Err(error) => return Err(error.into()),
            }
        }
        signal = tokio::signal::ctrl_c() => {
            signal?;
            tracing::info!("shutting down");
            if shutdown_tx.send(true).is_err() {
                tracing::warn!("shutdown receiver dropped before signal delivery");
            }

            match daemon.await {
                Ok(result) => result?,
                Err(error) => return Err(error.into()),
            }
        }
    }

    match tokio::fs::remove_file(&socket_path).await {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => tracing::warn!("failed to remove socket {}: {error}", socket_path.display()),
    }

    Ok(())
}

async fn run_subscribe(socket_path: &Path, events: Vec<String>) -> Result<()> {
    let mut client = SocketClient::connect(socket_path).await?;
    client
        .send(&serde_json::to_value(
            crate::protocol::ClientMessage::Subscribe { events },
        )?)
        .await?;
    ensure_ok_result(client.expect_result().await?)?;

    while let Some(message) = client.read_json().await? {
        if message.get("type").and_then(Value::as_str) == Some("event") {
            println!("{}", serde_json::to_string(&message)?);
        }
    }

    Ok(())
}

async fn run_action(socket_path: &Path, action: &str, params_json: Option<&str>) -> Result<()> {
    let mut client = SocketClient::connect(socket_path).await?;
    let params = match params_json {
        Some(raw) => serde_json::from_str(raw).context("parsing params_json")?,
        None => serde_json::json!({}),
    };

    client
        .send(&serde_json::to_value(
            crate::protocol::ClientMessage::Action {
                id: uuid::Uuid::new_v4().to_string(),
                action: action.to_string(),
                params,
            },
        )?)
        .await?;

    let result = client.expect_result().await?;
    println!("{}", serde_json::to_string(&result)?);
    ensure_ok_result(result)?;
    Ok(())
}

async fn run_info(socket_path: &Path) -> Result<()> {
    let mut client = SocketClient::connect(socket_path).await?;
    client
        .send(&serde_json::to_value(
            crate::protocol::ClientMessage::Action {
                id: uuid::Uuid::new_v4().to_string(),
                action: "info".to_string(),
                params: serde_json::json!({}),
            },
        )?)
        .await?;

    let result = client.expect_result().await?;
    ensure_ok_result(result.clone())?;
    println!(
        "{}",
        serde_json::to_string(
            result
                .get("data")
                .ok_or_else(|| anyhow!("missing data in info response"))?
        )?
    );
    Ok(())
}

fn ensure_ok_result(result: Value) -> Result<()> {
    match result.get("ok").and_then(Value::as_bool) {
        Some(true) => Ok(()),
        Some(false) => Err(anyhow!(
            "{}: {}",
            result
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("unknown_error"),
            result
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("request failed")
        )),
        None => Err(anyhow!("missing result status")),
    }
}

struct SocketClient {
    reader: BufReader<tokio::net::unix::OwnedReadHalf>,
    writer: tokio::net::unix::OwnedWriteHalf,
}

impl SocketClient {
    async fn connect(socket_path: &Path) -> Result<Self> {
        let stream = UnixStream::connect(socket_path)
            .await
            .with_context(|| format!("connecting to {}", socket_path.display()))?;
        let (reader, writer) = stream.into_split();
        let mut client = Self {
            reader: BufReader::new(reader),
            writer,
        };

        let hello = client
            .read_json()
            .await?
            .ok_or_else(|| anyhow!("daemon closed connection before hello"))?;
        if hello.get("type").and_then(Value::as_str) != Some("hello") {
            return Err(anyhow!("expected hello message from daemon"));
        }

        Ok(client)
    }

    async fn send(&mut self, message: &Value) -> Result<()> {
        let encoded = serde_json::to_string(message).context("serializing client message")?;
        self.writer
            .write_all(encoded.as_bytes())
            .await
            .context("writing client message")?;
        self.writer
            .write_all(b"\n")
            .await
            .context("writing client message delimiter")?;
        Ok(())
    }

    async fn expect_result(&mut self) -> Result<Value> {
        loop {
            match self.read_json().await? {
                Some(message) if message.get("type").and_then(Value::as_str) == Some("result") => {
                    return Ok(message);
                }
                Some(_) => continue,
                None => return Err(anyhow!("daemon closed connection before result")),
            }
        }
    }

    async fn read_json(&mut self) -> Result<Option<Value>> {
        let mut line = String::new();
        let bytes = self
            .reader
            .read_line(&mut line)
            .await
            .context("reading daemon message")?;
        if bytes == 0 {
            return Ok(None);
        }

        serde_json::from_str(line.trim_end())
            .map(Some)
            .context("parsing daemon message")
    }
}
