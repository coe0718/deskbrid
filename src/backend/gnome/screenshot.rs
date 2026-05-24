use super::GnomeBackend;
use crate::protocol::{self, Region};
use anyhow::Context;
use tokio::time::{timeout, Duration};
use tokio::process::Command;
use std::path::PathBuf;

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
        if self.sc_pw_node > 0 {
            if self.screenshot_via_pipewire(&path).await.is_ok() {
                let dims = get_png_dimensions(&path)?;
                return Ok(protocol::ScreenshotResult {
                    path,
                    width: dims.0,
                    height: dims.1,
                    format: "png".into(),
                });
            }
        }

        // Build a grim-compatible region string if we have geometry
        let capture_region: Option<String> = if let Some(ref wid) = window_id {
            let info = self.resolve_window(wid).await?;
            info.geometry
                .map(|geo| format!("{}x{}+{}+{}", geo.width, geo.height, geo.x, geo.y))
        } else {
            region
                .as_ref()
                .map(|r| format!("{}x{}+{}+{}", r.width, r.height, r.x, r.y))
        };

        // Try grim first (works on wlroots-based compositors, fast-path)
        let grim_ok = if let Some(ref cap) = capture_region {
            self.sh("grim", &["-g", cap, &path]).await.is_ok()
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
            let ext_ok = timeout(
                Duration::from_secs(5),
                self.screenshot_via_extension(&path),
            ).await;

            match ext_ok {
                Ok(Ok(())) => {} // Extension worked
                _ => {
                    // Fallback 2: XDG Desktop Portal (ScreenCast)
                    self.screenshot_via_portal(&path).await
                        .context("all screenshot methods failed (grim, extension, portal)")?;
                }
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
                "pipewiresrc", &format!("path={}", node_id),
                "!", "videoconvert",
                "!", "pngenc", "snapshot=true",
                "!", "filesink", &format!("location={}", output_path),
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
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/coemedia".to_string());
        let script = PathBuf::from(home).join("projects/deskbrid/scripts/screenshot_portal.py");
        if !script.exists() {
            anyhow::bail!("portal script not found: {}", script.display());
        }
        let output = Command::new("python3")
            .arg(&script)
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
