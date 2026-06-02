use super::*;
use crate::protocol;

pub(super) async fn screenshot(
    backend: &HyprBackend,
    monitor: Option<u32>,
    region: Option<protocol::Region>,
    window_id: Option<String>,
) -> anyhow::Result<protocol::ScreenshotResult> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let path = format!("/tmp/deskbrid_screenshot_{}.png", ts);
    if let Some(ref wid) = window_id {
        let info = windows::window_get(backend, wid).await?;
        if let Some(geo) = info.geometry {
            let region_str = format!("{},{} {}x{}", geo.x, geo.y, geo.width, geo.height);
            backend.sh("grim", &["-g", &region_str, &path]).await?;
            return Ok(protocol::ScreenshotResult {
                path: path.clone(),
                width: geo.width,
                height: geo.height,
                format: "png".into(),
            });
        }
    }
    if let Some(ref r) = region {
        let region_str = format!("{},{} {}x{}", r.x, r.y, r.width, r.height);
        backend.sh("grim", &["-g", &region_str, &path]).await?;
        return Ok(protocol::ScreenshotResult {
            path: path.clone(),
            width: r.width,
            height: r.height,
            format: "png".into(),
        });
    }
    if let Some(idx) = monitor {
        let monitors = {
            let m = backend.monitors.lock().unwrap();
            m.clone()
        };
        let name = monitors
            .get(idx as usize)
            .map(|m| m.name.clone())
            .unwrap_or_else(|| idx.to_string());
        backend.sh("grim", &["-o", &name, &path]).await?;
    } else {
        backend.sh("grim", &[&path]).await?;
    }
    let dims = get_png_dimensions(&path).await?;
    Ok(protocol::ScreenshotResult {
        path,
        width: dims.0,
        height: dims.1,
        format: "png".into(),
    })
}
