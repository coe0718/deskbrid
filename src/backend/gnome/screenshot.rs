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
            .as_nanos();
        let path = format!("/tmp/deskbrid_screenshot_{}.png", ts);

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

        // If grim failed (GNOME Wayland — no wlr-screencopy), use our GNOME extension
        if !grim_ok {
            self.screenshot_via_extension(&path).await?;
        }

        let dims = get_png_dimensions(&path)?;
        Ok(protocol::ScreenshotResult {
            path,
            width: dims.0,
            height: dims.1,
            format: "png".into(),
        })
    }

    /// Take a screenshot via the deskbrid GNOME Shell extension.
    /// The extension runs inside GNOME Shell and has access to its screenshot API.
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
