use crate::protocol::BacklightInfo;

/// Scan /sys/class/backlight/ for devices.
pub async fn backlight_list() -> anyhow::Result<Vec<BacklightInfo>> {
    let mut devices = Vec::new();
    let mut dir = match tokio::fs::read_dir("/sys/class/backlight").await {
        Ok(d) => d,
        Err(_) => anyhow::bail!("no backlight devices found"),
    };
    while let Some(entry) = dir.next_entry().await? {
        let device = entry.file_name().to_string_lossy().to_string();
        let base = entry.path();
        let max = tokio::fs::read_to_string(base.join("max_brightness"))
            .await
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(0);
        let cur = tokio::fs::read_to_string(base.join("brightness"))
            .await
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(0);
        let pct = if max > 0 {
            ((cur as f64 / max as f64) * 100.0).round() as u8
        } else {
            0
        };
        devices.push(BacklightInfo {
            device,
            max_brightness: max,
            brightness: cur,
            percentage: pct,
        });
    }
    if devices.is_empty() {
        anyhow::bail!("no backlight devices found");
    }
    Ok(devices)
}

/// Get brightness for a specific device (or first available if empty).
pub async fn backlight_get(device: Option<&str>) -> anyhow::Result<BacklightInfo> {
    let devices = backlight_list().await?;
    if let Some(name) = device {
        devices
            .into_iter()
            .find(|d| d.device == name)
            .ok_or_else(|| anyhow::anyhow!("backlight device not found: {name}"))
    } else {
        devices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("no backlight devices"))
    }
}

/// Set brightness. Accepts absolute value (0..max) or percentage string ("50%").
pub async fn backlight_set(device: Option<&str>, value: &str) -> anyhow::Result<BacklightInfo> {
    let info = backlight_get(device).await?;
    let new_val: u32 = if let Some(pct) = value.strip_suffix('%') {
        let p: f64 = pct
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid percentage: {value}"))?;
        if !(0.0..=100.0).contains(&p) {
            anyhow::bail!("percentage must be 0-100");
        }
        ((p / 100.0) * info.max_brightness as f64).round() as u32
    } else {
        value
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid brightness value: {value}"))?
    };

    let path = format!("/sys/class/backlight/{}/brightness", info.device);
    tokio::fs::write(&path, new_val.to_string()).await?;

    backlight_get(Some(&info.device)).await
}
