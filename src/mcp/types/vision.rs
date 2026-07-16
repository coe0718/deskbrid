//! MCP parameter types for vision operations: template matching, OCR, state detection.

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, schemars::JsonSchema, Default)]
pub struct VisionFindElementArgs {
    #[schemars(description = "Path to template image file (PNG)")]
    pub template_path: String,
    #[schemars(description = "Optional screenshot path — if omitted, captures live screen")]
    pub screenshot: Option<String>,
    #[schemars(description = "Minimum confidence threshold 0–1 (default 0.8)")]
    pub min_confidence: Option<f64>,
    #[schemars(description = "Maximum results to return (default 10)")]
    pub max_results: Option<u32>,
}

#[derive(Deserialize, Serialize, schemars::JsonSchema, Default)]
pub struct VisionFindByTextArgs {
    #[schemars(description = "Text to search for on screen (case-insensitive)")]
    pub text: String,
    #[schemars(description = "Optional screenshot path — if omitted, captures live screen")]
    pub screenshot: Option<String>,
}

#[derive(Deserialize, Serialize, schemars::JsonSchema, Default)]
pub struct VisionStateCheckArg {
    #[schemars(description = "Check kind: color_check, text_check, or element_check")]
    pub kind: String,
    #[schemars(description = "Expected value (hex colour, text string)")]
    pub expected: Option<serde_json::Value>,
    #[schemars(description = "Region x coordinate for color_check")]
    pub x: Option<i32>,
    #[schemars(description = "Region y coordinate for color_check")]
    pub y: Option<i32>,
    #[schemars(description = "Region width for color_check")]
    pub width: Option<i32>,
    #[schemars(description = "Region height for color_check")]
    pub height: Option<i32>,
    #[schemars(description = "Template path for element_check")]
    pub template_path: Option<String>,
    #[schemars(description = "Minimum confidence for element_check (default 0.8)")]
    pub min_confidence: Option<f64>,
}

#[derive(Deserialize, Serialize, schemars::JsonSchema, Default)]
pub struct VisionDetectStateArgs {
    #[schemars(description = "Optional screenshot path — if omitted, captures live screen")]
    pub screenshot: Option<String>,
    #[schemars(description = "List of state checks to perform")]
    pub checks: Vec<VisionStateCheckArg>,
}
