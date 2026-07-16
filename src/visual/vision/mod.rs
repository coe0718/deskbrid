use crate::backend::DesktopBackend;
use crate::daemon::expand_path;

use anyhow::Context;
use serde_json::json;
use std::path::PathBuf;

mod state;
mod template;
mod text;

use state::run_state_check;
use template::{TemplateMatchResult, template_match_ncc};
use text::find_text_matches;

fn validate_confidence(confidence: f64) -> anyhow::Result<f64> {
    if !confidence.is_finite() || !(0.0..=1.0).contains(&confidence) {
        anyhow::bail!("min_confidence must be a finite number between 0 and 1");
    }
    Ok(confidence)
}

async fn resolve_vision_screenshot(
    backend: Option<&dyn DesktopBackend>,
    screenshot: Option<&str>,
) -> anyhow::Result<PathBuf> {
    if let Some(path) = screenshot {
        return expand_path(path).await;
    }

    let backend = backend
        .context("no desktop backend loaded; provide a screenshot path when running headless")?;
    let captured = backend.screenshot(None, None, None).await?;
    Ok(PathBuf::from(captured.path))
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
    backend: Option<&dyn DesktopBackend>,
    request: VisionFindElementRequest,
) -> anyhow::Result<serde_json::Value> {
    let template_path = expand_path(&request.template_path).await?;
    let screenshot_path = resolve_vision_screenshot(backend, request.screenshot.as_deref()).await?;

    let min_confidence = validate_confidence(request.min_confidence.unwrap_or(0.8))?;
    let max_results = request.max_results.unwrap_or(5);
    if !(1..=100).contains(&max_results) {
        anyhow::bail!("max_results must be between 1 and 100");
    }
    let max_results = max_results as usize;

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

/// Find an element by OCR text, including phrases that span multiple words.
pub async fn vision_find_by_text(
    backend: Option<&dyn DesktopBackend>,
    request: VisionFindByTextRequest,
) -> anyhow::Result<serde_json::Value> {
    let query = request.text.trim();
    if query.is_empty() {
        anyhow::bail!("text must not be empty");
    }

    let screenshot_path = resolve_vision_screenshot(backend, request.screenshot.as_deref()).await?;
    let words = crate::ocr::ocr_words_from_path(&screenshot_path, "eng", 3).await?;
    let matching = find_text_matches(&words, query);

    Ok(json!({
        "elements": matching,
        "count": matching.len(),
        "query": request.text,
        "screenshot": screenshot_path.to_string_lossy(),
    }))
}

/// Detect UI state via multiple visual checks.
pub async fn vision_detect_state(
    backend: Option<&dyn DesktopBackend>,
    request: VisionDetectStateRequest,
) -> anyhow::Result<serde_json::Value> {
    let screenshot_path = resolve_vision_screenshot(backend, request.screenshot.as_deref()).await?;

    let mut checks = request.checks;
    for check in &mut checks {
        match check.kind.as_str() {
            "element_check" => {
                let template = check
                    .template_path
                    .as_ref()
                    .context("element_check requires template_path")?;
                check.template_path = Some(expand_path(template).await?.to_string_lossy().into());
                validate_confidence(check.min_confidence.unwrap_or(0.8))?;
            }
            "text_check" => {
                let text = check
                    .expected
                    .as_ref()
                    .and_then(|value| value.as_str())
                    .context("text_check requires expected text")?;
                if text.trim().is_empty() {
                    anyhow::bail!("text_check expected text must not be empty");
                }
            }
            "color_check" => {}
            other => anyhow::bail!("unknown vision state check kind: {other}"),
        }
    }

    let ocr_words = if checks.iter().any(|check| check.kind == "text_check") {
        Some(crate::ocr::ocr_words_from_path(&screenshot_path, "eng", 3).await?)
    } else {
        None
    };
    let image_path = screenshot_path.clone();
    let results = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<serde_json::Value>> {
        let image = image::open(&image_path)
            .with_context(|| format!("failed to open screenshot {}", image_path.display()))?;
        checks
            .iter()
            .map(|check| run_state_check(&image, check, ocr_words.as_deref()))
            .collect()
    })
    .await??;
    let passed = results
        .iter()
        .all(|result| result["passed"].as_bool().unwrap_or(false));

    Ok(json!({
        "screenshot": screenshot_path.to_string_lossy(),
        "passed": passed,
        "results": results,
    }))
}

#[cfg(test)]
mod tests;
