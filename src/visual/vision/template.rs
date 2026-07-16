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

/// Find matching positions for a template using normalized cross-correlation.
///
/// Large searches use a coarse image pyramid and then refine candidates against
/// the original pixels. `imageproc`'s matcher is O(screen pixels × template
/// pixels); running it directly on a 4K screenshot can otherwise tie up every
/// CPU core for minutes.
pub(super) fn template_match_ncc(
    image: &image::DynamicImage,
    template: &image::DynamicImage,
    min_confidence: f64,
    max_results: usize,
) -> Vec<TemplateMatchResult> {
    if max_results == 0
        || template.width() == 0
        || template.height() == 0
        || template.width() > image.width()
        || template.height() > image.height()
    {
        return Vec::new();
    }

    let scale = template_search_scale(image, template);
    if scale >= 0.999 {
        return suppress_overlaps(
            template_match_candidates(image, template, min_confidence, max_results * 50),
            max_results,
        );
    }

    let scaled_width = ((image.width() as f64 * scale).round() as u32).max(1);
    let scaled_height = ((image.height() as f64 * scale).round() as u32).max(1);
    let scaled_template_width =
        ((template.width() as f64 * scale).round() as u32).clamp(1, scaled_width);
    let scaled_template_height =
        ((template.height() as f64 * scale).round() as u32).clamp(1, scaled_height);
    let scaled_image = image.resize_exact(
        scaled_width,
        scaled_height,
        image::imageops::FilterType::Triangle,
    );
    let scaled_template = template.resize_exact(
        scaled_template_width,
        scaled_template_height,
        image::imageops::FilterType::Triangle,
    );
    let coarse_threshold = (min_confidence - 0.15).max(0.0);
    let coarse = template_match_candidates(
        &scaled_image,
        &scaled_template,
        coarse_threshold,
        max_results * 25,
    );

    let radius = (2.0 / scale).ceil() as u32 + 1;
    let mut refined = Vec::new();
    for candidate in coarse {
        let estimated_x = (candidate.x as f64 / scale).round() as u32;
        let estimated_y = (candidate.y as f64 / scale).round() as u32;
        let left = estimated_x.saturating_sub(radius);
        let top = estimated_y.saturating_sub(radius);
        let right = estimated_x
            .saturating_add(template.width())
            .saturating_add(radius)
            .min(image.width());
        let bottom = estimated_y
            .saturating_add(template.height())
            .saturating_add(radius)
            .min(image.height());
        if right.saturating_sub(left) < template.width()
            || bottom.saturating_sub(top) < template.height()
        {
            continue;
        }

        let search_region = image.crop_imm(left, top, right - left, bottom - top);
        for mut exact in template_match_candidates(&search_region, template, min_confidence, 3) {
            exact.x += left;
            exact.y += top;
            refined.push(exact);
        }
    }

    suppress_overlaps(refined, max_results)
}

fn template_search_scale(image: &image::DynamicImage, template: &image::DynamicImage) -> f64 {
    const MAX_SEARCH_DIMENSION: f64 = 512.0;
    const TARGET_TEMPLATE_PIXELS: f64 = 512.0;

    let max_dimension = image.width().max(image.height()) as f64;
    let dimension_scale = (MAX_SEARCH_DIMENSION / max_dimension).min(1.0);
    let template_pixels = u64::from(template.width()) * u64::from(template.height());
    let work_scale = (TARGET_TEMPLATE_PIXELS / template_pixels as f64)
        .sqrt()
        .min(1.0);
    let min_template_dimension = template.width().min(template.height()) as f64;
    let legibility_scale = (2.0 / min_template_dimension).min(1.0);

    dimension_scale.min(work_scale).max(legibility_scale)
}

fn template_match_candidates(
    image: &image::DynamicImage,
    template: &image::DynamicImage,
    min_confidence: f64,
    candidate_limit: usize,
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

    let mut candidates = Vec::new();
    for y in 0..result.height() {
        for x in 0..result.width() {
            let confidence = result.get_pixel(x, y).0[0] as f64;
            if confidence.is_finite() && confidence >= min_confidence {
                candidates.push(TemplateMatchResult {
                    x,
                    y,
                    width: tw,
                    height: th,
                    confidence,
                });
            }
        }
        if candidates.len() > candidate_limit.saturating_mul(2) {
            candidates.sort_by(|a, b| b.confidence.total_cmp(&a.confidence));
            candidates.truncate(candidate_limit);
        }
    }

    candidates.sort_by(|a, b| b.confidence.total_cmp(&a.confidence));
    candidates.truncate(candidate_limit);
    candidates
}

fn suppress_overlaps(
    mut candidates: Vec<TemplateMatchResult>,
    max_results: usize,
) -> Vec<TemplateMatchResult> {
    candidates.sort_by(|a, b| b.confidence.total_cmp(&a.confidence));
    let mut matches = Vec::with_capacity(max_results.min(candidates.len()));
    for candidate in candidates {
        if matches
            .iter()
            .all(|existing| !overlaps_substantially(&candidate, existing))
        {
            matches.push(candidate);
            if matches.len() == max_results {
                break;
            }
        }
    }
    matches
}

fn overlaps_substantially(a: &TemplateMatchResult, b: &TemplateMatchResult) -> bool {
    let left = a.x.max(b.x);
    let top = a.y.max(b.y);
    let right = a.x.saturating_add(a.width).min(b.x.saturating_add(b.width));
    let bottom =
        a.y.saturating_add(a.height)
            .min(b.y.saturating_add(b.height));
    if right <= left || bottom <= top {
        return false;
    }

    let intersection = u64::from(right - left) * u64::from(bottom - top);
    let union = u64::from(a.width) * u64::from(a.height) + u64::from(b.width) * u64::from(b.height)
        - intersection;
    intersection as f64 / union as f64 > 0.3
}
