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
    #[schemars(description = "Maximum results to return (default 5)")]
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
pub struct VisionDetectStateArgs {
    #[schemars(description = "Optional screenshot path — if omitted, captures live screen")]
    pub screenshot: Option<String>,
    #[schemars(description = "List of state checks to perform")]
    pub checks: Vec<crate::protocol::VisionStateCheck>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_state_schema_uses_nested_region() {
        let schema = schemars::schema_for!(VisionDetectStateArgs);
        let value = serde_json::to_value(schema).unwrap();
        let properties = &value["$defs"]["VisionStateCheck"]["properties"];

        assert!(properties.get("region").is_some());
        assert!(properties.get("x").is_none());
    }
}
