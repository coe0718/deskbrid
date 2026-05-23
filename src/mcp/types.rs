//! Input parameter types for MCP tools, with JsonSchema derives for tool discovery.
//! Each type maps to one MCP tool's parameters.

use serde::Deserialize;

#[derive(Deserialize, schemars::JsonSchema)]
pub struct WindowId {
    #[schemars(description = "Window ID from list_windows")]
    pub window_id: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct TypeText {
    #[schemars(description = "Text to type")]
    pub text: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct PressKeys {
    #[schemars(description = "Keys to press (e.g. ['Control_L', 'c'])")]
    pub keys: Vec<String>,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct MouseMove {
    #[schemars(description = "X coordinate")]
    pub x: f64,
    #[schemars(description = "Y coordinate")]
    pub y: f64,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct MouseClick {
    #[schemars(description = "Button: 'left', 'middle', or 'right'")]
    #[serde(default = "default_button")]
    pub button: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct ClickCoord {
    #[schemars(description = "X coordinate")]
    pub x: f64,
    #[schemars(description = "Y coordinate")]
    pub y: f64,
    #[schemars(description = "Button")]
    #[serde(default = "default_button")]
    pub button: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct Drag {
    #[schemars(description = "Start X")]
    pub from_x: f64,
    #[schemars(description = "Start Y")]
    pub from_y: f64,
    #[schemars(description = "End X")]
    pub to_x: f64,
    #[schemars(description = "End Y")]
    pub to_y: f64,
    #[schemars(description = "Button")]
    #[serde(default = "default_button")]
    pub button: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct ClipboardWrite {
    #[schemars(description = "Text to copy to clipboard")]
    pub text: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct A11yTree {
    #[schemars(description = "Filter by app name")]
    pub app_name: Option<String>,
    #[schemars(description = "Filter by process ID")]
    pub pid: Option<u32>,
    #[schemars(description = "Maximum nodes (default: 200)")]
    pub max_nodes: Option<usize>,
    #[schemars(description = "Maximum depth (default: 10)")]
    pub max_depth: Option<u32>,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct A11yAction {
    #[schemars(description = "AT-SPI object reference path")]
    pub object_ref: String,
    #[schemars(description = "Action name (e.g. 'click', 'activate')")]
    pub action_name: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct SetValue {
    #[schemars(description = "AT-SPI object reference path")]
    pub object_ref: String,
    #[schemars(description = "Value to set")]
    pub value: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct GetText {
    #[schemars(description = "AT-SPI object reference path")]
    pub object_ref: String,
    #[schemars(description = "Maximum characters to return")]
    pub max_chars: Option<i32>,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct ClickElement {
    #[schemars(description = "AT-SPI object reference path")]
    pub object_ref: String,
}

pub fn default_button() -> String {
    "left".into()
}
