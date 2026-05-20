use super::GnomeBackend;
use crate::protocol::{self, Region};

impl GnomeBackend {
    pub(super) async fn screenshot_inner(
        &self,
        monitor: Option<u32>,
        region: Option<Region>,
        window_id: Option<String>,
    ) -> anyhow::Result<protocol::ScreenshotResult> {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        let path = format!("/tmp/deskbrid_screenshot_{}.png", ts);

        if let Some(ref wid) = window_id {
            let info = self.resolve_window(wid).await?;
            if let Some(geo) = info.geometry {
                let region_str = format!("{}x{}+{}+{}", geo.width, geo.height, geo.x, geo.y);
                self.sh("grim", &["-g", &region_str, &path]).await?;
                return Ok(protocol::ScreenshotResult {
                    path: path.clone(),
                    width: geo.width,
                    height: geo.height,
                    format: "png".into(),
                });
            }
        }

        if let Some(ref r) = region {
            let region_str = format!("{}x{}+{}+{}", r.width, r.height, r.x, r.y);
            self.sh("grim", &["-g", &region_str, &path]).await?;
            return Ok(protocol::ScreenshotResult {
                path: path.clone(),
                width: r.width,
                height: r.height,
                format: "png".into(),
            });
        }

        if let Some(idx) = monitor {
            let monitors = self.get_monitors().await?;
            let name = monitors
                .get(idx as usize)
                .map(|m| m.name.clone())
                .unwrap_or_else(|| idx.to_string());
            self.sh("grim", &["-o", &name, &path]).await?;
        } else {
            self.sh("grim", &[&path]).await?;
        }

        let dims = get_png_dimensions(&path)?;
        Ok(protocol::ScreenshotResult {
            path,
            width: dims.0,
            height: dims.1,
            format: "png".into(),
        })
    }
}

fn get_png_dimensions(path: &str) -> anyhow::Result<(u32, u32)> {
    let data = std::fs::read(path)?;
    if data.len() < 24 {
        anyhow::bail!("PNG file too small");
    }
    let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    Ok((width, height))
}
