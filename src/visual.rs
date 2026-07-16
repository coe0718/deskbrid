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
pub(crate) struct DiffStats {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) total_pixels: u64,
    pub(crate) changed_pixels: u64,
    pub(crate) bbox: Option<BoundingBox>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BoundingBox {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

pub async fn screenshot_diff(
    backend: &dyn DesktopBackend,
    request: ScreenshotDiffRequest<'_>,
) -> anyhow::Result<serde_json::Value> {
    let before_path = expand_path(request.before_path).await?;
    let after_path = match request.after_path {
        Some(path) => expand_path(path).await?,
        None => {
            let screenshot = backend
                .screenshot(request.monitor, request.region, request.window_id)
                .await?;
            PathBuf::from(screenshot.path)
        }
    };

    let (stats, diff_image) =
        diff_image_paths_with_image(before_path.clone(), after_path.clone(), request.tolerance)
            .await?;

    let diff_path = if request.save_diff || request.diff_path.is_some() {
        let path = match request.diff_path {
            Some(path) => expand_path(path).await?,
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

pub(crate) async fn diff_image_paths(
    before_path: PathBuf,
    after_path: PathBuf,
    tolerance: u8,
) -> anyhow::Result<DiffStats> {
    let (stats, _) = diff_image_paths_with_image(before_path, after_path, tolerance).await?;
    Ok(stats)
}

async fn diff_image_paths_with_image(
    before_path: PathBuf,
    after_path: PathBuf,
    tolerance: u8,
) -> anyhow::Result<(DiffStats, DiffImage)> {
    tokio::task::spawn_blocking(move || {
        let before = image::open(&before_path)
            .with_context(|| format!("failed to open image {}", before_path.display()))?;
        let after = image::open(&after_path)
            .with_context(|| format!("failed to open image {}", after_path.display()))?;
        diff_images(&before, &after, tolerance)
    })
    .await?
}

pub(crate) fn diff_images(
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

// ── Template Matching ────────────────────────────────────────────

/// Result of a single template match.
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct TemplateMatchResult {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub confidence: f64,
}

/// Find matching positions for a template in an image using NCC.
///
/// Returns all positions above `min_confidence`, sorted by confidence descending,
/// up to `max_results`. The NCC value is mapped from [-1,1] to [0,1] range and
/// clamped so only positive correlations count.
fn template_match_ncc(
    image: &image::DynamicImage,
    template: &image::DynamicImage,
    min_confidence: f64,
    max_results: usize,
) -> Vec<TemplateMatchResult> {
    use imageproc::template_matching::{MatchTemplateMethod, match_template_parallel};

    let image_luma = image.to_luma8();
    let template_luma = template.to_luma8();
    let tw = template_luma.width();
    let th = template_luma.height();

    let result = match_template_parallel(
        &image_luma,
        &template_luma,
        MatchTemplateMethod::CrossCorrelationNormalized,
    );

    // NCC range is [-1, 1]. Map to [0, 1]: (ncc + 1) / 2
    let mut matches: Vec<TemplateMatchResult> = Vec::new();
    let threshold = ((min_confidence * 2.0) - 1.0).max(-1.0).min(1.0);

    for y in 0..result.height() {
        for x in 0..result.width() {
            let ncc = result.get_pixel(x, y).0[0] as f64;
            if ncc >= threshold {
                // Map [-1, 1] → [0, 1] so the API is intuitive
                let confidence = ((ncc + 1.0) / 2.0).clamp(0.0, 1.0);
                matches.push(TemplateMatchResult {
                    x,
                    y,
                    width: tw,
                    height: th,
                    confidence,
                });
            }
        }
    }

    matches.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
    matches.truncate(max_results);
    matches
}

// ── OCR via Tesseract ────────────────────────────────────────────

/// A single word recognised by OCR with bounding box and confidence.
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct OcrWord {
    pub text: String,
    pub left: u32,
    pub top: u32,
    pub width: u32,
    pub height: u32,
    pub confidence: f64,
}

/// Run OCR on an image file via tesseract CLI and return recognised words
/// with bounding boxes and confidence scores.
fn ocr_image_file(path: &std::path::Path) -> anyhow::Result<Vec<OcrWord>> {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};

    let mut child = Command::new("tesseract")
        .arg(path.to_str().unwrap())
        .arg("stdout")
        .arg("-l")
        .arg("eng")
        .arg("--tsv")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .context("failed to spawn tesseract — is tesseract-ocr installed?")?;

    let stdout = child.stdout.take().unwrap();
    let reader = BufReader::new(stdout);
    let mut words = Vec::new();

    for (i, line) in reader.lines().enumerate() {
        let line = line?;
        if i == 0 || line.is_empty() {
            continue; // skip header
        }
        // TSV columns: level, page_num, block_num, par_num, line_num, word_num,
        //              left, top, width, height, conf, text
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 12 {
            continue;
        }
        // Only word-level (level == 5) entries
        if cols[0].trim() != "5" {
            continue;
        }
        let text = cols[11].trim().to_string();
        if text.is_empty() {
            continue;
        }
        let left: u32 = cols[6].parse().unwrap_or(0);
        let top: u32 = cols[7].parse().unwrap_or(0);
        let width: u32 = cols[8].parse().unwrap_or(0);
        let height: u32 = cols[9].parse().unwrap_or(0);
        let raw_conf: f64 = cols[10].parse().unwrap_or(0.0);
        // Tesseract confidence is 0–100; normalise to 0–1
        let confidence = (raw_conf / 100.0).clamp(0.0, 1.0);

        words.push(OcrWord {
            text,
            left,
            top,
            width,
            height,
            confidence,
        });
    }

    let status = child.wait()?;
    if !status.success() {
        anyhow::bail!("tesseract exited with status {:?}", status.code());
    }

    Ok(words)
}

/// Run OCR on a DynamicImage by writing to a temp file first.
fn ocr_image_dynamic(image: &image::DynamicImage) -> anyhow::Result<Vec<OcrWord>> {
    let dir = tempfile::TempDir::new()?;
    let png_path = dir.path().join("_ocr_.png");
    image.save(&png_path)?;
    ocr_image_file(&png_path)
}

// ── Vision Action Implementations ─────────────────────────────────

/// Vision element detection request types
pub struct VisionFindElementRequest {
    pub template_path: String,
    pub screenshot: Option<String>,
    pub min_confidence: Option<f64>,
    pub max_results: Option<u32>,
}

pub struct VisionFindByTextRequest {
    pub text: String,
    pub screenshot: Option<String>,
}

pub struct VisionDetectStateRequest {
    pub screenshot: Option<String>,
    pub checks: Vec<crate::protocol::VisionStateCheck>,
}

/// Find UI element(s) by visual template matching.
pub async fn vision_find_element(
    backend: &dyn DesktopBackend,
    request: VisionFindElementRequest,
) -> anyhow::Result<serde_json::Value> {
    let template_path = expand_path(&request.template_path).await?;
    let screenshot_path = match request.screenshot {
        Some(s) => expand_path(&s).await?,
        None => {
            let screenshot = backend.screenshot(None, None, None).await?;
            PathBuf::from(screenshot.path)
        }
    };

    let min_confidence = request.min_confidence.unwrap_or(0.8);
    let max_results = request.max_results.unwrap_or(10) as usize;

    let template_path_clone = template_path.clone();
    let screenshot_path_clone = screenshot_path.clone();

    let matches =
        tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<TemplateMatchResult>> {
            let template = image::open(&template_path_clone).with_context(|| {
                format!("failed to open template {}", template_path_clone.display())
            })?;
            let screenshot = image::open(&screenshot_path_clone).with_context(|| {
                format!(
                    "failed to open screenshot {}",
                    screenshot_path_clone.display()
                )
            })?;

            if template.width() > screenshot.width() || template.height() > screenshot.height() {
                return Ok(Vec::new());
            }

            Ok(template_match_ncc(
                &screenshot,
                &template,
                min_confidence,
                max_results,
            ))
        })
        .await??;

    let elements: Vec<serde_json::Value> = matches
        .iter()
        .map(|m| {
            json!({
                "x": m.x,
                "y": m.y,
                "width": m.width,
                "height": m.height,
                "confidence": (m.confidence * 1000.0).round() / 1000.0,
            })
        })
        .collect();

    Ok(json!({
        "elements": elements,
        "count": elements.len(),
        "template_path": template_path.to_string_lossy(),
        "screenshot": screenshot_path.to_string_lossy(),
        "min_confidence": min_confidence,
        "max_results": max_results,
    }))
}

/// Find element by text label (hybrid OCR + position).
pub async fn vision_find_by_text(
    backend: &dyn DesktopBackend,
    request: VisionFindByTextRequest,
) -> anyhow::Result<serde_json::Value> {
    let screenshot_path = match &request.screenshot {
        Some(s) => expand_path(s).await?,
        None => {
            let s = backend.screenshot(None, None, None).await?;
            PathBuf::from(s.path)
        }
    };

    let screenshot = match &request.screenshot {
        Some(_) => image::open(&screenshot_path)
            .with_context(|| format!("failed to open screenshot {}", screenshot_path.display()))?,
        None => image::open(&screenshot_path)
            .with_context(|| format!("failed to open screenshot {}", screenshot_path.display()))?,
    };

    let query = request.text.clone().to_lowercase();
    let words = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<OcrWord>> {
        ocr_image_dynamic(&screenshot)
    })
    .await??;

    // Find words matching the query text (case-insensitive substring match)
    let matching: Vec<serde_json::Value> = words
        .into_iter()
        .filter(|w| w.text.to_lowercase().contains(&query))
        .map(|w| {
            json!({
                "text": w.text,
                "left": w.left,
                "top": w.top,
                "width": w.width,
                "height": w.height,
                "confidence": (w.confidence * 1000.0).round() / 1000.0,
            })
        })
        .collect();

    Ok(json!({
        "elements": matching,
        "count": matching.len(),
        "query": request.text,
        "screenshot": screenshot_path.to_string_lossy(),
    }))
}

/// Detect UI state via multiple visual checks.
pub async fn vision_detect_state(
    backend: &dyn DesktopBackend,
    request: VisionDetectStateRequest,
) -> anyhow::Result<serde_json::Value> {
    use crate::protocol::VisionStateCheck;

    let screenshot = match &request.screenshot {
        Some(s) => {
            let path = expand_path(s).await?;
            image::open(&path)
                .with_context(|| format!("failed to open screenshot {}", path.display()))?
        }
        None => {
            let s = backend.screenshot(None, None, None).await?;
            let path = PathBuf::from(s.path);
            image::open(&path)
                .with_context(|| format!("failed to open screenshot {}", path.display()))?
        }
    };

    let screenshot_path = match &request.screenshot {
        Some(s) => expand_path(s).await?,
        None => {
            let s = backend.screenshot(None, None, None).await?;
            PathBuf::from(s.path)
        }
    };

    let screenshot_arc = std::sync::Arc::new(screenshot);

    // Run checks — independent work, but we run them sequentially in spawn_blocking
    // for simplicity (OCR already dominates the cost).
    let results = tokio::task::spawn_blocking({
        let img = screenshot_arc.clone();
        move || -> anyhow::Result<Vec<serde_json::Value>> {
            let mut results = Vec::new();
            for check in &request.checks {
                let result = run_state_check(&img, check)?;
                results.push(result);
            }
            Ok(results)
        }
    })
    .await??;

    Ok(json!({
        "screenshot": screenshot_path.to_string_lossy(),
        "results": results,
    }))
}

/// Run a single vision state check.
fn run_state_check(
    image: &image::DynamicImage,
    check: &crate::protocol::VisionStateCheck,
) -> anyhow::Result<serde_json::Value> {
    match check.kind.as_str() {
        "color_check" => {
            // Check if a region matches an expected colour.
            // expected format: { "r": u8, "g": u8, "b": u8, "a": u8 }
            // or hex string "#RRGGBB" / "#RRGGBBAA"
            let region = check
                .region
                .as_ref()
                .context("color_check requires a region")?;
            let expected = check
                .expected
                .as_ref()
                .context("color_check requires expected colour")?;

            let expected_rgba = parse_colour_value(expected)?;
            let pixel = image.get_pixel(region.x as u32, region.y as u32);
            let matches = pixel.0 == expected_rgba;

            Ok(json!({
                "kind": "color_check",
                "passed": matches,
                "region": {
                    "x": region.x, "y": region.y,
                    "width": region.width, "height": region.height,
                },
                "expected": format!("#{:02X}{:02X}{:02X}{:02X}", expected_rgba[0], expected_rgba[1], expected_rgba[2], expected_rgba[3]),
                "actual": format!("#{:02X}{:02X}{:02X}{:02X}", pixel.0[0], pixel.0[1], pixel.0[2], pixel.0[3]),
            }))
        }
        "text_check" => {
            // Check if specified text is present or absent.
            let expected_text = check
                .expected
                .as_ref()
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();
            let words = ocr_image_dynamic(image)?;
            let found = words
                .iter()
                .any(|w| w.text.to_lowercase().contains(&expected_text));

            // If no explicit expected value, presence is the pass condition
            let passed = check.expected.is_none() || found;

            Ok(json!({
                "kind": "text_check",
                "passed": passed,
                "text_found": found,
                "query": expected_text,
            }))
        }
        "element_check" => {
            // Check if a template element exists on screen.
            let template_path = check
                .template_path
                .as_ref()
                .context("element_check requires template_path")?;
            let template = image::open(template_path)
                .with_context(|| format!("failed to open template {}", template_path))?;

            let min_confidence = check.min_confidence.unwrap_or(0.8);
            let matches = template_match_ncc(image, &template, min_confidence, 1);

            Ok(json!({
                "kind": "element_check",
                "passed": !matches.is_empty(),
                "found": matches.is_empty(),
                "matches": matches.len(),
                "best_confidence": matches.first().map(|m| (m.confidence * 1000.0).round() / 1000.0),
            }))
        }
        other => Ok(json!({
            "kind": other,
            "passed": false,
            "note": format!("unknown check kind: {other}"),
        })),
    }
}

/// Parse a colour value from JSON — supports hex strings and {r,g,b,a} objects.
fn parse_colour_value(value: &serde_json::Value) -> anyhow::Result<[u8; 4]> {
    if let Some(s) = value.as_str() {
        let s = s.trim_start_matches('#');
        match s.len() {
            6 => {
                let r = u8::from_str_radix(&s[0..2], 16)?;
                let g = u8::from_str_radix(&s[2..4], 16)?;
                let b = u8::from_str_radix(&s[4..6], 16)?;
                Ok([r, g, b, 255])
            }
            8 => {
                let r = u8::from_str_radix(&s[0..2], 16)?;
                let g = u8::from_str_radix(&s[2..4], 16)?;
                let b = u8::from_str_radix(&s[4..6], 16)?;
                let a = u8::from_str_radix(&s[6..8], 16)?;
                Ok([r, g, b, a])
            }
            _ => anyhow::bail!("invalid hex colour: #{s} (expected 6 or 8 hex digits)"),
        }
    } else if let Some(obj) = value.as_object() {
        let r = obj.get("r").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
        let g = obj.get("g").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
        let b = obj.get("b").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
        let a = obj.get("a").and_then(|v| v.as_u64()).unwrap_or(255) as u8;
        Ok([r, g, b, a])
    } else {
        anyhow::bail!("invalid colour value — expected hex string or {{r,g,b,a}} object")
    }
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
