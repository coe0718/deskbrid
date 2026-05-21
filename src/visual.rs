use crate::backend::DesktopBackend;
use crate::daemon::expand_path;
use crate::protocol::Region;
use anyhow::Context;
use image::{GenericImageView, ImageBuffer, Rgba};
use serde_json::json;
use std::path::PathBuf;

type DiffImage = ImageBuffer<Rgba<u8>, Vec<u8>>;

pub struct ScreenshotDiffRequest<'a> {
    pub before_path: &'a str,
    pub after_path: Option<&'a str>,
    pub tolerance: u8,
    pub diff_path: Option<&'a str>,
    pub save_diff: bool,
    pub monitor: Option<u32>,
    pub region: Option<Region>,
    pub window_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct DiffStats {
    width: u32,
    height: u32,
    total_pixels: u64,
    changed_pixels: u64,
    bbox: Option<BoundingBox>,
}

#[derive(Debug, Clone, PartialEq)]
struct BoundingBox {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

pub async fn screenshot_diff(
    backend: &dyn DesktopBackend,
    request: ScreenshotDiffRequest<'_>,
) -> anyhow::Result<serde_json::Value> {
    let before_path = expand_path(request.before_path)?;
    let after_path = match request.after_path {
        Some(path) => expand_path(path)?,
        None => {
            let screenshot = backend
                .screenshot(request.monitor, request.region, request.window_id)
                .await?;
            PathBuf::from(screenshot.path)
        }
    };

    let tolerance = request.tolerance;
    let before_for_diff = before_path.clone();
    let after_for_diff = after_path.clone();
    let (stats, diff_image) = tokio::task::spawn_blocking(move || {
        let before = image::open(&before_for_diff)
            .with_context(|| format!("failed to open image {}", before_for_diff.display()))?;
        let after = image::open(&after_for_diff)
            .with_context(|| format!("failed to open image {}", after_for_diff.display()))?;
        diff_images(&before, &after, tolerance)
    })
    .await??;

    let diff_path = if request.save_diff || request.diff_path.is_some() {
        let path = match request.diff_path {
            Some(path) => expand_path(path)?,
            None => temp_diff_path(),
        };
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let save_path = path.clone();
        tokio::task::spawn_blocking(move || {
            diff_image
                .save(&save_path)
                .with_context(|| format!("failed to save diff image {}", save_path.display()))
        })
        .await??;
        Some(path)
    } else {
        None
    };

    let percent_changed = if stats.total_pixels == 0 {
        0.0
    } else {
        stats.changed_pixels as f64 / stats.total_pixels as f64 * 100.0
    };

    Ok(json!({
        "before_path": before_path.to_string_lossy(),
        "after_path": after_path.to_string_lossy(),
        "diff_path": diff_path.map(|p| p.to_string_lossy().to_string()),
        "width": stats.width,
        "height": stats.height,
        "total_pixels": stats.total_pixels,
        "changed_pixels": stats.changed_pixels,
        "percent_changed": (percent_changed * 1000.0).round() / 1000.0,
        "changed": stats.changed_pixels > 0,
        "tolerance": request.tolerance,
        "bounding_box": stats.bbox.map(|bbox| json!({
            "x": bbox.x,
            "y": bbox.y,
            "width": bbox.width,
            "height": bbox.height
        }))
    }))
}

fn diff_images(
    before: &image::DynamicImage,
    after: &image::DynamicImage,
    tolerance: u8,
) -> anyhow::Result<(DiffStats, DiffImage)> {
    if before.dimensions() != after.dimensions() {
        anyhow::bail!(
            "image dimensions differ: before={:?}, after={:?}",
            before.dimensions(),
            after.dimensions()
        );
    }
    let before = before.to_rgba8();
    let after = after.to_rgba8();
    let (width, height) = before.dimensions();
    let mut diff = ImageBuffer::from_pixel(width, height, Rgba([0, 0, 0, 0]));
    let mut changed_pixels = 0u64;
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0;
    let mut max_y = 0;

    for y in 0..height {
        for x in 0..width {
            let a = before.get_pixel(x, y).0;
            let b = after.get_pixel(x, y).0;
            if pixel_changed(a, b, tolerance) {
                changed_pixels += 1;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
                diff.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            }
        }
    }

    let bbox = (changed_pixels > 0).then_some(BoundingBox {
        x: min_x,
        y: min_y,
        width: max_x - min_x + 1,
        height: max_y - min_y + 1,
    });
    Ok((
        DiffStats {
            width,
            height,
            total_pixels: width as u64 * height as u64,
            changed_pixels,
            bbox,
        },
        diff,
    ))
}

fn pixel_changed(a: [u8; 4], b: [u8; 4], tolerance: u8) -> bool {
    a.iter()
        .zip(b.iter())
        .any(|(a, b)| a.abs_diff(*b) > tolerance)
}

fn temp_diff_path() -> PathBuf {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    PathBuf::from("/tmp/deskbrid").join(format!("diff_{ts}.png"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::DynamicImage;

    #[test]
    fn detects_changed_bounding_box() {
        let before = ImageBuffer::from_pixel(4, 4, Rgba([0, 0, 0, 255]));
        let mut after = ImageBuffer::from_pixel(4, 4, Rgba([0, 0, 0, 255]));
        after.put_pixel(1, 2, Rgba([255, 255, 255, 255]));
        after.put_pixel(2, 2, Rgba([255, 255, 255, 255]));

        let (stats, _) = diff_images(
            &DynamicImage::ImageRgba8(before),
            &DynamicImage::ImageRgba8(after),
            0,
        )
        .unwrap();

        assert_eq!(stats.changed_pixels, 2);
        assert_eq!(
            stats.bbox,
            Some(BoundingBox {
                x: 1,
                y: 2,
                width: 2,
                height: 1,
            })
        );
    }

    #[test]
    fn honors_tolerance() {
        assert!(!pixel_changed([10, 10, 10, 255], [12, 10, 10, 255], 2));
        assert!(pixel_changed([10, 10, 10, 255], [13, 10, 10, 255], 2));
    }
}
