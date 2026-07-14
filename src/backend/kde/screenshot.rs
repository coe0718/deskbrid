use super::*;
use crate::protocol;

pub(super) async fn screenshot(
    backend: &KdeBackend,
    _monitor: Option<u32>,
    _region: Option<protocol::Region>,
    _window_id: Option<String>,
) -> anyhow::Result<protocol::ScreenshotResult> {
    let raw_path = crate::daemon::helpers::screenshot_temp_path().await;
    let out_path = crate::daemon::helpers::screenshot_temp_path().await;

    if let Some(ref wid) = _window_id {
        let info = crate::backend::DesktopBackend::window_get(backend, wid).await?;
        if let Some(geo) = info.geometry {
            backend
                .sh("spectacle", &["-b", "-n", "-o", &raw_path])
                .await?;
            let crop = format!("{}x{}+{}+{}", geo.width, geo.height, geo.x, geo.y);
            backend
                .sh("convert", &[&raw_path, "-crop", &crop, &out_path])
                .await?;
            tokio::fs::remove_file(&raw_path).await.ok();
            return Ok(protocol::ScreenshotResult {
                path: out_path,
                width: geo.width,
                height: geo.height,
                format: "png".into(),
            });
        }
    }

    if let Some(ref r) = _region {
        backend
            .sh("spectacle", &["-b", "-n", "-o", &raw_path])
            .await?;
        let crop = format!("{}x{}+{}+{}", r.width, r.height, r.x, r.y);
        backend
            .sh("convert", &[&raw_path, "-crop", &crop, &out_path])
            .await?;
        tokio::fs::remove_file(&raw_path).await.ok();
        return Ok(protocol::ScreenshotResult {
            path: out_path,
            width: r.width,
            height: r.height,
            format: "png".into(),
        });
    }

    backend
        .sh("spectacle", &["-b", "-n", "-o", &out_path])
        .await?;
    let dims = backend
        .sh("identify", &["-format", "%w %h", &out_path])
        .await
        .unwrap_or_default();
    let wh: Vec<u32> = dims
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect();

    Ok(protocol::ScreenshotResult {
        path: out_path,
        width: wh.first().copied().unwrap_or(0),
        height: wh.get(1).copied().unwrap_or(0),
        format: "png".into(),
    })
}

pub(super) async fn notification_send(
    backend: &KdeBackend,
    app_name: &str,
    title: &str,
    body: &str,
    urgency: &str,
) -> anyhow::Result<u32> {
    let urgency_map = match urgency {
        "critical" => "critical",
        "high" => "critical",
        "low" => "low",
        _ => "normal",
    };
    backend
        .sh(
            "notify-send",
            &["-a", app_name, "-u", urgency_map, title, body],
        )
        .await?;
    Ok(0)
}

pub(super) async fn notification_close(_backend: &KdeBackend, _id: u32) -> anyhow::Result<()> {
    anyhow::bail!("closing notifications is not supported by notify-send on KDE")
}
