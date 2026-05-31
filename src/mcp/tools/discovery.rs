use super::t;
use serde_json::{Value, json};

pub fn tools() -> Vec<Value> {
    vec![
        // ══════ Discovery ══════
        t(
            "list_windows",
            "List all open windows with IDs, titles, classes, workspace, and geometry.",
            json!({"type":"object","properties":{},"required":[]}),
        ),
        t(
            "focused_window",
            "Get the currently focused/active window.",
            json!({"type":"object","properties":{},"required":[]}),
        ),
        t(
            "list_workspaces",
            "List all virtual desktops/workspaces with current state.",
            json!({"type":"object","properties":{},"required":[]}),
        ),
        t(
            "list_apps",
            "List AT-SPI application roots running on the desktop.",
            json!({"type":"object","properties":{},"required":[]}),
        ),
        t(
            "get_accessibility_tree",
            "Full AT-SPI tree for an app or window with bounds, roles, states, actions, and text.",
            json!({"type":"object","properties":{"app_name":{"type":"string","description":"Filter by app name"},"pid":{"type":"integer","description":"Filter by process ID"},"max_nodes":{"type":"integer","description":"Maximum nodes (default: 200)"},"max_depth":{"type":"integer","description":"Maximum depth (default: 10)"}},"required":[]}),
        ),
        t(
            "screenshot",
            "Take a screenshot. Returns base64-encoded PNG.",
            json!({"type":"object","properties":{},"required":[]}),
        ),
        t(
            "screenshot_region",
            "Capture a region of the screen or a specific window.",
            json!({"type":"object","properties":{"monitor":{"type":"integer","description":"Monitor index"},"window_id":{"type":"string","description":"Window ID to capture"},"region_x":{"type":"integer","description":"Region x"},"region_y":{"type":"integer","description":"Region y"},"region_w":{"type":"integer","description":"Region width"},"region_h":{"type":"integer","description":"Region height"}},"required":[]}),
        ),
        t(
            "screenshot_diff",
            "Pixel diff between two screenshots. Useful for detecting UI changes.",
            json!({"type":"object","properties":{"before_path":{"type":"string","description":"Path to before screenshot"},"after_path":{"type":"string","description":"Path to after screenshot"},"tolerance":{"type":"integer","description":"Pixel tolerance (default: 10)"},"diff_path":{"type":"string","description":"Save diff image"},"monitor":{"type":"integer","description":"Monitor index"}},"required":["before_path"]}),
        ),
        // ══════ Window Control ══════
        t(
            "focus_window",
            "Focus (activate) a window by its ID.",
            json!({"type":"object","properties":{"window_id":{"type":"string","description":"Window ID from list_windows"}},"required":["window_id"]}),
        ),
        t(
            "close_window",
            "Close a window by its ID.",
            json!({"type":"object","properties":{"window_id":{"type":"string","description":"Window ID from list_windows"}},"required":["window_id"]}),
        ),
        t(
            "minimize_window",
            "Minimize a window by its ID.",
            json!({"type":"object","properties":{"window_id":{"type":"string","description":"Window ID from list_windows"}},"required":["window_id"]}),
        ),
        t(
            "maximize_window",
            "Maximize a window by its ID.",
            json!({"type":"object","properties":{"window_id":{"type":"string","description":"Window ID from list_windows"}},"required":["window_id"]}),
        ),
        t(
            "move_resize_window",
            "Move and/or resize a window.",
            json!({"type":"object","properties":{"window_id":{"type":"string","description":"Window ID"},"x":{"type":"integer","description":"X position"},"y":{"type":"integer","description":"Y position"},"width":{"type":"integer","description":"Width in pixels"},"height":{"type":"integer","description":"Height in pixels"}},"required":["window_id","x","y","width","height"]}),
        ),
        t(
            "tile_window",
            "Tile a window to a preset position.",
            json!({"type":"object","properties":{"window_id":{"type":"string","description":"Window ID"},"preset":{"type":"string","description":"Preset: left, right, maximize, fullscreen"},"monitor":{"type":"integer","description":"Monitor index"},"padding":{"type":"integer","description":"Padding in pixels"}},"required":["window_id","preset"]}),
        ),
        t(
            "activate_or_launch",
            "Focus an existing app window or launch it if not running.",
            json!({"type":"object","properties":{"app_id":{"type":"string","description":"Application ID"},"command":{"type":"array","items":{"type":"string"},"description":"Launch command"},"workdir":{"type":"string","description":"Working directory"}},"required":["app_id"]}),
        ),
        // ══════ Workspaces ══════
        t(
            "switch_workspace",
            "Switch to a specific workspace by index.",
            json!({"type":"object","properties":{"workspace_id":{"type":"integer","description":"Workspace index (0-based)"}},"required":["workspace_id"]}),
        ),
        t(
            "move_window_to_workspace",
            "Move a window to another workspace.",
            json!({"type":"object","properties":{"window_id":{"type":"string","description":"Window ID"},"workspace_id":{"type":"integer","description":"Target workspace index"},"follow":{"type":"boolean","description":"Follow window to target"}},"required":["window_id","workspace_id"]}),
        ),
        // ══════ AT-SPI ══════
        t(
            "perform_action",
            "Perform an AT-SPI action on an accessibility element.",
            json!({"type":"object","properties":{"object_ref":{"type":"string","description":"AT-SPI object reference path"},"action_name":{"type":"string","description":"Action name (click, activate, toggle)"}},"required":["object_ref"]}),
        ),
        t(
            "set_element_value",
            "Set the text value of an AT-SPI editable element.",
            json!({"type":"object","properties":{"object_ref":{"type":"string","description":"AT-SPI object reference path"},"value":{"type":"string","description":"Value to set"}},"required":["object_ref","value"]}),
        ),
        t(
            "get_element_text",
            "Get the text content from an AT-SPI element.",
            json!({"type":"object","properties":{"object_ref":{"type":"string","description":"AT-SPI object reference path"},"max_chars":{"type":"integer","description":"Maximum characters"}},"required":["object_ref"]}),
        ),
        t(
            "click_element",
            "Click an AT-SPI element using its bounds.",
            json!({"type":"object","properties":{"object_ref":{"type":"string","description":"AT-SPI object reference path"}},"required":["object_ref"]}),
        ),
    ]
}
