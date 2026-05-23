use super::*;
use crate::protocol::DeskbridEvent;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::process::Command;
use tokio::sync::broadcast;

impl HyprBackend {
    pub async fn new(event_tx: broadcast::Sender<DeskbridEvent>) -> anyhow::Result<Self> {
        let (instance_sig, wl_socket) = detect_hypr_instance().await;
        let xdg_runtime =
            std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
        if let Some(ref sig) = instance_sig {
            if sig.is_empty() {
                eprintln!(
                    "[deskbrid] WARN: detected empty instance sig (found dirs but name empty)"
                );
            } else {
                eprintln!("[deskbrid] detected Hyprland instance: {sig}");
            }
        } else {
            eprintln!("[deskbrid] WARN: no Hyprland instance detected (xdg={xdg_runtime})");
        }

        let backend = Self {
            event_tx,
            watchers: Arc::new(Mutex::new(HashMap::new())),
            last_mouse: std::sync::Mutex::new((960.0, 540.0)),
            monitors: std::sync::Mutex::new(Vec::new()),
            instance_sig,
            wl_socket,
            xdg_runtime,
        };
        // Cache monitor list on startup
        if let Ok(monitors) = backend.monitors_inner().await
            && let Ok(mut m) = backend.monitors.lock()
        {
            *m = monitors;
        }
        Ok(backend)
    }

    /// Run `hyprctl` with JSON output, return parsed JSON value.
    pub(super) async fn hyprctl_json(&self, args: &[&str]) -> anyhow::Result<serde_json::Value> {
        let mut cmd = Command::new("hyprctl");
        cmd.args(args)
            .arg("-j")
            .stdin(Stdio::null())
            .stderr(Stdio::piped());
        if let Some(sig) = &self.instance_sig {
            cmd.env("HYPRLAND_INSTANCE_SIGNATURE", sig);
        }
        let output = cmd.output().await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("hyprctl failed: {}", stderr.trim());
        }
        let stdout = String::from_utf8(output.stdout)?;
        Ok(serde_json::from_str(&stdout)?)
    }

    /// Run `hyprctl dispatch` (no JSON output, just success/fail).
    pub(super) async fn hyprctl_dispatch(&self, dispatch: &str) -> anyhow::Result<()> {
        let mut cmd = Command::new("hyprctl");
        cmd.arg("dispatch")
            .arg(dispatch)
            .stdin(Stdio::null())
            .stderr(Stdio::piped());
        if let Some(sig) = &self.instance_sig {
            cmd.env("HYPRLAND_INSTANCE_SIGNATURE", sig);
        }
        let output = cmd.output().await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("hyprctl dispatch '{}' failed: {}", dispatch, stderr.trim());
        }
        Ok(())
    }

    /// Run `hyprctl keyword` for live compositor settings.
    pub(super) async fn hyprctl_keyword(&self, keyword: &str, value: &str) -> anyhow::Result<()> {
        let mut cmd = Command::new("hyprctl");
        cmd.arg("keyword")
            .arg(keyword)
            .arg(value)
            .stdin(Stdio::null())
            .stderr(Stdio::piped());
        if let Some(sig) = &self.instance_sig {
            cmd.env("HYPRLAND_INSTANCE_SIGNATURE", sig);
        }
        let output = cmd.output().await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("hyprctl keyword {} failed: {}", keyword, stderr.trim());
        }
        Ok(())
    }

    /// Run a shell command and return stdout.
    pub(super) async fn sh(&self, cmd: &str, args: &[&str]) -> anyhow::Result<String> {
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
        Ok(String::from_utf8(output.stdout)?.trim().to_string())
    }

    /// Run a command, return true if exit code is 0.
    pub(super) async fn sh_ok(&self, cmd: &str, args: &[&str]) -> bool {
        let mut command = Command::new(cmd);
        command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        command.env("XDG_RUNTIME_DIR", &self.xdg_runtime);
        if let Some(sock) = &self.wl_socket {
            command.env("WAYLAND_DISPLAY", sock);
        }
        if let Some(sig) = &self.instance_sig
            && !sig.is_empty()
        {
            command.env("HYPRLAND_INSTANCE_SIGNATURE", sig);
        }
        command.status().await.map(|s| s.success()).unwrap_or(false)
    }
}
