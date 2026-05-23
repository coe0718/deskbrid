use super::Action;
use super::types::RequestOptions;

mod a11y;
mod apps;
mod audio;
mod audit;
mod bluetooth;
mod browser;
mod clipboard;
mod color_pick;
mod files;
mod helpers;
mod hotkeys;
mod input;
mod location;
mod monitor;
mod mpris;
mod network;
mod notifications;
mod process;
mod screenshot;
mod system;
mod windows;
mod workspace_layout;

pub fn from_json(line: &str) -> anyhow::Result<(String, Action)> {
    let (id, action, _) = from_json_with_options(line)?;
    Ok((id, action))
}

pub fn from_json_with_options(line: &str) -> anyhow::Result<(String, Action, RequestOptions)> {
    let raw: serde_json::Value = serde_json::from_str(line)?;
    let msg_type = raw["type"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing 'type' field"))?
        .to_string();
    let id = raw["id"].as_str().unwrap_or("?").to_string();
    let options = RequestOptions {
        dry_run: raw["dry_run"].as_bool().unwrap_or(false),
        timeout_ms: raw["timeout_ms"].as_u64(),
    };

    let action = match msg_type.as_str() {
        "ping" => Action::Ping,
        "subscribe" => {
            let events: Vec<String> = raw["events"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            Action::Subscribe { events }
        }
        "unsubscribe" => {
            let events: Vec<String> = raw["events"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            Action::Unsubscribe { events }
        }
        "disconnect" => Action::Disconnect,

        // Windows
        s if s.starts_with("windows.") => windows::parse_windows(&raw, &id, s)?,

        // Workspaces / Layout profiles
        s if s.starts_with("workspaces.") || s.starts_with("layout_profiles.") => {
            workspace_layout::parse_workspace_layout(&raw, &id, s)?
        }

        // Input
        s if s.starts_with("input.") => input::parse_input(&raw, &id, s)?,

        // Clipboard
        s if s.starts_with("clipboard.") => clipboard::parse_clipboard(&raw, &id, s)?,

        // Apps
        s if s.starts_with("apps.") => apps::parse_apps(&raw, &id, s)?,

        // MPRIS
        s if s.starts_with("mpris.") => mpris::parse_mpris(&raw, &id, s)?,

        // Color picker
        s if s.starts_with("color.") => color_pick::parse_color_pick(&raw, &id, s)?,

        // Screenshot
        s if s == "screenshot" || s.starts_with("screenshot.") => {
            screenshot::parse_screenshot(&raw, &id, s)?
        }

        // Audit
        s if s.starts_with("audit.") => audit::parse_audit(&raw, &id, s)?,

        // Notifications
        s if s.starts_with("notification.") => notifications::parse_notifications(&raw, &id, s)?,

        // System
        s if s.starts_with("system.")
            || s.starts_with("service.")
            || s.starts_with("journal.")
            || s.starts_with("timer.")
            || s == "wait.for" =>
        {
            system::parse_system(&raw, &id, s)?
        }

        // Network
        s if s.starts_with("network.") => network::parse_network(&raw, &id, s)?,

        // Bluetooth
        s if s.starts_with("bluetooth.") => bluetooth::parse_bluetooth(&raw, &id, s)?,

        // Files
        s if s.starts_with("files.") => files::parse_files(&raw, &id, s)?,

        // Browser
        s if s.starts_with("browser.") => browser::parse_browser(&raw, &id, s)?,

        // Accessibility
        s if s.starts_with("a11y.") => a11y::parse_a11y(&raw, &id, s)?,

        // Process / Terminal
        s if s.starts_with("process.") || s.starts_with("terminal.") => {
            process::parse_process(&raw, &id, s)?
        }

        // Hotkeys
        s if s.starts_with("hotkeys.") => hotkeys::parse_hotkeys(&raw, &id, s)?,

        // Audio
        s if s.starts_with("audio.") => audio::parse_audio(&raw, &id, s)?,

        // Monitor
        s if s.starts_with("monitor.") => monitor::parse_monitor(&raw, &id, s)?,

        // Location / UI
        s if s.starts_with("location.") || s.starts_with("ui.") => {
            location::parse_location(&raw, &id, s)?
        }

        // Connection (subscribe / unsubscribe / disconnect handled above)
        "capabilities.list" => Action::CapabilitiesList,

        _ => anyhow::bail!("unknown action type: {}", msg_type),
    };

    Ok((id, action, options))
}
