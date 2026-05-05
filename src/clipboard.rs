//! Clipboard monitoring and access using wl-clipboard tools.

use crate::events::EventBus;
use anyhow::{anyhow, Context, Result};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::watch;
use tokio::sync::RwLock;
use tracing::warn;

#[derive(Clone)]
pub struct Monitor {
    last_text: Arc<RwLock<Option<String>>>,
}

impl Monitor {
    pub async fn new(event_bus: EventBus, shutdown: watch::Receiver<bool>) -> Result<Self> {
        ensure_tool("wl-paste", "--version").await?;
        ensure_tool("wl-copy", "--version").await?;

        let monitor = Self {
            last_text: Arc::new(RwLock::new(None)),
        };
        let task_monitor = monitor.clone();
        tokio::spawn(async move {
            task_monitor.watch(event_bus, shutdown).await;
        });
        Ok(monitor)
    }

    pub async fn read(&self) -> Result<String> {
        let output = Command::new("wl-paste")
            .arg("--no-newline")
            .output()
            .await
            .context("running wl-paste")?;
        if !output.status.success() {
            return Err(anyhow!(
                "wl-paste failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        String::from_utf8(output.stdout).context("clipboard text was not valid utf-8")
    }

    pub async fn write(&self, text: &str) -> Result<()> {
        let mut child = Command::new("wl-copy")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("spawning wl-copy")?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("wl-copy stdin unavailable"))?;
        stdin
            .write_all(text.as_bytes())
            .await
            .context("writing clipboard text to wl-copy")?;
        drop(stdin);

        let output = child
            .wait_with_output()
            .await
            .context("waiting for wl-copy")?;
        if !output.status.success() {
            return Err(anyhow!(
                "wl-copy failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        *self.last_text.write().await = Some(text.to_string());
        Ok(())
    }

    async fn watch(&self, event_bus: EventBus, mut shutdown: watch::Receiver<bool>) {
        loop {
            let mut child = match Command::new("wl-paste")
                .arg("--watch")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .kill_on_drop(true)
                .spawn()
            {
                Ok(child) => child,
                Err(error) => {
                    warn!("clipboard watch spawn failed: {error:#}");
                    break;
                }
            };

            let stdout = match child.stdout.take() {
                Some(stdout) => stdout,
                None => {
                    warn!("clipboard watch stdout unavailable");
                    break;
                }
            };
            let mut lines = BufReader::new(stdout).lines();

            loop {
                tokio::select! {
                    _ = shutdown.changed() => return,
                    change = lines.next_line() => match change {
                        Ok(Some(_)) => {
                            match self.read().await {
                                Ok(text) => {
                                    let mut guard = self.last_text.write().await;
                                    if guard.as_ref() != Some(&text) {
                                        *guard = Some(text.clone());
                                        event_bus.emit(
                                            "clipboard",
                                            serde_json::json!({
                                                "text": text,
                                                "mime_types": ["text/plain"],
                                                "timestamp": now_ts(),
                                            }),
                                        );
                                    }
                                }
                                Err(error) => warn!("clipboard read failed after watch event: {error:#}"),
                            }
                        }
                        Ok(None) => {
                            warn!("clipboard watch exited");
                            break;
                        }
                        Err(error) => {
                            warn!("clipboard watch read failed: {error:#}");
                            break;
                        }
                    }
                }
            }
        }
    }
}

async fn ensure_tool(tool: &str, arg: &str) -> Result<()> {
    let output = Command::new(tool)
        .arg(arg)
        .output()
        .await
        .with_context(|| format!("running {tool} {arg}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow!(
            "{tool} unavailable: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
