//! Screen capture fallbacks using external tools.

use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::process::Command;

pub async fn fallback_screenshot(_monitor: Option<u32>) -> Result<String> {
    let path = screenshot_path("screenshot", "png").await?;

    let gnome = Command::new("gnome-screenshot")
        .arg("-f")
        .arg(&path)
        .output()
        .await;
    match gnome {
        Ok(output) if output.status.success() => return Ok(path.display().to_string()),
        Ok(_) | Err(_) => {}
    }

    let grim = Command::new("grim")
        .arg(&path)
        .output()
        .await
        .context("running grim fallback")?;
    if !grim.status.success() {
        return Err(anyhow!(
            "grim failed: {}",
            String::from_utf8_lossy(&grim.stderr)
        ));
    }

    Ok(path.display().to_string())
}

pub async fn screenshot_path(prefix: &str, extension: &str) -> Result<PathBuf> {
    let directory = PathBuf::from("/tmp/deskbrid");
    tokio::fs::create_dir_all(&directory)
        .await
        .context("creating screenshot output dir")?;
    Ok(directory.join(format!("{prefix}_{}.{}", unix_ts(), extension)))
}

fn unix_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
