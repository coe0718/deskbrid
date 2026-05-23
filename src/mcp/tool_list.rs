//! MCP tool definitions — the full list of tools exposed via MCP.
//! Kept in sync with server.rs rmcp tools.

use serde_json::{Value, json};

pub fn list_tools() -> Vec<Value> {
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
        // ══════ Input ══════
        t(
            "type_text",
            "Type a string via keyboard input.",
            json!({"type":"object","properties":{"text":{"type":"string","description":"Text to type"}},"required":["text"]}),
        ),
        t(
            "press_key",
            "Press a single key (e.g. Return, Escape, Tab).",
            json!({"type":"object","properties":{"key":{"type":"string","description":"Single key name"}},"required":["key"]}),
        ),
        t(
            "press_keys",
            "Press a key combination.",
            json!({"type":"object","properties":{"keys":{"type":"array","items":{"type":"string"},"description":"Keys to press"}},"required":["keys"]}),
        ),
        t(
            "mouse_move",
            "Move the mouse cursor to absolute coordinates.",
            json!({"type":"object","properties":{"x":{"type":"number","description":"X coordinate"},"y":{"type":"number","description":"Y coordinate"}},"required":["x","y"]}),
        ),
        t(
            "mouse_click",
            "Click a mouse button at the current position.",
            json!({"type":"object","properties":{"button":{"type":"string","description":"Button: left, middle, or right"}},"required":[]}),
        ),
        t(
            "mouse_scroll",
            "Scroll the mouse wheel.",
            json!({"type":"object","properties":{"dx":{"type":"number","description":"Horizontal scroll"},"dy":{"type":"number","description":"Vertical scroll (negative = down)"}},"required":[]}),
        ),
        t(
            "click_coordinate",
            "Move to pixel coordinates and click.",
            json!({"type":"object","properties":{"x":{"type":"number","description":"X coordinate"},"y":{"type":"number","description":"Y coordinate"},"button":{"type":"string","description":"Button"}},"required":["x","y"]}),
        ),
        t(
            "drag",
            "Click-and-drag between two pixel coordinates.",
            json!({"type":"object","properties":{"from_x":{"type":"number","description":"Start X"},"from_y":{"type":"number","description":"Start Y"},"to_x":{"type":"number","description":"End X"},"to_y":{"type":"number","description":"End Y"},"button":{"type":"string","description":"Button"}},"required":["from_x","from_y","to_x","to_y"]}),
        ),
        // ══════ Clipboard ══════
        t(
            "clipboard_read",
            "Read the current clipboard contents.",
            json!({"type":"object","properties":{},"required":[]}),
        ),
        t(
            "clipboard_write",
            "Write text to the system clipboard.",
            json!({"type":"object","properties":{"text":{"type":"string","description":"Text to copy"}},"required":["text"]}),
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
        // ══════ Diagnostics ══════
        t(
            "doctor",
            "Check AT-SPI accessibility readiness.",
            json!({"type":"object","properties":{},"required":[]}),
        ),
        t(
            "setup_accessibility",
            "Enable AT-SPI accessibility via gsettings.",
            json!({"type":"object","properties":{},"required":[]}),
        ),
        t(
            "capabilities",
            "List all available Deskbrid capabilities and tool types.",
            json!({"type":"object","properties":{},"required":[]}),
        ),
        // ══════ System ══════
        t(
            "system_info",
            "System information — hostname, OS, kernel, uptime, memory, CPU.",
            json!({"type":"object","properties":{},"required":[]}),
        ),
        t(
            "battery_status",
            "Battery percentage and charging state.",
            json!({"type":"object","properties":{},"required":[]}),
        ),
        t(
            "idle_seconds",
            "User idle time in seconds.",
            json!({"type":"object","properties":{},"required":[]}),
        ),
        t(
            "network_status",
            "Network interfaces, IP addresses, and connectivity state.",
            json!({"type":"object","properties":{},"required":[]}),
        ),
        t(
            "bluetooth_list",
            "List paired Bluetooth devices.",
            json!({"type":"object","properties":{},"required":[]}),
        ),
        t(
            "bluetooth_scan",
            "Scan for nearby Bluetooth devices.",
            json!({"type":"object","properties":{"duration":{"type":"integer","description":"Scan duration in seconds"}},"required":[]}),
        ),
        t(
            "service_status",
            "Check a systemd service's status.",
            json!({"type":"object","properties":{"name":{"type":"string","description":"systemd unit name"}},"required":["name"]}),
        ),
        t(
            "service_start",
            "Start a systemd service.",
            json!({"type":"object","properties":{"name":{"type":"string","description":"systemd unit name"}},"required":["name"]}),
        ),
        t(
            "service_stop",
            "Stop a systemd service.",
            json!({"type":"object","properties":{"name":{"type":"string","description":"systemd unit name"}},"required":["name"]}),
        ),
        t(
            "journal_query",
            "Query the systemd journal.",
            json!({"type":"object","properties":{"since":{"type":"integer","description":"Since timestamp (unix seconds)"},"until":{"type":"integer","description":"Until timestamp"},"unit":{"type":"string","description":"Filter by unit name"},"priority":{"type":"integer","description":"Max priority"},"tail":{"type":"integer","description":"Number of recent entries"}},"required":[]}),
        ),
        // ══════ Audio ══════
        t(
            "list_audio_sinks",
            "List audio output devices with volume and mute state.",
            json!({"type":"object","properties":{},"required":[]}),
        ),
        t(
            "set_volume",
            "Set audio sink volume.",
            json!({"type":"object","properties":{"sink_id":{"type":"integer","description":"Sink ID from list_audio_sinks"},"volume":{"type":"number","description":"Volume 0.0–1.0"}},"required":["sink_id","volume"]}),
        ),
        // ══════ File Operations ══════
        t(
            "file_list",
            "List files and directories at a path.",
            json!({"type":"object","properties":{"path":{"type":"string","description":"Directory path"}},"required":["path"]}),
        ),
        t(
            "file_read",
            "Read contents of a file.",
            json!({"type":"object","properties":{"path":{"type":"string","description":"File path"},"offset":{"type":"integer","description":"Byte offset"},"limit":{"type":"integer","description":"Maximum bytes"}},"required":["path"]}),
        ),
        t(
            "file_write",
            "Write content to a file (create or overwrite).",
            json!({"type":"object","properties":{"path":{"type":"string","description":"File path"},"content":{"type":"string","description":"Content to write"},"append":{"type":"boolean","description":"Append instead of overwrite"}},"required":["path","content"]}),
        ),
        t(
            "file_search",
            "Search filesystem by glob or regex pattern.",
            json!({"type":"object","properties":{"pattern":{"type":"string","description":"Search pattern"},"root":{"type":"string","description":"Root directory"},"max_results":{"type":"integer","description":"Maximum results"}},"required":["pattern"]}),
        ),
        t(
            "file_copy",
            "Copy a file or directory.",
            json!({"type":"object","properties":{"source":{"type":"string","description":"Source path"},"destination":{"type":"string","description":"Destination path"}},"required":["source","destination"]}),
        ),
        t(
            "file_watch",
            "Watch a path for file changes.",
            json!({"type":"object","properties":{"path":{"type":"string","description":"Path to watch"},"recursive":{"type":"boolean","description":"Watch recursively"},"patterns":{"type":"array","items":{"type":"string"},"description":"File patterns"}},"required":["path"]}),
        ),
        // ══════ Terminal ══════
        t(
            "terminal_create",
            "Create a PTY terminal.",
            json!({"type":"object","properties":{"shell":{"type":"string","description":"Shell (default: /bin/bash)"},"cwd":{"type":"string","description":"Working directory"},"rows":{"type":"integer","description":"Terminal rows"},"cols":{"type":"integer","description":"Terminal columns"}},"required":[]}),
        ),
        t(
            "terminal_write",
            "Send input to a terminal.",
            json!({"type":"object","properties":{"terminal_id":{"type":"string","description":"Terminal ID"},"input":{"type":"string","description":"Input to send"}},"required":["terminal_id","input"]}),
        ),
        t(
            "terminal_read",
            "Read output from a terminal.",
            json!({"type":"object","properties":{"terminal_id":{"type":"string","description":"Terminal ID"},"max_bytes":{"type":"integer","description":"Maximum bytes"},"flush":{"type":"boolean","description":"Flush output first"}},"required":["terminal_id"]}),
        ),
        t(
            "terminal_resize",
            "Resize a terminal's rows and columns.",
            json!({"type":"object","properties":{"terminal_id":{"type":"string","description":"Terminal ID"},"rows":{"type":"integer","description":"Rows"},"cols":{"type":"integer","description":"Columns"}},"required":["terminal_id","rows","cols"]}),
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
        // ══════ Process ══════
        t(
            "list_processes",
            "List running processes with PID, name, CPU, and memory.",
            json!({"type":"object","properties":{},"required":[]}),
        ),
        t(
            "start_process",
            "Start a new background process. Returns the PID.",
            json!({"type":"object","properties":{"command":{"type":"array","items":{"type":"string"},"description":"Command and args"},"workdir":{"type":"string","description":"Working directory"}},"required":["command"]}),
        ),
        t(
            "stop_process",
            "Stop a running process by PID.",
            json!({"type":"object","properties":{"pid":{"type":"integer","description":"Process ID"},"signal":{"type":"string","description":"Signal (default: SIGTERM)"}},"required":["pid"]}),
        ),
        t(
            "signal_process",
            "Send a signal to a running process.",
            json!({"type":"object","properties":{"pid":{"type":"integer","description":"Process ID"},"signal":{"type":"string","description":"Signal name"}},"required":["pid","signal"]}),
        ),
        t(
            "process_exists",
            "Check if a process with the given PID exists.",
            json!({"type":"object","properties":{"pid":{"type":"integer","description":"Process ID"}},"required":["pid"]}),
        ),
        t(
            "wait_for_process",
            "Wait for a process to exit.",
            json!({"type":"object","properties":{"pid":{"type":"integer","description":"Process ID"},"timeout_ms":{"type":"integer","description":"Timeout in milliseconds"}},"required":["pid"]}),
        ),
        // ══════ Notifications ══════
        t(
            "send_notification",
            "Send a desktop notification via D-Bus.",
            json!({"type":"object","properties":{"app_name":{"type":"string","description":"App name"},"title":{"type":"string","description":"Title"},"body":{"type":"string","description":"Body text"},"urgency":{"type":"string","description":"Urgency: low, normal, critical"}},"required":["app_name","title","body"]}),
        ),
        t(
            "close_notification",
            "Close a desktop notification by ID.",
            json!({"type":"object","properties":{"notification_id":{"type":"integer","description":"Notification ID"}},"required":["notification_id"]}),
        ),
        // ══════ Hotkeys ══════
        t(
            "register_hotkey",
            "Register a global hotkey combination.",
            json!({"type":"object","properties":{"hotkey_id":{"type":"string","description":"Hotkey identifier"},"keys":{"type":"array","items":{"type":"string"},"description":"Key combination"}},"required":["hotkey_id","keys"]}),
        ),
        t(
            "unregister_hotkey",
            "Unregister a previously registered hotkey.",
            json!({"type":"object","properties":{"hotkey_id":{"type":"string","description":"Hotkey ID"}},"required":["hotkey_id"]}),
        ),
    ]
}

fn t(name: &str, description: &str, input_schema: Value) -> Value {
    json!({"name": name, "description": description, "inputSchema": input_schema})
}
