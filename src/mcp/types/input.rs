//! MCP parameter types for input operations: keyboard, mouse, hotkeys.

use serde::Deserialize;
fn default_button() -> String {
    "left".into()
}

// ── Input ──────────────────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct TypeText {
    #[schemars(description = "Text to type")]
    pub text: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct PressKey {
    #[schemars(description = "Single key name (e.g. 'Return', 'Escape', 'Tab')")]
    pub key: String,
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
pub struct MouseScroll {
    #[schemars(description = "Horizontal scroll delta")]
    #[serde(default)]
    pub dx: f64,
    #[schemars(description = "Vertical scroll delta (negative = down)")]
    #[serde(default)]
    pub dy: f64,
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

// ── Hotkeys ────────────────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct HotkeyRegister {
    #[schemars(description = "Unique hotkey identifier")]
    pub hotkey_id: String,
    #[schemars(description = "Key combination (e.g. ['Control_L', 'Shift_L', 'x'])")]
    pub keys: Vec<String>,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct HotkeyUnregister {
    #[schemars(description = "Hotkey ID to unregister")]
    pub hotkey_id: String,
}
