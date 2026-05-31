use super::t;
use serde_json::{Value, json};

pub fn tools() -> Vec<Value> {
    vec![
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
        // ══════ Screencast ══════
        t(
            "screencast_start",
            "Start recording the desktop to a video file.",
            json!({"type":"object","properties":{"output_path":{"type":"string","description":"Output file path for the recording"}},"required":["output_path"]}),
        ),
        t(
            "screencast_stop",
            "Stop the running screencast recording.",
            json!({"type":"object","properties":{},"required":[]}),
        ),
    ]
}
