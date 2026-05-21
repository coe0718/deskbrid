pub fn role_name(id: u32) -> String {
    match id {
        0 => "invalid",
        1 => "alert",
        4 => "check_box",
        7 => "combo_box",
        11 => "dialog",
        17 => "frame",
        24 => "label",
        26 => "list",
        27 => "list_item",
        28 => "menu",
        29 => "menu_bar",
        30 => "menu_item",
        34 => "panel",
        35 => "password_text",
        38 => "push_button",
        39 => "radio_button",
        44 => "scroll_pane",
        50 => "table",
        51 => "table_cell",
        55 => "terminal",
        56 => "text",
        57 => "toggle_button",
        58 => "tool_bar",
        64 => "window",
        70 => "application",
        74 => "entry",
        94 => "grouping",
        _ => "unknown",
    }
    .into()
}

pub fn parse_states(bits: &[u32]) -> Vec<String> {
    let names = [
        "active",
        "armed",
        "busy",
        "checked",
        "collapsed",
        "defunct",
        "editable",
        "enabled",
        "expandable",
        "expanded",
        "focusable",
        "focused",
        "has_tooltip",
        "horizontal",
        "iconified",
        "modal",
        "multi_line",
        "multiselectable",
        "opaque",
        "pressed",
        "resizable",
        "selectable",
        "selected",
        "sensitive",
        "showing",
        "single_line",
        "stale",
        "transient",
        "vertical",
        "visible",
        "manages_descendants",
        "indeterminate",
        "required",
        "truncated",
        "animated",
        "invalid_entry",
        "supports_autocompletion",
        "selectable_text",
        "is_default",
        "visited",
        "checkable",
        "has_popup",
        "read_only",
    ];
    let mut states = Vec::new();
    for (i, name) in names.iter().enumerate() {
        let word = i / 32;
        let bit = i % 32;
        if let Some(mask) = bits.get(word)
            && mask & (1u32 << bit) != 0
        {
            states.push((*name).to_string());
        }
    }
    states
}
