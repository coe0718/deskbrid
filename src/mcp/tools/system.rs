use super::t;
use serde_json::{Value, json};

pub fn tools() -> Vec<Value> {
    vec![
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
    ]
}
