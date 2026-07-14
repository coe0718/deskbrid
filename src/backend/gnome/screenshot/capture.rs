use crate::backend::gnome::GnomeBackend;
use crate::protocol::{self, Region};
use anyhow::Context;
use tokio::process::Command;

use super::crop::{CropRect, crop_png, get_png_dimensions};

impl GnomeBackend {
    pub(crate) async fn screenshot_inner(
        &self,
        monitor: Option<u32>,
        region: Option<Region>,
        window_id: Option<String>,
    ) -> anyhow::Result<protocol::ScreenshotResult> {
        let path = crate::daemon::helpers::screenshot_temp_path().await;

        // Fast path: use existing Mutter ScreenCast PipeWire stream (no dialogs)
        if monitor.is_none()
            && region.is_none()
            && window_id.is_none()
            && self.sc_pw_node > 0
            && self.screenshot_via_pipewire(&path).await.is_ok()
        {
            let dims = get_png_dimensions(&path).await?;
            return Ok(protocol::ScreenshotResult {
                path,
                width: dims.0,
                height: dims.1,
                format: "png".into(),
            });
        }

        // Build a grim-compatible region string if we have geometry
        let capture_region: Option<CropRect> = if let Some(ref wid) = window_id {
            let info = self.resolve_window(wid).await?;
            let geo = info
                .geometry
                .ok_or_else(|| anyhow::anyhow!("window has no geometry: {}", wid))?;
            Some(CropRect {
                x: geo.x,
                y: geo.y,
                width: geo.width,
                height: geo.height,
            })
        } else {
            region.as_ref().map(CropRect::from_region)
        };

        // Try grim first (works on wlroots-based compositors, fast-path)
        let grim_ok = if let Some(ref cap) = capture_region {
            let geometry = cap.to_grim_geometry();
            self.sh("grim", &["-g", &geometry, &path]).await.is_ok()
        } else if let Some(idx) = monitor {
            let monitors = self.get_monitors().await?;
            let name = monitors
                .get(idx as usize)
                .map(|m| m.name.clone())
                .unwrap_or_else(|| idx.to_string());
            self.sh("grim", &["-o", &name, &path]).await.is_ok()
        } else {
            self.sh("grim", &[&path]).await.is_ok()
        };

        // If grim failed (GNOME Wayland — no wlr-screencopy), try fallbacks
        if !grim_ok {
            super::fallback::run_fallbacks(self, &path).await?;

            if let Some(crop) = capture_region {
                crop_png(&path, crop)?;
            } else if let Some(idx) = monitor {
                let monitors = self.get_monitors().await?;
                let mon = monitors
                    .get(idx as usize)
                    .ok_or_else(|| anyhow::anyhow!("monitor index out of range: {}", idx))?;
                crop_png(
                    &path,
                    CropRect {
                        x: mon.x,
                        y: mon.y,
                        width: mon.width,
                        height: mon.height,
                    },
                )?;
            }
        }

        let dims = get_png_dimensions(&path).await?;
        Ok(protocol::ScreenshotResult {
            path,
            width: dims.0,
            height: dims.1,
            format: "png".into(),
        })
    }

    /// Fast path: capture a single frame from the existing Mutter ScreenCast
    /// PipeWire stream. No dialogs, no portal — just grabs the current frame.
    async fn screenshot_via_pipewire(&self, output_path: &str) -> anyhow::Result<()> {
        let node_id = self.sc_pw_node;
        let output = Command::new("gst-launch-1.0")
            .args([
                "-q",
                "pipewiresrc",
                &format!("path={}", node_id),
                "!",
                "videoconvert",
                "!",
                "pngenc",
                "snapshot=true",
                "!",
                "filesink",
                &format!("location={}", output_path),
            ])
            .output()
            .await
            .context("running gst-launch pipewiresrc")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("pipewire screenshot failed: {}", stderr);
        }
        Ok(())
    }
}
