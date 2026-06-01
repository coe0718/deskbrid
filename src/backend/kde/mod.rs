// KDE Plasma backend — split across submodules (under 250 lines each).
use crate::protocol;
use crate::protocol::DeskbridEvent;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::process::Command;
use tokio::sync::broadcast;

mod desktop_settings;
pub(crate) mod helpers;
mod io;
mod keyboard_layout;
mod kwin_scripts;
mod networking;
mod screenshot;
mod system;
#[cfg(test)]
mod tests;
mod trait_impl;
mod windows_core;
mod windows_layout;
mod workspaces;

use helpers::*;

pub struct KdeBackend {
    pub(super) event_tx: broadcast::Sender<DeskbridEvent>,
    pub(super) watchers: Arc<Mutex<HashMap<String, notify::RecommendedWatcher>>>,
    pub(super) xdg_runtime: String,
    pub(super) wl_socket: Option<String>,
}

impl KdeBackend {
    pub async fn new(event_tx: broadcast::Sender<DeskbridEvent>) -> anyhow::Result<Self> {
        let xdg_runtime = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
        let wl_socket = std::env::var("WAYLAND_DISPLAY").ok();
        eprintln!("[deskbrid] KDE backend initialized (xdg={xdg_runtime})");
        Ok(Self {
            event_tx,
            watchers: Arc::new(Mutex::new(HashMap::new())),
            xdg_runtime,
            wl_socket,
        })
    }

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

    pub(super) async fn sh_owned(&self, cmd: &str, args: Vec<String>) -> anyhow::Result<String> {
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        self.sh(cmd, &refs).await
    }

    pub(super) async fn qdbus(
        &self,
        service: &str,
        path: &str,
        method: &str,
        args: &[&str],
    ) -> anyhow::Result<String> {
        let mut all_args = vec![service, path, method];
        all_args.extend_from_slice(args);
        self.sh("qdbus6", &all_args).await
    }

    pub(super) async fn kwin_js(&self, js: &str) -> anyhow::Result<Vec<String>> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap();
        let marker = format!("KWIN_DESKBRID_{}", now.as_nanos());
        let wrapped = format!("print(\"{}\");\n{js}\nprint(\"{}\");", marker, marker);
        let tmp = format!("/tmp/deskbrid_kwin_{}.js", std::process::id());
        tokio::fs::write(&tmp, wrapped.as_bytes()).await?;

        let resp = self
            .sh(
                "dbus-send",
                &[
                    "--print-reply",
                    "--dest=org.kde.KWin",
                    "/Scripting",
                    "org.kde.kwin.Scripting.loadScript",
                    &format!("string:{}", tmp),
                ],
            )
            .await?;

        let num = resp
            .split_whitespace()
            .filter_map(|w| w.parse::<u32>().ok())
            .next()
            .ok_or_else(|| anyhow::anyhow!("could not parse script number: {}", resp))?;

        self.sh(
            "dbus-send",
            &[
                "--print-reply",
                "--dest=org.kde.KWin",
                &format!("/Scripting/Script{}", num),
                "org.kde.kwin.Script.run",
            ],
        )
        .await
        .ok();

        let mut out = String::new();
        for _ in 0..10 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let resp = self
                .sh(
                    "journalctl",
                    &[
                        "--since",
                        "30 seconds ago",
                        "_COMM=kwin_wayland",
                        "-o",
                        "cat",
                        "-n",
                        "300",
                    ],
                )
                .await
                .unwrap_or_default();
            if resp.contains(&marker) {
                out = resp;
                break;
            }
        }

        self.sh(
            "dbus-send",
            &[
                "--dest=org.kde.KWin",
                &format!("/Scripting/Script{}", num),
                "org.kde.kwin.Script.stop",
            ],
        )
        .await
        .ok();

        let _ = tokio::fs::remove_file(&tmp).await;

        let mut in_block = false;
        let mut results = Vec::new();
        for line in out.lines() {
            let trimmed = line.trim();
            if trimmed == marker {
                in_block = !in_block;
                continue;
            }
            if in_block {
                results.push(trimmed.strip_prefix("js: ").unwrap_or(trimmed).to_string());
            }
        }
        Ok(results)
    }
}
