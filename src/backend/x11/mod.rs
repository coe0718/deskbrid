use crate::protocol;
use crate::protocol::DeskbridEvent;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::process::Command;
use tokio::sync::broadcast;

mod audio;
mod bluetooth;
mod files;
pub(crate) mod helpers;
mod keyboard_layout;
mod monitor;
mod networking;
mod notifications;
mod screenshot;
mod system_info;
#[cfg(test)]
mod tests;
mod trait_impl;
mod windows;

use helpers::*;

pub struct X11Backend {
    #[allow(dead_code)]
    event_tx: broadcast::Sender<DeskbridEvent>,
    #[allow(dead_code)]
    watchers: Arc<Mutex<HashMap<String, notify::RecommendedWatcher>>>,
    pub(super) detected_de: String,
}

impl X11Backend {
    pub async fn new(
        event_tx: broadcast::Sender<DeskbridEvent>,
        detected_de: String,
    ) -> anyhow::Result<Self> {
        tracing::info!("Detected {} via X11 backend", detected_de);

        // Auto-detect DISPLAY and XAUTHORITY if not set in environment.
        // The daemon may be started from SSH or a context that doesn't inherit
        // the desktop session's X11 auth — find them so child tools work.
        if std::env::var("DISPLAY").is_err() {
            // SAFETY: Setting DISPLAY for child process environment.
            // :0 is the standard first X11 display and safe to assume.
            unsafe { std::env::set_var("DISPLAY", ":0"); }
        }
        if std::env::var("XAUTHORITY").is_err() {
            let xauthority_path = std::env::var("HOME")
                .map(|h| std::path::PathBuf::from(h).join(".Xauthority"))
                .unwrap_or_default();
            let candidates = [xauthority_path];
            let mut found = None;
            for c in &candidates {
                if c.exists() {
                    found = Some(c.clone());
                    break;
                }
            }
            // Fallback: scan /tmp/xauth_* for files owned by current user
            if found.is_none() {
                if let Ok(entries) = std::fs::read_dir("/tmp") {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        if name.starts_with("xauth_") {
                            found = Some(path);
                            break;
                        }
                    }
                }
            }
            if let Some(auth_path) = found {
                tracing::info!(
                    "Auto-detected XAUTHORITY={}",
                    auth_path.display()
                );
                // SAFETY: Setting XAUTHORITY for child X11 tools (xdotool, wmctrl, etc.).
                unsafe { std::env::set_var("XAUTHORITY", auth_path); }
            } else {
                tracing::warn!(
                    "No XAUTHORITY found — X11 tools (xdotool, wmctrl) will fail. \
                     Set XAUTHORITY environment variable."
                );
            }
        }

        Ok(Self {
            event_tx,
            watchers: Arc::new(Mutex::new(HashMap::new())),
            detected_de,
        })
    }
    pub(super) async fn sh(&self, cmd: &str, args: &[&str]) -> anyhow::Result<String> {
        let out = Command::new(cmd)
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await?;
        if !out.status.success() {
            anyhow::bail!(
                "{} failed: {}",
                cmd,
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(String::from_utf8(out.stdout)?.trim().to_string())
    }

    pub(super) async fn sh_owned(&self, cmd: &str, args: Vec<String>) -> anyhow::Result<String> {
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        self.sh(cmd, &refs).await
    }

    pub(super) async fn sh_ok(&self, cmd: &str, args: &[&str]) -> bool {
        self.sh(cmd, args).await.is_ok()
    }

    pub(super) fn ensure_window_id(id: &str) -> anyhow::Result<()> {
        if id.trim().is_empty() {
            anyhow::bail!("window id must not be empty");
        }
        Ok(())
    }

    pub(super) async fn xrandr_monitors(&self) -> anyhow::Result<Vec<protocol::MonitorInfo>> {
        let out = self.sh("xrandr", &["--query"]).await?;
        Ok(parse_xrandr_query(&out))
    }
}
