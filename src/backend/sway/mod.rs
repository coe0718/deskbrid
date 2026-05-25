use crate::backend::DesktopBackend;
use crate::protocol::DeskbridEvent;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::process::Command;
use tokio::sync::broadcast;

pub(crate) mod helpers;

use helpers::*;

pub struct SwayBackend {
    pub(super) event_tx: broadcast::Sender<DeskbridEvent>,
    pub(super) watchers: Arc<Mutex<HashMap<String, notify::RecommendedWatcher>>>,
    pub(super) sway_socket: Option<String>,
    pub(super) xdg_runtime: String,
}

impl SwayBackend {
    pub async fn new(event_tx: broadcast::Sender<DeskbridEvent>) -> anyhow::Result<Self> {
        let sway_socket = std::env::var("SWAYSOCK").ok();
        let xdg_runtime = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");

        if sway_socket.is_none() {
            eprintln!("[deskbrid] WARN: SWAYSOCK not set — swaymsg may fail");
        }

        Ok(Self {
            event_tx,
            watchers: Arc::new(Mutex::new(HashMap::new())),
            sway_socket,
            xdg_runtime,
        })
    }

    async fn swaymsg_json(&self, args: &[&str]) -> anyhow::Result<serde_json::Value> {
        let mut cmd = Command::new("swaymsg");
        cmd.args(args).stdin(Stdio::null()).stderr(Stdio::piped());
        self.apply_env(&mut cmd);
        let output = cmd.output().await?;
        if !output.status.success() {
            anyhow::bail!(
                "swaymsg failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(serde_json::from_str(&String::from_utf8(output.stdout)?)?)
    }

    /// Run arbitrary swaymsg commands (no JSON output expected).
    async fn swaymsg_raw(&self, args: &[&str]) -> anyhow::Result<()> {
        let mut cmd = Command::new("swaymsg");
        cmd.args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        self.apply_env(&mut cmd);
        let output = cmd.output().await?;
        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let detail = if stdout.is_empty() { stderr } else { stdout };
            anyhow::bail!("swaymsg failed: {}", detail);
        }
        Ok(())
    }

    async fn sh(&self, cmd: &str, args: &[&str]) -> anyhow::Result<String> {
        let mut c = Command::new(cmd);
        c.args(args).stdin(Stdio::null()).stderr(Stdio::piped());
        self.apply_env(&mut c);
        let out = c.output().await?;
        if !out.status.success() {
            anyhow::bail!(
                "{} failed: {}",
                cmd,
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(String::from_utf8(out.stdout)?.trim().to_string())
    }

    fn apply_env(&self, cmd: &mut Command) {
        cmd.env("XDG_RUNTIME_DIR", &self.xdg_runtime);
        if let Some(ref sock) = self.sway_socket {
            cmd.env("SWAYSOCK", sock);
        }
    }

    async fn ydotool(&self, args: &[&str]) -> anyhow::Result<()> {
        self.sh("ydotool", args).await.map(|_| ())
    }
}

mod audio;
mod bluetooth;
mod files;
mod keyboard;
mod monitor;
mod networking;
mod notifications;
mod screenshot;
mod system_info;
mod trait_impl;
mod windows;
mod workspaces;
