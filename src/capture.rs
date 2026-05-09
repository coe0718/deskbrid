//! Screen capture fallbacks using external tools.
//! PipeWire screencast will replace this in Phase 3.

use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;
use tokio::process::Command;

pub async fn fallback_screenshot(_monitor: Option<u32>) -> Result<String> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    let dir = PathBuf::from("/tmp/deskbrid");
    tokio::fs::create_dir_all(&dir).await?;
    let path = dir.join(format!("screenshot_{}.png", ts));

    // Try gnome-screenshot first
    let gnome = Command::new("gnome-screenshot")
        .arg("-f")
        .arg(&path)
        .output()
        .await;

    match gnome {
        Ok(output) if output.status.success() => return Ok(path.display().to_string()),
        _ => {}
    }

    // Fallback: XDG Desktop Portal
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/coemedia".to_string());
    let script_path = PathBuf::from(home)
        .join("projects/deskbrid/scripts/screenshot_portal.py");

    let portal = Command::new("python3")
        .arg(&script_path)
        .output()
        .await
        .context("running portal screenshot script")?;

    if portal.status.success() {
        let portal_path = String::from_utf8_lossy(&portal.stdout).trim().to_string();
        if !portal_path.is_empty() {
            return Ok(portal_path);
        }
    }

    Err(anyhow!(
        "no screenshot method available (tried gnome-screenshot, xdg-desktop-portal)"
    ))
}
