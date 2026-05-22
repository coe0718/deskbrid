use super::*;
use crate::protocol;

pub(super) async fn screenshot(
    backend: &CosmicBackend,
    _monitor: Option<u32>,
    region: Option<protocol::Region>,
    _window_id: Option<String>,
) -> anyhow::Result<protocol::ScreenshotResult> {
    let path = format!(
        "/tmp/deskbrid/screenshot_{}.png",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );

    // Ensure tmp dir exists
    let _ = tokio::fs::create_dir_all("/tmp/deskbrid").await;

    if let Some(r) = region {
        let geo = format!("{}x{}+{}+{}", r.width, r.height, r.x, r.y);
        backend.sh("grim", &["-g", &geo, &path]).await?;
    } else {
        backend.sh("grim", &[&path]).await?;
    }

    // Get dimensions from the file
    let dims_output = backend.sh("identify", &["-format", "%w %h", &path]).await?;
    let dims: Vec<&str> = dims_output.split_whitespace().collect();
    let width = dims.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let height = dims.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);

    Ok(protocol::ScreenshotResult {
        path,
        width,
        height,
        format: "png".to_string(),
    })
}

// ─── Notifications ──────────────────────────────────
