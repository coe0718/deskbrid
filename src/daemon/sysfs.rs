use serde_json::Value;
use std::path::{Path, PathBuf};

const BACKLIGHT_ROOT: &str = "/sys/class/backlight";

pub async fn backlight_get(device: Option<&str>) -> anyhow::Result<Value> {
    let devices = read_backlights().await?;
    if let Some(device) = device {
        let Some(entry) = devices.iter().find(|entry| entry["name"] == device) else {
            anyhow::bail!("backlight device not found: {}", device);
        };
        return Ok(serde_json::json!({"device": entry, "devices": devices}));
    }
    Ok(serde_json::json!({"devices": devices}))
}

pub async fn backlight_set(percent: f64, device: Option<&str>) -> anyhow::Result<Value> {
    let selected = select_backlight(device).await?;
    let max = read_u64(&selected.join("max_brightness")).await?;
    if max == 0 {
        anyhow::bail!("backlight device has zero max_brightness");
    }
    let value = ((max as f64) * (percent / 100.0)).round() as u64;
    tokio::fs::write(selected.join("brightness"), value.to_string()).await?;
    let name = selected
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string();
    Ok(serde_json::json!({
        "device": name,
        "percent": percent,
        "brightness": value,
        "max_brightness": max
    }))
}

async fn read_backlights() -> anyhow::Result<Vec<Value>> {
    let mut entries = match tokio::fs::read_dir(BACKLIGHT_ROOT).await {
        Ok(entries) => entries,
        Err(_) => return Ok(Vec::new()),
    };
    let mut devices = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let max = read_u64(&path.join("max_brightness")).await.unwrap_or(0);
        let brightness = read_u64(&path.join("brightness")).await.unwrap_or(0);
        let percent = if max > 0 {
            (brightness as f64 / max as f64) * 100.0
        } else {
            0.0
        };
        devices.push(serde_json::json!({
            "name": name,
            "brightness": brightness,
            "max_brightness": max,
            "percent": percent,
            "writable": writable(&path.join("brightness")).await
        }));
    }
    devices.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    Ok(devices)
}

async fn select_backlight(device: Option<&str>) -> anyhow::Result<PathBuf> {
    let root = Path::new(BACKLIGHT_ROOT);
    if let Some(device) = device {
        let path = root.join(device);
        if tokio::fs::metadata(&path).await.is_ok() {
            return Ok(path);
        }
        anyhow::bail!("backlight device not found: {}", device);
    }

    let mut entries = tokio::fs::read_dir(root).await?;
    while let Some(entry) = entries.next_entry().await? {
        return Ok(entry.path());
    }
    anyhow::bail!("no backlight devices found")
}

async fn read_u64(path: &Path) -> anyhow::Result<u64> {
    Ok(tokio::fs::read_to_string(path).await?.trim().parse()?)
}

async fn writable(path: &Path) -> bool {
    tokio::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .await
        .is_ok()
}
