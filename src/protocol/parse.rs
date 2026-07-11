use super::Action;
use super::types::RequestOptions;

mod a11y;
mod agent;
mod apps;
mod audio;
mod audit;
mod blackboard;
mod bluetooth;
mod browser;
mod clipboard;
mod color_pick;
mod confirmation;
mod desktop;
mod files;
mod helpers;
mod hotkeys;
mod input;
mod location;
mod locks;
mod macro_cmd;
mod monitor;
mod mpris;
mod network;
mod notifications;
mod process;
mod rules;
mod screenshot;
mod search;
mod secrets;
mod sessions;
mod system;
mod watch;
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
        require_confirmation: raw["require_confirmation"].as_bool(),
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

        // Screen/text watching
        s if s.starts_with("region_watch.") || s.starts_with("text_watch.") => {
            watch::parse_watch(&raw, &id, s)?
        }

        // Screencast
        s if s.starts_with("screencast.") => match s {
            "screencast.start" => {
                let output_path = raw["output_path"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("screencast.start requires output_path"))?
                    .to_string();
                Action::ScreencastStart { output_path }
            }
            "screencast.stop" => Action::ScreencastStop,
            _ => anyhow::bail!("unknown screencast action: {}", s),
        },

        // Desktop Portal
        s if s.starts_with("portal.") => match s {
            "portal.screenshot" => Action::PortalScreenshot {
                interactive: raw["interactive"].as_bool().unwrap_or(false),
            },
            "portal.screencast_start" => {
                let output_path = raw["output_path"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("portal.screencast_start requires output_path"))?
                    .to_string();
                Action::PortalScreencastStart { output_path }
            }
            "portal.screencast_stop" => Action::PortalScreencastStop,
            _ => anyhow::bail!("unknown portal action: {}", s),
        },

        // Audit
        s if s.starts_with("audit.") => audit::parse_audit(&raw, &id, s)?,

        // Notifications
        s if s.starts_with("notification.") => notifications::parse_notifications(&raw, &id, s)?,

        // System
        s if s.starts_with("system.")
            || s.starts_with("service.")
            || s.starts_with("journal.")
            || s.starts_with("timer.")
            || s.starts_with("clients.")
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

        // Desktop Settings
        s if s.starts_with("desktop.") => desktop::parse_desktop(&raw, &id, s)?,

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

        // Macro
        s if s.starts_with("macro.") => macro_cmd::parse_macro(&raw, &id, s)?,

        // Sessions
        s if s.starts_with("session.") => sessions::parse_session(&raw, &id, s)?,

        // Rules
        s if s.starts_with("rule.") => rules::parse_rules(&raw, &id, s)?,

        // Blackboard
        s if s.starts_with("blackboard.") => blackboard::parse_blackboard(&raw, &id, s)?,

        // Confirmation
        s if s.starts_with("confirmation.") => confirmation::parse_confirmation(&raw, &id, s)?,

        // Agent messaging
        s if s.starts_with("agent.") => agent::parse_agent(&raw, &id, s)?,

        // Lock / mutex primitives
        s if s.starts_with("lock.") => locks::parse_lock(&raw, &id, s)?,

        // Unified search
        s if s.starts_with("search.") => search::parse_search(&raw, &id, s)?,

        // Secrets/keyring
        s if s.starts_with("secrets.") => secrets::parse_secrets(&raw, &id, s)?,

        // D-Bus raw call — escape hatch, doesn't fit a prefix group
        "dbus.call" => system::parse_system(&raw, &id, msg_type.as_str())?,

        // Power profiles daemon — talks to net.hadess.PowerProfiles via system bus
        s if s.starts_with("power.profile.") => system::parse_system(&raw, &id, msg_type.as_str())?,

        _ => anyhow::bail!("unknown action type: {}", msg_type),
    };

    Ok((id, action, options))
}
