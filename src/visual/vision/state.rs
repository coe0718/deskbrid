use super::template::template_match_ncc;
use super::text::find_text_matches;
use super::validate_confidence;
use anyhow::Context;
use image::GenericImageView;
use serde_json::json;

pub(super) fn run_state_check(
    image: &image::DynamicImage,
    check: &crate::protocol::VisionStateCheck,
    ocr_words: Option<&[crate::ocr::OcrWord]>,
) -> anyhow::Result<serde_json::Value> {
    match check.kind.as_str() {
        "color_check" => {
            let region = check
                .region
                .as_ref()
                .context("color_check requires a region")?;
            let expected = check
                .expected
                .as_ref()
                .context("color_check requires expected colour")?;
            let expected_rgba = parse_colour_value(expected)?;
            let end_x = region
                .x
                .checked_add(region.width)
                .context("color_check region x overflow")?;
            let end_y = region
                .y
                .checked_add(region.height)
                .context("color_check region y overflow")?;
            if region.width == 0 || region.height == 0 {
                anyhow::bail!("color_check region width and height must be greater than zero");
            }
            if end_x > image.width() || end_y > image.height() {
                anyhow::bail!(
                    "color_check region ({}, {}, {}, {}) exceeds image bounds {}x{}",
                    region.x,
                    region.y,
                    region.width,
                    region.height,
                    image.width(),
                    image.height()
                );
            }

            let mut matching_pixels = 0_u64;
            let mut sums = [0_u64; 4];
            for y in region.y..end_y {
                for x in region.x..end_x {
                    let pixel = image.get_pixel(x, y).0;
                    if pixel == expected_rgba {
                        matching_pixels += 1;
                    }
                    for (sum, channel) in sums.iter_mut().zip(pixel) {
                        *sum += u64::from(channel);
                    }
                }
            }
            let total_pixels = u64::from(region.width) * u64::from(region.height);
            let average = sums.map(|sum| (sum / total_pixels) as u8);
            let passed = matching_pixels == total_pixels;

            Ok(json!({
                "kind": "color_check",
                "passed": passed,
                "region": region,
                "expected": format_colour(expected_rgba),
                "actual_average": format_colour(average),
                "matching_pixels": matching_pixels,
                "total_pixels": total_pixels,
            }))
        }
        "text_check" => {
            let expected_text = check
                .expected
                .as_ref()
                .and_then(|value| value.as_str())
                .context("text_check requires expected text")?;
            let words = ocr_words.context("text_check requires OCR results")?;
            let matches = find_text_matches(words, expected_text);
            let found = !matches.is_empty();

            Ok(json!({
                "kind": "text_check",
                "passed": found,
                "text_found": found,
                "query": expected_text,
                "matches": matches,
            }))
        }
        "element_check" => {
            let template_path = check
                .template_path
                .as_ref()
                .context("element_check requires template_path")?;
            let template = image::open(template_path)
                .with_context(|| format!("failed to open template {template_path}"))?;
            let min_confidence = validate_confidence(check.min_confidence.unwrap_or(0.8))?;
            let matches = template_match_ncc(image, &template, min_confidence, 1);
            let found = !matches.is_empty();

            Ok(json!({
                "kind": "element_check",
                "passed": found,
                "found": found,
                "matches": matches.len(),
                "best_confidence": matches.first().map(|m| (m.confidence * 1000.0).round() / 1000.0),
            }))
        }
        other => anyhow::bail!("unknown vision state check kind: {other}"),
    }
}

fn format_colour(rgba: [u8; 4]) -> String {
    format!(
        "#{:02X}{:02X}{:02X}{:02X}",
        rgba[0], rgba[1], rgba[2], rgba[3]
    )
}

/// Parse a colour value from JSON — supports hex strings and {r,g,b,a} objects.
pub(super) fn parse_colour_value(value: &serde_json::Value) -> anyhow::Result<[u8; 4]> {
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
        let r = parse_colour_channel(obj, "r", None)?;
        let g = parse_colour_channel(obj, "g", None)?;
        let b = parse_colour_channel(obj, "b", None)?;
        let a = parse_colour_channel(obj, "a", Some(255))?;
        Ok([r, g, b, a])
    } else {
        anyhow::bail!("invalid colour value — expected hex string or {{r,g,b,a}} object")
    }
}

fn parse_colour_channel(
    object: &serde_json::Map<String, serde_json::Value>,
    channel: &str,
    default: Option<u8>,
) -> anyhow::Result<u8> {
    let Some(value) = object.get(channel) else {
        return default.context(format!("colour object requires '{channel}'"));
    };
    let value = value
        .as_u64()
        .with_context(|| format!("colour channel '{channel}' must be an integer"))?;
    u8::try_from(value).with_context(|| format!("colour channel '{channel}' must be 0..=255"))
}
