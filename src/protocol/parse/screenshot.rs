use super::helpers::*;
use crate::protocol::Action;
use crate::protocol::types::Region;
use anyhow::Context;
use serde_json::Value;

pub(super) fn parse_screenshot(raw: &Value, _id: &str, type_str: &str) -> anyhow::Result<Action> {
    Ok(match type_str {
        // Screenshot
        "screenshot" => Action::Screenshot {
            monitor: raw["monitor"].as_u64().map(|v| v as u32),
            region: raw.get("region").and_then(|r| {
                Some(Region {
                    x: r["x"].as_u64()? as u32,
                    y: r["y"].as_u64()? as u32,
                    width: r["width"].as_u64()? as u32,
                    height: r["height"].as_u64()? as u32,
                })
            }),
            window_id: raw["window_id"].as_str().map(String::from),
            output: raw["output"].as_str().map(String::from),
        },
        "screenshot.ocr" => Action::ScreenshotOcr {
            path: optional_non_empty_string(raw, "path")?,
            language: optional_non_empty_string(raw, "language")?,
            psm: optional_u32(raw, "psm")?,
            bounding_boxes: raw["bounding_boxes"].as_bool().unwrap_or(false),
            monitor: raw["monitor"].as_u64().map(|v| v as u32),
            region: raw.get("region").and_then(|r| {
                Some(Region {
                    x: r["x"].as_u64()? as u32,
                    y: r["y"].as_u64()? as u32,
                    width: r["width"].as_u64()? as u32,
                    height: r["height"].as_u64()? as u32,
                })
            }),
            window_id: raw["window_id"].as_str().map(String::from),
        },
        "screenshot.diff" => Action::ScreenshotDiff {
            before_path: required_non_empty_string_alias(raw, "before_path", "before")?,
            after_path: optional_non_empty_string_alias(raw, "after_path", "after")?,
            tolerance: optional_u8(raw, "tolerance")?,
            diff_path: optional_non_empty_string(raw, "diff_path")?,
            save_diff: raw["save_diff"].as_bool().unwrap_or(false),
            monitor: raw["monitor"].as_u64().map(|v| v as u32),
            region: raw.get("region").and_then(|r| {
                Some(Region {
                    x: r["x"].as_u64()? as u32,
                    y: r["y"].as_u64()? as u32,
                    width: r["width"].as_u64()? as u32,
                    height: r["height"].as_u64()? as u32,
                })
            }),
            window_id: raw["window_id"].as_str().map(String::from),
        },
        "vision.find_element" => Action::VisionFindElement {
            template_path: required_non_empty_string(raw, "template_path")?,
            screenshot: optional_non_empty_string(raw, "screenshot")?,
            min_confidence: optional_f64(raw, "min_confidence")?,
            max_results: optional_u32(raw, "max_results")?,
        },
        "vision.find_by_text" => Action::VisionFindByText {
            text: required_non_empty_string(raw, "text")?,
            screenshot: optional_non_empty_string(raw, "screenshot")?,
        },
        "vision.detect_state" => Action::VisionDetectState {
            screenshot: optional_non_empty_string(raw, "screenshot")?,
            checks: raw["checks"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .cloned()
                        .map(serde_json::from_value)
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()
                .context("vision.detect_state contains an invalid check")?
                .unwrap_or_default(),
        },
        _ => anyhow::bail!("unknown screenshot type: {type_str}"),
    })
}
