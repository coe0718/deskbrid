use super::helpers::*;
use crate::protocol::Action;
use serde_json::Value;

pub(super) fn parse_input(raw: &Value, _id: &str, type_str: &str) -> anyhow::Result<Action> {
    Ok(match type_str {
        // Input
        "input.keyboard" => {
            let sub = raw["action"].as_str().unwrap_or("key");
            match sub {
                "type" => Action::InputKeyboardType {
                    text: raw["text"].as_str().unwrap_or("").into(),
                },
                "combo" => {
                    let keys: Vec<String> = raw["keys"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();
                    Action::InputKeyboardCombo { keys }
                }
                _ => Action::InputKeyboardKey {
                    key: raw["key"].as_str().unwrap_or("").into(),
                },
            }
        }
        "input.mouse" => Action::InputMouse {
            action: raw["action"].as_str().unwrap_or("move").into(),
            x: raw["x"].as_f64(),
            y: raw["y"].as_f64(),
            button: raw["button"].as_str().map(String::from),
            dx: raw["dx"].as_f64(),
            dy: raw["dy"].as_f64(),
        },
        "input.mouse.drag" => Action::InputMouseDrag {
            from_x: required_number(raw, "from_x")?,
            from_y: required_number(raw, "from_y")?,
            to_x: required_number(raw, "to_x")?,
            to_y: required_number(raw, "to_y")?,
            button: raw["button"].as_str().map(String::from),
            duration_ms: raw["duration_ms"].as_u64(),
        },
        // Keyboard layouts
        "input.layouts.list" | "input.list_layouts" => Action::InputListLayouts,
        "input.layout.get" | "input.get_layout" => Action::InputGetLayout,
        "input.layout.set" | "input.set_layout" => Action::InputSetLayout {
            index: raw["index"].as_u64().map(|v| v as u32),
            name: raw["name"].as_str().map(String::from),
            variant: raw["variant"].as_str().map(String::from),
        },
        "input.layout.add" | "input.add_layout" => Action::InputAddLayout {
            name: raw["name"].as_str().map(String::from).unwrap_or_default(),
            variant: raw["variant"].as_str().map(String::from),
        },
        "input.layout.remove" | "input.remove_layout" => Action::InputRemoveLayout {
            index: raw["index"].as_u64().map(|v| v as u32).unwrap_or(0),
        },
        _ => anyhow::bail!("unknown input type: {type_str}"),
    })
}
