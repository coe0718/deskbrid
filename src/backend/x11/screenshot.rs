use super::*;
use crate::protocol;
use crate::protocol::Region;

pub(super) async fn screenshot(
    backend: &X11Backend,
    _monitor: Option<u32>,
    region: Option<Region>,
    _window_id: Option<String>,
) -> anyhow::Result<protocol::ScreenshotResult> {
    let path = crate::daemon::helpers::screenshot_temp_path().await;
    if let Some(r) = region {
        let geo = format!("{}x{}+{}+{}", r.width, r.height, r.x, r.y);
        backend
            .sh("import", &["-window", "root", "-crop", &geo, &path])
            .await?;
        Ok(protocol::ScreenshotResult {
            path,
            width: r.width,
            height: r.height,
            format: "png".into(),
        })
    } else {
        backend.sh("import", &["-window", "root", &path]).await?;
        let dims = backend
            .sh("identify", &["-format", "%w %h", &path])
            .await
            .unwrap_or_else(|_| "0 0".into());
        let mut parts = dims.split_whitespace();
        let w: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let h: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        Ok(protocol::ScreenshotResult {
            path,
            width: w,
            height: h,
            format: "png".into(),
        })
    }
}
