//! Screen capture fallbacks using external tools.
//! PipeWire screencast will replace this in Phase 3.

use anyhow::{Context, Result, anyhow};
use tokio::process::Command;

const PORTAL_SCREENSHOT_SCRIPT: &str = include_str!("../scripts/screenshot_portal.py");

pub async fn fallback_screenshot(_monitor: Option<u32>) -> Result<String> {
    let path = crate::daemon::helpers::screenshot_temp_path();

    // Try gnome-screenshot first
    let gnome = Command::new("gnome-screenshot")
        .arg("-f")
        .arg(&path)
        .output()
        .await;

    match gnome {
        Ok(output) if output.status.success() => return Ok(path),
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

    if portal.status.success() && std::path::Path::new(&path).exists() {
        return Ok(path);
    }

    Err(anyhow!(
        "no screenshot method available (tried gnome-screenshot, xdg-desktop-portal)"
    ))
}
