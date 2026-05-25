use super::GnomeBackend;
use crate::protocol::{self, Region};
use anyhow::Context;
use tokio::process::Command;
use tokio::time::{Duration, timeout};

const PORTAL_SCREENSHOT_SCRIPT: &str = include_str!("../../../scripts/screenshot_portal.py");

#[derive(Clone, Copy)]
struct CropRect {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl CropRect {
    fn from_region(region: &Region) -> Self {
        Self {
            x: region.x as i32,
            y: region.y as i32,
            width: region.width,
            height: region.height,
        }
    }

    fn to_grim_geometry(self) -> String {
        format!("{}x{}+{}+{}", self.width, self.height, self.x, self.y)
    }
}

impl GnomeBackend {
    pub(super) async fn screenshot_inner(
        &self,
        monitor: Option<u32>,
        region: Option<Region>,
        window_id: Option<String>,
    ) -> anyhow::Result<protocol::ScreenshotResult> {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let path = format!("/tmp/deskbrid_screenshot_{}.png", ts);

        // Fast path: use existing Mutter ScreenCast PipeWire stream (no dialogs)
        if monitor.is_none()
            && region.is_none()
            && window_id.is_none()
            && self.sc_pw_node > 0
            && self.screenshot_via_pipewire(&path).await.is_ok()
        {
            let dims = get_png_dimensions(&path)?;
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

        // If grim failed (GNOME Wayland — no wlr-screencopy), try multiple fallbacks
        if !grim_ok {
            // Fallback 1: GNOME Shell extension (may hang on GNOME 47+)
            let ext_ok =
                timeout(Duration::from_secs(5), self.screenshot_via_extension(&path)).await;

            match ext_ok {
                Ok(Ok(())) => {} // Extension worked
                _ => {
                    // Fallback 2: XDG Desktop Portal (ScreenCast)
                    self.screenshot_via_portal(&path)
                        .await
                        .context("all screenshot methods failed (grim, extension, portal)")?;
                }
            }

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

        let dims = get_png_dimensions(&path)?;
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

    /// Fallback: screenshot via XDG Desktop Portal (ScreenCast).
    /// Uses an external Python script that talks PipeWire.
    async fn screenshot_via_portal(&self, output_path: &str) -> anyhow::Result<()> {
        let output = Command::new("python3")
            .arg("-c")
            .arg(PORTAL_SCREENSHOT_SCRIPT)
            .arg(output_path)
            .output()
            .await
            .context("running portal screenshot script")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("portal script failed: {}", stderr);
        }
        Ok(())
    }

    /// Take a screenshot via the deskbrid GNOME Shell extension.
    /// The extension runs inside GNOME Shell and has access to its screenshot API.
    /// NOTE: This method may hang on GNOME 47+ — callers should use a timeout.
    async fn screenshot_via_extension(&self, output_path: &str) -> anyhow::Result<()> {
        const DBUS_SERVICE: &str = "org.deskbrid.WindowManager";
        const DBUS_PATH: &str = "/org/deskbrid/WindowManager";
        const DBUS_IFACE: &str = "org.deskbrid.WindowManager";

        let reply = self
            .conn
            .call_method(
                Some(DBUS_SERVICE),
                DBUS_PATH,
                Some(DBUS_IFACE),
                "Screenshot",
                &(output_path,),
            )
            .await
            .map_err(|e| anyhow::anyhow!("extension Screenshot call failed: {e}"))?;

        let success: bool = reply.body().deserialize()?;
        if !success {
            anyhow::bail!("extension screenshot returned false");
        }
        Ok(())
    }
}

fn get_png_dimensions(path: &str) -> anyhow::Result<(u32, u32)> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut header = [0u8; 24];
    file.read_exact(&mut header)?;
    let width = u32::from_be_bytes([header[16], header[17], header[18], header[19]]);
    let height = u32::from_be_bytes([header[20], header[21], header[22], header[23]]);
    Ok((width, height))
}

fn crop_png(path: &str, rect: CropRect) -> anyhow::Result<()> {
    let image = image::open(path)
        .with_context(|| format!("opening screenshot for crop: {}", path))?
        .to_rgba8();
    let (image_width, image_height) = image.dimensions();

    if rect.x < 0
        || rect.y < 0
        || rect.width == 0
        || rect.height == 0
        || rect.x as u64 + rect.width as u64 > image_width as u64
        || rect.y as u64 + rect.height as u64 > image_height as u64
    {
        anyhow::bail!(
            "requested screenshot crop {}x{}+{}+{} is outside captured image {}x{}",
            rect.width,
            rect.height,
            rect.x,
            rect.y,
            image_width,
            image_height
        );
    }

    let cropped = image::imageops::crop_imm(
        &image,
        rect.x as u32,
        rect.y as u32,
        rect.width,
        rect.height,
    )
    .to_image();
    cropped
        .save(path)
        .with_context(|| format!("saving cropped screenshot: {}", path))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{CropRect, crop_png, get_png_dimensions};
    use image::{ImageBuffer, Rgba};

    fn temp_png_path(name: &str) -> String {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("/tmp/deskbrid_{}_{}_{}.png", name, std::process::id(), ts)
    }

    #[test]
    fn crop_png_rewrites_file_to_requested_region() {
        let path = temp_png_path("crop");
        let image = ImageBuffer::from_fn(4, 3, |x, y| Rgba([x as u8, y as u8, 0, 255]));
        image.save(&path).unwrap();

        crop_png(
            &path,
            CropRect {
                x: 1,
                y: 1,
                width: 2,
                height: 1,
            },
        )
        .unwrap();

        assert_eq!(get_png_dimensions(&path).unwrap(), (2, 1));
        let cropped = image::open(&path).unwrap().to_rgba8();
        assert_eq!(cropped.get_pixel(0, 0).0, [1, 1, 0, 255]);
        assert_eq!(cropped.get_pixel(1, 0).0, [2, 1, 0, 255]);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn crop_png_rejects_out_of_bounds_region() {
        let path = temp_png_path("crop_oob");
        let image = ImageBuffer::from_pixel(2, 2, Rgba([0u8, 0, 0, 255]));
        image.save(&path).unwrap();

        let err = crop_png(
            &path,
            CropRect {
                x: 1,
                y: 1,
                width: 2,
                height: 1,
            },
        )
        .unwrap_err();

        assert!(err.to_string().contains("outside captured image"));

        let _ = std::fs::remove_file(path);
    }
}
