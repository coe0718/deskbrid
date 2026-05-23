use crate::backend::DesktopBackend;
use crate::protocol::DeskbridEvent;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::process::Command;
use tokio::sync::broadcast;

pub(crate) mod helpers;
use helpers::*;

pub struct WayfireBackend {
    pub(super) event_tx: broadcast::Sender<DeskbridEvent>,
    pub(super) watchers: Arc<Mutex<HashMap<String, notify::RecommendedWatcher>>>,
    pub(super) xdg_runtime: String,
}

impl WayfireBackend {
    pub async fn new(event_tx: broadcast::Sender<DeskbridEvent>) -> anyhow::Result<Self> {
        let xdg_runtime = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
        Ok(Self {
            event_tx,
            watchers: Arc::new(Mutex::new(HashMap::new())),
            xdg_runtime,
        })
    }

    async fn wf_ipc_json(&self, args: &[&str]) -> anyhow::Result<serde_json::Value> {
        let mut cmd = Command::new("wf-ipc");
        cmd.args(args).stdin(Stdio::null()).stderr(Stdio::piped());
        self.apply_env(&mut cmd);
        let output = cmd.output().await?;
        if !output.status.success() {
            anyhow::bail!(
                "wf-ipc failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(serde_json::from_str(&String::from_utf8(output.stdout)?)?)
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
    }

    async fn ydotool(&self, args: &[&str]) -> anyhow::Result<()> {
        self.sh("ydotool", args).await.map(|_| ())
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
