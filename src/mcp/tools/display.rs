use super::t;
use serde_json::{Value, json};

pub fn tools() -> Vec<Value> {
    vec![
        // ══════ Monitor ══════
        t(
            "list_monitors",
            "List all connected monitors with resolution, position, scale, and refresh rate.",
            json!({"type":"object","properties":{},"required":[]}),
        ),
        t(
            "set_primary_monitor",
            "Set a monitor as the primary display.",
            json!({"type":"object","properties":{"output":{"type":"string","description":"Monitor output name"}},"required":["output"]}),
        ),
        t(
            "set_monitor_resolution",
            "Change a monitor's resolution and optionally refresh rate.",
            json!({"type":"object","properties":{"output":{"type":"string","description":"Monitor output name"},"width":{"type":"integer","description":"Width"},"height":{"type":"integer","description":"Height"},"refresh_rate":{"type":"number","description":"Refresh rate in Hz"}},"required":["output","width","height"]}),
        ),
        t(
            "set_monitor_scale",
            "Set a monitor's display scale factor.",
            json!({"type":"object","properties":{"output":{"type":"string","description":"Monitor output name"},"scale":{"type":"number","description":"Scale factor"}},"required":["output","scale"]}),
        ),
        t(
            "set_monitor_rotation",
            "Rotate a monitor's display output.",
            json!({"type":"object","properties":{"output":{"type":"string","description":"Monitor output name"},"rotation":{"type":"string","description":"Rotation: normal, left, right, inverted"}},"required":["output","rotation"]}),
        ),
        t(
            "enable_monitor",
            "Enable a previously disabled monitor.",
            json!({"type":"object","properties":{"output":{"type":"string","description":"Monitor output name"}},"required":["output"]}),
        ),
        t(
            "disable_monitor",
            "Disable a monitor output.",
            json!({"type":"object","properties":{"output":{"type":"string","description":"Monitor output name"}},"required":["output"]}),
        ),
        // ══════ Browser (CDP) ══════
        t(
            "list_browser_tabs",
            "List open browser tabs via Chrome DevTools Protocol.",
            json!({"type":"object","properties":{},"required":[]}),
        ),
        t(
            "browser_navigate",
            "Navigate a browser tab to a URL.",
            json!({"type":"object","properties":{"tab_index":{"type":"integer","description":"Tab index"},"url":{"type":"string","description":"URL"}},"required":["url"]}),
        ),
        t(
            "browser_evaluate",
            "Evaluate JavaScript in a browser tab.",
            json!({"type":"object","properties":{"tab_index":{"type":"integer","description":"Tab index"},"expression":{"type":"string","description":"JavaScript expression"},"await_promise":{"type":"boolean","description":"Wait for promise"}},"required":["expression"]}),
        ),
        t(
            "browser_screenshot",
            "Take a screenshot of a browser tab.",
            json!({"type":"object","properties":{"tab_index":{"type":"integer","description":"Tab index"}},"required":[]}),
        ),
        t(
            "browser_click",
            "Click an element in a browser tab by CSS selector.",
            json!({"type":"object","properties":{"tab_index":{"type":"integer","description":"Tab index"},"selector":{"type":"string","description":"CSS selector"}},"required":["selector"]}),
        ),
        // ══════ MPRIS ══════
        t(
            "list_media_players",
            "List MPRIS media players on the D-Bus session bus.",
            json!({"type":"object","properties":{},"required":[]}),
        ),
        t(
            "media_player_info",
            "Get detailed info about an MPRIS media player.",
            json!({"type":"object","properties":{"player":{"type":"string","description":"Player bus name"}},"required":[]}),
        ),
        t(
            "media_player_control",
            "Control an MPRIS media player (play, pause, next, previous, stop).",
            json!({"type":"object","properties":{"player":{"type":"string","description":"Player bus name"},"action":{"type":"string","description":"Action"}},"required":["action"]}),
        ),
        // ══════ Layout Profiles ══════
        t(
            "layout_list",
            "List saved window layout profiles.",
            json!({"type":"object","properties":{},"required":[]}),
        ),
        t(
            "layout_save",
            "Save current window layout as a named profile.",
            json!({"type":"object","properties":{"name":{"type":"string","description":"Layout profile name"},"overwrite":{"type":"boolean","description":"Overwrite existing"}},"required":["name"]}),
        ),
        t(
            "layout_restore",
            "Restore a saved window layout profile.",
            json!({"type":"object","properties":{"name":{"type":"string","description":"Layout profile name"}},"required":["name"]}),
        ),
        t(
            "layout_delete",
            "Delete a saved layout profile.",
            json!({"type":"object","properties":{"name":{"type":"string","description":"Layout profile name"}},"required":["name"]}),
        ),
    ]
}
