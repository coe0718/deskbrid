//! COSMIC desktop backend — controls windows/workspaces via `cosmic-helper`
//! subprocess. Shares input, clipboard, screenshot, and notification tooling
//! with the Hyprland backend pattern.

use crate::backend::DesktopBackend;
use crate::protocol;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::Mutex;

pub(crate) mod helpers;

use helpers::*;

pub struct CosmicBackend {
    #[allow(dead_code)]
    event_tx: tokio::sync::broadcast::Sender<protocol::DeskbridEvent>,
    #[allow(dead_code)]
    watchers: Arc<Mutex<HashMap<String, notify::RecommendedWatcher>>>,
    last_mouse: std::sync::Mutex<(f64, f64)>,
    wl_socket: Option<String>,
    xdg_runtime: String,
    /// Path to the cosmic-helper binary
    helper_path: String,
}

impl CosmicBackend {
    pub async fn new(
        event_tx: tokio::sync::broadcast::Sender<protocol::DeskbridEvent>,
    ) -> anyhow::Result<Self> {
        let xdg_runtime = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
        let wl_socket = std::env::var("WAYLAND_DISPLAY").ok();

        // Find cosmic-helper binary: next to our binary, then on PATH
        let helper_path = Self::find_helper();

        let backend = Self {
            event_tx,
            watchers: Arc::new(Mutex::new(HashMap::new())),
            last_mouse: std::sync::Mutex::new((960.0, 540.0)),
            wl_socket,
            xdg_runtime,
            helper_path,
        };

        Ok(backend)
    }

    fn find_helper() -> String {
        // Check if sibling binary exists
        if let Ok(exe) = std::env::current_exe() {
            let sibling = exe.parent().unwrap().join("cosmic-helper");
            if sibling.exists() {
                return sibling.to_string_lossy().to_string();
            }
        }
        // Fallback to PATH
        "cosmic-helper".to_string()
    }

    /// Run cosmic-helper CLI and parse JSON output
    async fn helper_json(&self, args: &[&str]) -> anyhow::Result<serde_json::Value> {
        let output = Command::new(&self.helper_path)
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .env("XDG_RUNTIME_DIR", &self.xdg_runtime)
            .env(
                "WAYLAND_DISPLAY",
                self.wl_socket.as_deref().unwrap_or("wayland-0"),
            )
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("cosmic-helper failed: {}", stderr.trim());
        }

        let stdout = String::from_utf8(output.stdout)?;
        if stdout.trim().is_empty() || stdout.trim() == "null" {
            return Ok(serde_json::Value::Null);
        }

        Ok(serde_json::from_str(&stdout)?)
    }

    /// Run cosmic-helper CLI, check exit code and JSON response
    async fn helper_run(&self, args: &[&str]) -> anyhow::Result<()> {
        let output = Command::new(&self.helper_path)
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .env("XDG_RUNTIME_DIR", &self.xdg_runtime)
            .env(
                "WAYLAND_DISPLAY",
                self.wl_socket.as_deref().unwrap_or("wayland-0"),
            )
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "cosmic-helper '{}' failed: {}",
                args.join(" "),
                stderr.trim()
            );
        }

        // Check JSON response for {"ok": false} — helper may exit 0
        // even when the operation wasn't actually performed.
        if let Ok(body) = String::from_utf8(output.stdout)
            && let Ok(resp) = serde_json::from_str::<serde_json::Value>(&body)
            && resp.get("ok").and_then(|v| v.as_bool()) == Some(false)
        {
            let detail = resp
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            anyhow::bail!("cosmic-helper '{}' failed: {}", args.join(" "), detail);
        }

        Ok(())
    }

    /// Run a command and return stdout
    async fn sh(&self, cmd: &str, args: &[&str]) -> anyhow::Result<String> {
        let mut command = Command::new(cmd);
        command
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::piped());
        command.env("XDG_RUNTIME_DIR", &self.xdg_runtime);
        if let Some(sock) = &self.wl_socket {
            command.env("WAYLAND_DISPLAY", sock);
        }
        let output = command.output().await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("{} failed: {}", cmd, stderr.trim());
        }
        Ok(String::from_utf8(output.stdout)?)
    }

    /// Run a command with owned String args (delegates to sh).
    async fn sh_owned(&self, cmd: &str, args: Vec<String>) -> anyhow::Result<String> {
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        self.sh(cmd, &refs).await
    }
}

mod audio;
mod bluetooth;
mod files;
mod monitor;
mod networking;
mod notifications;
mod screenshot;
mod system_info;
mod trait_impl;
mod windows;
mod workspaces;
