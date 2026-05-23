use super::overrides::set_requires;

pub fn apply_shared_linux_tool_capabilities(
    actions: &mut serde_json::Map<String, serde_json::Value>,
    desktop: &str,
) {
    set_requires(actions, "audio.list_sinks", &["pactl"]);
    set_requires(actions, "audio.set_sink_volume", &["pactl"]);
    set_requires(actions, "files.search", &["find"]);
    set_requires(actions, "network.wifi.connect", &["nmcli"]);

    if !desktop.contains("gnome") {
        set_requires(actions, "network.status", &["nmcli"]);
        set_requires(actions, "network.interfaces", &["nmcli"]);
        set_requires(actions, "network.wifi.scan", &["nmcli"]);
        for action in [
            "bluetooth.list",
            "bluetooth.scan",
            "bluetooth.scan_stop",
            "bluetooth.connect",
            "bluetooth.disconnect",
        ] {
            set_requires(actions, action, &["bluetoothctl"]);
        }
    }

    if desktop.contains("x11") {
        set_requires(actions, "system.idle", &["xprintidle"]);
    }
}
