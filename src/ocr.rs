use crate::backend::DesktopBackend;
use crate::daemon::expand_path;
use crate::protocol::Region;
use anyhow::Context;
use serde_json::json;
use std::path::PathBuf;
use tokio::process::Command;

#[derive(Debug)]
struct OcrWord {
    text: String,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    confidence: f64,
}

pub struct OcrRequest<'a> {
    pub path: Option<&'a str>,
    pub language: Option<&'a str>,
    pub psm: Option<u32>,
    pub bounding_boxes: bool,
    pub monitor: Option<u32>,
    pub region: Option<Region>,
    pub window_id: Option<String>,
}

pub async fn screenshot_ocr(
    backend: &dyn DesktopBackend,
    request: OcrRequest<'_>,
) -> anyhow::Result<serde_json::Value> {
    let source_path = match request.path {
        Some(path) => expand_path(path)?,
        None => {
            let screenshot = backend
                .screenshot(request.monitor, request.region, request.window_id)
                .await?;
            PathBuf::from(screenshot.path)
        }
    };
    if tokio::fs::metadata(&source_path).await.is_err() {
        anyhow::bail!("screenshot path does not exist: {}", source_path.display());
    }

    let language = request.language.unwrap_or("eng");
    validate_language(language)?;
    let psm = request.psm.unwrap_or(3);
    if psm > 13 {
        anyhow::bail!("psm must be between 0 and 13");
    }

    let output = Command::new("tesseract")
        .arg(&source_path)
        .arg("stdout")
        .arg("-l")
        .arg(language)
        .arg("--psm")
        .arg(psm.to_string())
        .arg("tsv")
        .output()
        .await
        .context("failed to run tesseract; install tesseract-ocr and language packs")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("tesseract failed: {}", stderr.trim());
    }

    let tsv = String::from_utf8_lossy(&output.stdout);
    let words = parse_tsv(&tsv);
    let text = words
        .iter()
        .map(|word| word.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let confidence = mean_confidence(&words);
    let word_values = if request.bounding_boxes {
        words
            .iter()
            .map(|word| {
                json!({
                    "text": word.text,
                    "x": word.x,
                    "y": word.y,
                    "width": word.width,
                    "height": word.height,
                    "confidence": word.confidence
                })
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    Ok(json!({
        "text": text,
        "confidence": confidence,
        "words": word_values,
        "source_path": source_path.to_string_lossy(),
        "language": language,
        "psm": psm
    }))
}

fn parse_tsv(tsv: &str) -> Vec<OcrWord> {
    tsv.lines().skip(1).filter_map(parse_tsv_line).collect()
}

fn parse_tsv_line(line: &str) -> Option<OcrWord> {
    let columns: Vec<&str> = line.split('\t').collect();
    if columns.len() < 12 || columns.first().copied()? != "5" {
        return None;
    }
    let text = columns[11..].join("\t").trim().to_string();
    if text.is_empty() {
        return None;
    }
    let confidence = columns.get(10)?.parse::<f64>().ok()?;
    if confidence < 0.0 {
        return None;
    }
    Some(OcrWord {
        text,
        x: parse_u32(columns.get(6)?)?,
        y: parse_u32(columns.get(7)?)?,
        width: parse_u32(columns.get(8)?)?,
        height: parse_u32(columns.get(9)?)?,
        confidence,
    })
}

fn parse_u32(value: &str) -> Option<u32> {
    value.parse::<u32>().ok()
}

fn mean_confidence(words: &[OcrWord]) -> f64 {
    if words.is_empty() {
        return 0.0;
    }
    let sum: f64 = words.iter().map(|word| word.confidence).sum();
    (sum / words.len() as f64 * 10.0).round() / 10.0
}

fn validate_language(language: &str) -> anyhow::Result<()> {
    if language.trim().is_empty()
        || !language
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '+' || c == '-')
    {
        anyhow::bail!("language must contain only letters, numbers, '_', '-' or '+'");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tesseract_tsv_words() {
        let tsv = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n\
5\t1\t1\t1\t1\t1\t10\t20\t30\t15\t95.5\tHello\n\
5\t1\t1\t1\t1\t2\t44\t20\t35\t15\t90.0\tworld\n";
        let words = parse_tsv(tsv);
        assert_eq!(words.len(), 2);
        assert_eq!(words[0].text, "Hello");
        assert_eq!(mean_confidence(&words), 92.8);
    }

    #[test]
    fn rejects_suspicious_language_names() {
        assert!(validate_language("eng").is_ok());
        assert!(validate_language("eng+spa").is_ok());
        assert!(validate_language("../eng").is_err());
    }
}
