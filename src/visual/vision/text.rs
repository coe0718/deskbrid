use serde_json::json;

pub(super) fn find_text_matches(
    words: &[crate::ocr::OcrWord],
    query: &str,
) -> Vec<serde_json::Value> {
    let query = query.trim().to_lowercase();
    let mut matches = Vec::new();

    for start in 0..words.len() {
        let first_word = words[start].text.to_lowercase();
        if !first_word.contains(&query) && !query.starts_with(&first_word) {
            continue;
        }

        let line_id = words[start].line_id;
        let mut text = String::new();
        let mut confidence_sum = 0.0;
        let mut right = words[start].x;
        let mut bottom = words[start].y;

        for word in words
            .iter()
            .skip(start)
            .take_while(|word| word.line_id == line_id)
        {
            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(&word.text);
            confidence_sum += word.confidence;
            right = right.max(word.x.saturating_add(word.width));
            bottom = bottom.max(word.y.saturating_add(word.height));

            let normalized = text.to_lowercase();
            if normalized.contains(&query) {
                let word_count = text.split_whitespace().count() as f64;
                matches.push(json!({
                    "text": text,
                    "x": words[start].x,
                    "y": words[start].y,
                    "width": right.saturating_sub(words[start].x),
                    "height": bottom.saturating_sub(words[start].y),
                    "confidence": ((confidence_sum / word_count / 100.0) * 1000.0).round() / 1000.0,
                }));
                break;
            }
            if normalized.len() > query.len().saturating_add(64) {
                break;
            }
        }
    }

    matches
}
