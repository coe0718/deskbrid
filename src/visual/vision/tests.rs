use super::state::{parse_colour_value, run_state_check};
use super::template::template_match_ncc;
use super::text::find_text_matches;
use crate::ocr::OcrWord;
use image::{DynamicImage, ImageBuffer, Rgba};
use serde_json::json;

fn ocr_word(text: &str, x: u32, width: u32, confidence: f64) -> OcrWord {
    OcrWord {
        text: text.into(),
        x,
        y: 20,
        width,
        height: 15,
        confidence,
        line_id: [1, 1, 1, 1],
    }
}

#[test]
fn template_threshold_uses_normalized_score_directly() {
    let image = DynamicImage::ImageRgba8(ImageBuffer::from_pixel(4, 4, Rgba([0, 0, 0, 255])));
    let template =
        DynamicImage::ImageRgba8(ImageBuffer::from_pixel(2, 2, Rgba([255, 255, 255, 255])));

    let matches = template_match_ncc(&image, &template, 0.4, 5);

    assert!(matches.is_empty(), "zero correlation must not pass 0.4");
}

#[test]
fn scaled_template_search_refines_exact_coordinates() {
    let mut image = ImageBuffer::from_pixel(800, 600, Rgba([20_u8, 20, 20, 255]));
    let template = ImageBuffer::from_fn(32, 32, |x, y| {
        Rgba([
            ((x * 17 + y * 5) % 251) as u8,
            ((x * 3 + y * 19) % 253) as u8,
            ((x * 11 + y * 7) % 249) as u8,
            255,
        ])
    });
    image::imageops::replace(&mut image, &template, 350, 280);

    let matches = template_match_ncc(
        &DynamicImage::ImageRgba8(image),
        &DynamicImage::ImageRgba8(template),
        0.99,
        5,
    );

    assert_eq!(matches.len(), 1);
    assert_eq!((matches[0].x, matches[0].y), (350, 280));
}

#[test]
fn finds_phrase_as_one_box_without_duplicate_single_word_match() {
    let words = vec![
        ocr_word("Hello", 10, 30, 95.0),
        ocr_word("World", 45, 35, 90.0),
    ];

    let phrase = find_text_matches(&words, "hello world");
    let word = find_text_matches(&words, "world");

    assert_eq!(phrase.len(), 1);
    assert_eq!(phrase[0]["x"], 10);
    assert_eq!(phrase[0]["width"], 70);
    assert_eq!(word.len(), 1);
    assert_eq!(word[0]["x"], 45);
}

#[test]
fn equal_sized_template_matches_once() {
    let image = DynamicImage::ImageRgba8(ImageBuffer::from_fn(3, 3, |x, y| {
        Rgba([(x * 70) as u8, (y * 70) as u8, ((x + y) * 30) as u8, 255])
    }));

    let matches = template_match_ncc(&image, &image, 0.99, 5);

    assert_eq!(matches.len(), 1);
    assert_eq!((matches[0].x, matches[0].y), (0, 0));
    assert!((matches[0].confidence - 1.0).abs() < f64::EPSILON);
}

#[test]
fn element_check_reports_found_true_for_match() {
    let mut image = ImageBuffer::from_pixel(6, 6, Rgba([0, 0, 0, 255]));
    for y in 2..4 {
        for x in 2..4 {
            image.put_pixel(x, y, Rgba([255, 255, 255, 255]));
        }
    }
    let template = ImageBuffer::from_pixel(2, 2, Rgba([255_u8, 255, 255, 255]));
    let temp = tempfile::tempdir().unwrap();
    let template_path = temp.path().join("template.png");
    template.save(&template_path).unwrap();
    let check = crate::protocol::VisionStateCheck {
        kind: "element_check".into(),
        expected: None,
        region: None,
        template_path: Some(template_path.to_string_lossy().into_owned()),
        min_confidence: Some(0.99),
    };

    let result = run_state_check(&DynamicImage::ImageRgba8(image), &check, None).unwrap();

    assert_eq!(result["passed"], true);
    assert_eq!(result["found"], true);
}

#[test]
fn color_check_requires_entire_region_to_match() {
    let mut image = ImageBuffer::from_pixel(2, 1, Rgba([255, 0, 0, 255]));
    image.put_pixel(1, 0, Rgba([0, 0, 255, 255]));
    let check = crate::protocol::VisionStateCheck {
        kind: "color_check".into(),
        expected: Some(json!("#FF0000")),
        region: Some(crate::protocol::Region {
            x: 0,
            y: 0,
            width: 2,
            height: 1,
        }),
        template_path: None,
        min_confidence: None,
    };

    let result = run_state_check(&DynamicImage::ImageRgba8(image), &check, None).unwrap();

    assert_eq!(result["passed"], false);
}

#[test]
fn color_check_rejects_out_of_bounds_region() {
    let image = DynamicImage::ImageRgba8(ImageBuffer::from_pixel(2, 2, Rgba([255, 0, 0, 255])));
    let check = crate::protocol::VisionStateCheck {
        kind: "color_check".into(),
        expected: Some(json!("#FF0000")),
        region: Some(crate::protocol::Region {
            x: 2,
            y: 0,
            width: 1,
            height: 1,
        }),
        template_path: None,
        min_confidence: None,
    };

    assert!(run_state_check(&image, &check, None).is_err());
}

#[test]
fn color_object_rejects_channels_over_255() {
    assert!(parse_colour_value(&json!({"r": 256, "g": 0, "b": 0})).is_err());
}
