//! Screen capture fallbacks using external tools.
//! PipeWire screencast will replace this in Phase 3.

use anyhow::{Context, Result, anyhow};
use std::path::PathBuf;
use tokio::process::Command;

const PORTAL_SCREENSHOT_SCRIPT: &str = include_str!("../scripts/screenshot_portal.py");

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
    let portal = Command::new("python3")
        .arg("-c")
        .arg(PORTAL_SCREENSHOT_SCRIPT)
        .arg(&path)
        .output()
        .await
        .context("running portal screenshot script")?;

    if portal.status.success() && path.exists() {
        return Ok(path.display().to_string());
    }

    Err(anyhow!(
        "no screenshot method available (tried gnome-screenshot, xdg-desktop-portal)"
    ))
}
