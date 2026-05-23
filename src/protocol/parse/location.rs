use crate::protocol::Action;
use serde_json::Value;

pub(super) fn parse_location(raw: &Value, _id: &str, type_str: &str) -> anyhow::Result<Action> {
    Ok(match type_str {
        // Location
        "location.get" => Action::LocationGet,
        "ui.tree.get" => Action::UiTreeGet,
        "ui.element.click" => Action::UiElementClick {
            selector: raw["selector"].as_str().unwrap_or("").into(),
            tab_index: raw["tab_index"].as_u64().map(|v| v as u32),
        },
        "ui.element.set_text" => Action::UiElementSetText {
            selector: raw["selector"].as_str().unwrap_or("").into(),
            text: raw["text"].as_str().unwrap_or("").into(),
            tab_index: raw["tab_index"].as_u64().map(|v| v as u32),
        },
        _ => anyhow::bail!("unknown location type: {type_str}"),
    })
}
