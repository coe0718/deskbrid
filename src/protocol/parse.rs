// reason: exhaustive match on 90+ Action enum variants — cannot split without breaking exhaustiveness

use super::Action;
use super::types::{Region, RequestOptions};

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

        // Windows
        "windows.list" => Action::WindowsList,
        "windows.focus" => Action::WindowsFocus(required_non_empty_string(&raw, "window_id")?),
        "windows.get" => Action::WindowsGet(required_non_empty_string(&raw, "window_id")?),
        "windows.close" => Action::WindowsClose(required_non_empty_string(&raw, "window_id")?),
        "windows.minimize" => {
            Action::WindowsMinimize(required_non_empty_string(&raw, "window_id")?)
        }
        "windows.maximize" => {
            Action::WindowsMaximize(required_non_empty_string(&raw, "window_id")?)
        }
        "windows.move_resize" => {
            let x = raw["x"]
                .as_i64()
                .ok_or_else(|| anyhow::anyhow!("missing or invalid 'x' field"))?
                as i32;
            let y = raw["y"]
                .as_i64()
                .ok_or_else(|| anyhow::anyhow!("missing or invalid 'y' field"))?
                as i32;
            let width = raw["width"]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("missing or invalid 'width' field"))?
                as u32;
            let height = raw["height"]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("missing or invalid 'height' field"))?
                as u32;
            if width == 0 || height == 0 {
                anyhow::bail!("'width' and 'height' must be positive");
            }
            Action::WindowsMoveResize {
                window_id: required_non_empty_string(&raw, "window_id")?,
                x,
                y,
                width,
                height,
            }
        }
        "windows.activate_or_launch" => Action::WindowsActivateOrLaunch {
            app_id: required_non_empty_string(&raw, "app_id")?,
            command: optional_string_array(&raw, "command")?,
            workdir: raw["workdir"].as_str().map(String::from),
            env: raw["env"].as_object().map(|o| {
                o.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                    .collect()
            }),
        },

        // Workspaces
        "workspaces.list" => Action::WorkspacesList,
        "workspaces.switch" => {
            Action::WorkspaceSwitch(raw["workspace_id"].as_u64().unwrap_or(0) as u32)
        }
        "workspaces.move_window" => Action::WorkspaceMoveWindow {
            window_id: required_non_empty_string(&raw, "window_id")?,
            workspace_id: raw["workspace_id"].as_u64().unwrap_or(0) as u32,
            follow: raw["follow"].as_bool().unwrap_or(false),
        },

        // Layout profiles
        "layout_profiles.list" => Action::LayoutProfilesList,
        "layout_profiles.get" => Action::LayoutProfileGet {
            name: required_non_empty_string(&raw, "name")?,
        },
        "layout_profiles.save" => Action::LayoutProfileSave {
            name: required_non_empty_string(&raw, "name")?,
            overwrite: raw["overwrite"].as_bool().unwrap_or(false),
        },
        "layout_profiles.delete" => Action::LayoutProfileDelete {
            name: required_non_empty_string(&raw, "name")?,
        },
        "layout_profiles.restore" => Action::LayoutProfileRestore {
            name: required_non_empty_string(&raw, "name")?,
        },

        // Input
        "input.keyboard" => {
            let sub = raw["action"].as_str().unwrap_or("key");
            match sub {
                "type" => Action::InputKeyboardType {
                    text: raw["text"].as_str().unwrap_or("").into(),
                },
                "combo" => {
                    let keys: Vec<String> = raw["keys"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();
                    Action::InputKeyboardCombo { keys }
                }
                _ => Action::InputKeyboardKey {
                    key: raw["key"].as_str().unwrap_or("").into(),
                },
            }
        }
        "input.mouse" => Action::InputMouse {
            action: raw["action"].as_str().unwrap_or("move").into(),
            x: raw["x"].as_f64(),
            y: raw["y"].as_f64(),
            button: raw["button"].as_str().map(String::from),
            dx: raw["dx"].as_f64(),
            dy: raw["dy"].as_f64(),
        },

        // Clipboard
        "clipboard.read" => Action::ClipboardRead,
        "clipboard.write" => Action::ClipboardWrite {
            text: raw["text"].as_str().unwrap_or("").into(),
        },

        // Screenshot
        "screenshot" => Action::Screenshot {
            monitor: raw["monitor"].as_u64().map(|v| v as u32),
            region: raw.get("region").and_then(|r| {
                Some(Region {
                    x: r["x"].as_u64()? as u32,
                    y: r["y"].as_u64()? as u32,
                    width: r["width"].as_u64()? as u32,
                    height: r["height"].as_u64()? as u32,
                })
            }),
            window_id: raw["window_id"].as_str().map(String::from),
        },
        "screenshot.ocr" => Action::ScreenshotOcr {
            path: optional_non_empty_string(&raw, "path")?,
            language: optional_non_empty_string(&raw, "language")?,
            psm: optional_u32(&raw, "psm")?,
            bounding_boxes: raw["bounding_boxes"].as_bool().unwrap_or(false),
            monitor: raw["monitor"].as_u64().map(|v| v as u32),
            region: raw.get("region").and_then(|r| {
                Some(Region {
                    x: r["x"].as_u64()? as u32,
                    y: r["y"].as_u64()? as u32,
                    width: r["width"].as_u64()? as u32,
                    height: r["height"].as_u64()? as u32,
                })
            }),
            window_id: raw["window_id"].as_str().map(String::from),
        },
        "screenshot.diff" => Action::ScreenshotDiff {
            before_path: required_non_empty_string_alias(&raw, "before_path", "before")?,
            after_path: optional_non_empty_string_alias(&raw, "after_path", "after")?,
            tolerance: optional_u8(&raw, "tolerance")?,
            diff_path: optional_non_empty_string(&raw, "diff_path")?,
            save_diff: raw["save_diff"].as_bool().unwrap_or(false),
            monitor: raw["monitor"].as_u64().map(|v| v as u32),
            region: raw.get("region").and_then(|r| {
                Some(Region {
                    x: r["x"].as_u64()? as u32,
                    y: r["y"].as_u64()? as u32,
                    width: r["width"].as_u64()? as u32,
                    height: r["height"].as_u64()? as u32,
                })
            }),
            window_id: raw["window_id"].as_str().map(String::from),
        },

        // Audit
        "audit.log" => Action::AuditLog {
            limit: raw["limit"].as_u64().map(|value| value as usize),
            action_type: optional_non_empty_string(&raw, "action_type")?,
            status: optional_non_empty_string(&raw, "status")?,
        },
        "audit.clear" => Action::AuditClear,

        // Notifications
        "notification.send" => Action::NotificationSend {
            app_name: raw["app_name"].as_str().unwrap_or("deskbrid").into(),
            title: raw["title"].as_str().unwrap_or("").into(),
            body: raw["body"].as_str().unwrap_or("").into(),
            urgency: raw["urgency"].as_str().unwrap_or("normal").into(),
        },
        "notification.close" => Action::NotificationClose {
            notification_id: raw["notification_id"].as_u64().unwrap_or(0) as u32,
        },

        // System
        "system.info" => Action::SystemInfo,
        "system.capabilities" => Action::SystemCapabilities,
        "system.health" => Action::SystemHealth,
        "system.confinement" => Action::SystemConfinement,
        "system.remediate" => Action::SystemRemediate {
            check: raw["check"].as_str().unwrap_or("").into(),
            apply: raw["apply"].as_bool().unwrap_or(false),
        },
        "system.normalize_coords" => Action::SystemNormalizeCoords {
            x: raw["x"].as_f64().unwrap_or(0.0),
            y: raw["y"].as_f64().unwrap_or(0.0),
            monitor: raw["monitor"].as_u64().map(|m| m as u32),
        },
        "wait.for" => Action::WaitFor {
            condition: required_non_empty_string(&raw, "condition")?,
            params: raw
                .get("params")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({})),
            timeout_ms: raw["timeout_ms"]
                .as_u64()
                .or_else(|| raw["timeout"].as_u64())
                .unwrap_or(30_000),
            interval_ms: raw["interval_ms"].as_u64(),
        },
        "system.idle" => Action::SystemIdle,
        "system.power" => Action::SystemPower {
            action: raw["action"].as_str().unwrap_or("").into(),
        },
        "system.battery" => Action::SystemBattery,
        "system.inhibit" => Action::SystemInhibit {
            what: required_non_empty_string(&raw, "what")?,
            who: required_non_empty_string(&raw, "who")?,
            why: raw["why"].as_str().map(String::from),
            mode: raw["mode"].as_str().map(String::from),
        },
        "system.release_inhibit" => Action::SystemReleaseInhibit {
            inhibitor_id: required_positive_u32(&raw, "inhibitor_id")?,
        },
        "system.sessions" => Action::SystemListSessions,
        "system.lock_session" => Action::SystemLockSession {
            session_id: optional_non_empty_string(&raw, "session_id")?,
        },
        "system.switch_user" => Action::SystemSwitchUser {
            username: required_non_empty_string(&raw, "username")?,
        },
        "system.check_auth" => Action::SystemCheckAuth {
            action_id: required_non_empty_string(&raw, "action_id")?,
        },
        "system.elevate" => Action::SystemElevate {
            action_id: required_non_empty_string(&raw, "action_id")?,
            reason: raw["reason"].as_str().map(String::from),
        },
        "service.status" => Action::ServiceStatus {
            name: required_non_empty_string(&raw, "name")?,
        },
        "service.start" => Action::ServiceStart {
            name: required_non_empty_string(&raw, "name")?,
        },
        "service.stop" => Action::ServiceStop {
            name: required_non_empty_string(&raw, "name")?,
        },
        "service.restart" => Action::ServiceRestart {
            name: required_non_empty_string(&raw, "name")?,
        },
        "service.enable" => Action::ServiceEnable {
            name: required_non_empty_string(&raw, "name")?,
            runtime: raw["runtime"].as_bool().unwrap_or(false),
        },
        "service.disable" => Action::ServiceDisable {
            name: required_non_empty_string(&raw, "name")?,
            runtime: raw["runtime"].as_bool().unwrap_or(false),
        },
        "service.list" => Action::ServiceList {
            unit_type: raw["unit_type"].as_str().map(String::from),
        },
        "journal.query" => Action::JournalQuery {
            since: raw["since"].as_u64(),
            until: raw["until"].as_u64(),
            unit: optional_non_empty_string(&raw, "unit")?,
            priority: optional_priority(&raw, "priority")?,
            tail: optional_u32(&raw, "tail")?,
        },
        "timer.list" => Action::TimerList,
        "timer.start" => Action::TimerStart {
            name: required_non_empty_string(&raw, "name")?,
        },
        "timer.stop" => Action::TimerStop {
            name: required_non_empty_string(&raw, "name")?,
        },

        // Network
        "network.status" => Action::NetworkStatus,
        "network.interfaces" => Action::NetworkInterfaces,
        "network.wifi.scan" => Action::NetworkWifiScan,
        "network.wifi.connect" => Action::NetworkWifiConnect {
            ssid: raw["ssid"].as_str().unwrap_or("").into(),
            password: raw["password"].as_str().map(String::from),
        },

        // Bluetooth
        "bluetooth.list" => Action::BluetoothList,
        "bluetooth.scan" => Action::BluetoothScan {
            duration: raw["duration"].as_u64().map(|v| v as u32),
        },
        "bluetooth.scan_stop" => Action::BluetoothStopScan,
        "bluetooth.connect" => Action::BluetoothConnect {
            address: raw["address"].as_str().unwrap_or("").into(),
        },
        "bluetooth.disconnect" => Action::BluetoothDisconnect {
            address: raw["address"].as_str().unwrap_or("").into(),
        },
        "bluetooth.pair" => Action::BluetoothPair {
            address: raw["address"].as_str().unwrap_or("").into(),
        },
        "bluetooth.forget" => Action::BluetoothForget {
            address: raw["address"].as_str().unwrap_or("").into(),
        },

        // Files
        "files.watch" => Action::FilesWatch {
            path: raw["path"].as_str().unwrap_or("").into(),
            recursive: raw["recursive"].as_bool().unwrap_or(true),
            patterns: raw["patterns"].as_array().map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            }),
        },
        "files.unwatch" => Action::FilesUnwatch {
            path: raw["path"].as_str().unwrap_or("").into(),
        },
        "files.search" => Action::FilesSearch {
            pattern: raw["pattern"].as_str().unwrap_or("").into(),
            root: raw["root"].as_str().map(String::from),
            max_results: raw["max_results"].as_u64().unwrap_or(50) as u32,
        },
        "files.read" => Action::FilesRead {
            path: raw["path"].as_str().unwrap_or("").into(),
            offset: raw["offset"].as_u64(),
            limit: raw["limit"].as_u64(),
        },
        "files.write" => Action::FilesWrite {
            path: raw["path"].as_str().unwrap_or("").into(),
            content: raw["content"].as_str().unwrap_or("").into(),
            append: raw["append"].as_bool().unwrap_or(false),
        },
        "files.copy" => Action::FilesCopy {
            source: raw["source"].as_str().unwrap_or("").into(),
            destination: raw["destination"].as_str().unwrap_or("").into(),
        },
        "files.move" => Action::FilesMove {
            source: raw["source"].as_str().unwrap_or("").into(),
            destination: raw["destination"].as_str().unwrap_or("").into(),
        },
        "files.delete" => Action::FilesDelete {
            path: raw["path"].as_str().unwrap_or("").into(),
            recursive: raw["recursive"].as_bool().unwrap_or(false),
        },
        "files.mkdir" => Action::FilesMkdir {
            path: raw["path"].as_str().unwrap_or("").into(),
            parents: raw["parents"].as_bool().unwrap_or(true),
        },
        "files.list" => Action::FilesList {
            path: raw["path"].as_str().unwrap_or(".").into(),
        },

        // Browser
        "browser.list_tabs" => Action::BrowserListTabs,
        "browser.navigate" => Action::BrowserNavigate {
            tab_index: raw["tab_index"].as_u64().map(|v| v as u32),
            url: raw["url"].as_str().unwrap_or("").into(),
        },
        "browser.evaluate" => Action::BrowserEvaluate {
            tab_index: raw["tab_index"].as_u64().map(|v| v as u32),
            expression: raw["expression"].as_str().unwrap_or("").into(),
            await_promise: raw["await_promise"].as_bool().unwrap_or(true),
        },
        "browser.screenshot_tab" => Action::BrowserScreenshotTab {
            tab_index: raw["tab_index"].as_u64().map(|v| v as u32),
        },
        "browser.click" => Action::BrowserClick {
            tab_index: raw["tab_index"].as_u64().map(|v| v as u32),
            selector: raw["selector"].as_str().unwrap_or("").into(),
        },

        // Accessibility
        "a11y.tree" => Action::A11yTree {
            depth: raw["depth"].as_u64().map(|v| v as u32),
        },
        "a11y.get_element" => Action::A11yGetElement {
            role: raw["role"].as_str().map(String::from),
            name: raw["name"].as_str().map(String::from),
            index: raw["index"].as_u64().map(|v| v as u32),
        },
        "a11y.click_element" => Action::A11yClickElement {
            role: raw["role"].as_str().map(String::from),
            name: raw["name"].as_str().map(String::from),
            index: raw["index"].as_u64().map(|v| v as u32),
        },
        "a11y.get_text" => Action::A11yGetText {
            role: raw["role"].as_str().map(String::from),
            name: raw["name"].as_str().map(String::from),
            index: raw["index"].as_u64().map(|v| v as u32),
        },

        // Process
        "process.list" => Action::ProcessList,
        "process.start" => Action::ProcessStart {
            command: raw["command"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            workdir: raw["workdir"].as_str().map(String::from),
            env: raw["env"].as_object().map(|o| {
                o.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                    .collect()
            }),
        },
        "process.stop" => Action::ProcessStop {
            pid: raw["pid"].as_u64().unwrap_or(0) as u32,
            signal: raw["signal"].as_str().map(String::from),
        },
        "process.signal" => Action::ProcessSignal {
            pid: raw["pid"].as_u64().unwrap_or(0) as u32,
            signal: raw["signal"].as_str().unwrap_or("TERM").to_string(),
        },
        "process.exists" => Action::ProcessExists {
            pid: raw["pid"].as_u64().ok_or_else(|| {
                anyhow::anyhow!("missing or invalid 'pid' in process.exists request")
            })? as u32,
        },
        "process.wait" => Action::ProcessWait {
            pid: raw["pid"].as_u64().ok_or_else(|| {
                anyhow::anyhow!("missing or invalid 'pid' in process.wait request")
            })? as u32,
            timeout_ms: raw["timeout_ms"].as_u64(),
        },
        "terminal.create" => Action::TerminalCreate {
            shell: optional_non_empty_string(&raw, "shell")?,
            cwd: optional_non_empty_string(&raw, "cwd")?,
            env: raw["env"].as_object().map(|o| {
                o.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                    .collect()
            }),
            rows: optional_positive_u16(&raw, "rows")?,
            cols: optional_positive_u16(&raw, "cols")?,
        },
        "terminal.write" => Action::TerminalWrite {
            terminal_id: required_non_empty_string(&raw, "terminal_id")?,
            input: raw["input"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("missing or invalid 'input' field"))?
                .to_string(),
        },
        "terminal.read" => Action::TerminalRead {
            terminal_id: required_non_empty_string(&raw, "terminal_id")?,
            max_bytes: raw["max_bytes"].as_u64(),
            flush: raw["flush"].as_bool().unwrap_or(true),
        },
        "terminal.resize" => Action::TerminalResize {
            terminal_id: required_non_empty_string(&raw, "terminal_id")?,
            rows: required_positive_u16(&raw, "rows")?,
            cols: required_positive_u16(&raw, "cols")?,
        },
        "terminal.list" => Action::TerminalList,
        "terminal.kill" => Action::TerminalKill {
            terminal_id: required_non_empty_string(&raw, "terminal_id")?,
            signal: raw["signal"].as_str().map(String::from),
        },
        "capabilities.list" => Action::CapabilitiesList,

        // Hotkeys
        "hotkeys.register" => Action::HotkeysRegister {
            hotkey_id: raw["hotkey_id"].as_str().unwrap_or("").into(),
            keys: raw["keys"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        },
        "hotkeys.unregister" => Action::HotkeysUnregister {
            hotkey_id: raw["hotkey_id"].as_str().unwrap_or("").into(),
        },

        // Audio
        "audio.list_sinks" => Action::AudioListSinks,
        "audio.set_sink_volume" => Action::AudioSetSinkVolume {
            sink_id: raw["sink_id"].as_u64().unwrap_or(0) as u32,
            volume: raw["volume"].as_f64().unwrap_or(1.0),
        },

        // Monitor
        "monitor.list" => Action::MonitorList,
        "monitor.set_primary" => Action::MonitorSetPrimary {
            output: required_non_empty_string(&raw, "output")?,
        },
        "monitor.set_resolution" => {
            let refresh_rate = match optional_positive_f64(&raw, "refresh_rate")? {
                Some(refresh_rate) => Some(refresh_rate),
                None => optional_positive_f64(&raw, "refresh")?,
            };
            Action::MonitorSetResolution {
                output: required_non_empty_string(&raw, "output")?,
                width: required_positive_u32(&raw, "width")?,
                height: required_positive_u32(&raw, "height")?,
                refresh_rate,
            }
        }
        "monitor.set_scale" => Action::MonitorSetScale {
            output: required_non_empty_string(&raw, "output")?,
            scale: required_positive_f64(&raw, "scale")?,
        },
        "monitor.set_rotation" => Action::MonitorSetRotation {
            output: required_non_empty_string(&raw, "output")?,
            rotation: required_rotation(&raw, "rotation")?,
        },
        "monitor.enable" => Action::MonitorEnable {
            output: required_non_empty_string(&raw, "output")?,
        },
        "monitor.disable" => Action::MonitorDisable {
            output: required_non_empty_string(&raw, "output")?,
        },

        // Location
        "location.get" => Action::LocationGet,
        "ui.tree.get" => Action::UiTreeGet,
        "ui.element.click" => Action::UiElementClick {
            selector: raw["selector"].as_str().unwrap_or("").into(),
        },
        "ui.element.set_text" => Action::UiElementSetText {
            selector: raw["selector"].as_str().unwrap_or("").into(),
            text: raw["text"].as_str().unwrap_or("").into(),
        },

        // Connection
        "subscribe" => Action::Subscribe {
            events: raw["events"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        },
        "unsubscribe" => Action::Unsubscribe {
            events: raw["events"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        },
        "disconnect" => Action::Disconnect,

        _ => anyhow::bail!("unknown action type: {}", msg_type),
    };

    Ok((id, action, options))
}

// Helper functions for JSON validation

fn required_non_empty_string(raw: &serde_json::Value, field: &str) -> anyhow::Result<String> {
    let value = raw[field]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing or invalid '{}' field", field))?;
    if value.trim().is_empty() {
        anyhow::bail!("'{}' must not be empty", field);
    }
    Ok(value.to_string())
}

fn optional_non_empty_string(
    raw: &serde_json::Value,
    field: &str,
) -> anyhow::Result<Option<String>> {
    let Some(value) = raw.get(field) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let value = value
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("invalid '{}' field", field))?;
    if value.trim().is_empty() {
        anyhow::bail!("'{}' must not be empty", field);
    }
    Ok(Some(value.to_string()))
}

fn required_non_empty_string_alias(
    raw: &serde_json::Value,
    primary: &str,
    alias: &str,
) -> anyhow::Result<String> {
    match optional_non_empty_string(raw, primary)? {
        Some(value) => Ok(value),
        None => required_non_empty_string(raw, alias),
    }
}

fn optional_non_empty_string_alias(
    raw: &serde_json::Value,
    primary: &str,
    alias: &str,
) -> anyhow::Result<Option<String>> {
    match optional_non_empty_string(raw, primary)? {
        Some(value) => Ok(Some(value)),
        None => optional_non_empty_string(raw, alias),
    }
}

fn required_positive_u32(raw: &serde_json::Value, field: &str) -> anyhow::Result<u32> {
    let value = raw[field]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("missing or invalid '{}' field", field))?;
    if value == 0 || value > u32::MAX as u64 {
        anyhow::bail!("'{}' must be a positive 32-bit integer", field);
    }
    Ok(value as u32)
}

fn optional_u32(raw: &serde_json::Value, field: &str) -> anyhow::Result<Option<u32>> {
    let Some(value) = raw.get(field) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let value = value
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("invalid '{}' field", field))?;
    if value > u32::MAX as u64 {
        anyhow::bail!("'{}' must fit in a 32-bit integer", field);
    }
    Ok(Some(value as u32))
}

fn required_positive_u16(raw: &serde_json::Value, field: &str) -> anyhow::Result<u16> {
    let value = raw[field]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("missing or invalid '{}' field", field))?;
    if value == 0 || value > u16::MAX as u64 {
        anyhow::bail!("'{}' must be a positive 16-bit integer", field);
    }
    Ok(value as u16)
}

fn optional_positive_u16(raw: &serde_json::Value, field: &str) -> anyhow::Result<Option<u16>> {
    let Some(value) = raw.get(field) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let value = value
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("invalid '{}' field", field))?;
    if value == 0 || value > u16::MAX as u64 {
        anyhow::bail!("'{}' must be a positive 16-bit integer", field);
    }
    Ok(Some(value as u16))
}

fn optional_priority(raw: &serde_json::Value, field: &str) -> anyhow::Result<Option<u8>> {
    let Some(value) = raw.get(field) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let value = value
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("invalid '{}' field", field))?;
    if value > 7 {
        anyhow::bail!("'{}' must be 0-7", field);
    }
    Ok(Some(value as u8))
}

fn optional_u8(raw: &serde_json::Value, field: &str) -> anyhow::Result<Option<u8>> {
    let Some(value) = raw.get(field) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let value = value
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("invalid '{}' field", field))?;
    if value > u8::MAX as u64 {
        anyhow::bail!("'{}' must fit in an 8-bit integer", field);
    }
    Ok(Some(value as u8))
}

fn required_positive_f64(raw: &serde_json::Value, field: &str) -> anyhow::Result<f64> {
    let value = raw[field]
        .as_f64()
        .ok_or_else(|| anyhow::anyhow!("missing or invalid '{}' field", field))?;
    if !value.is_finite() || value <= 0.0 {
        anyhow::bail!("'{}' must be a positive finite number", field);
    }
    Ok(value)
}

fn optional_positive_f64(raw: &serde_json::Value, field: &str) -> anyhow::Result<Option<f64>> {
    let Some(value) = raw.get(field) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let value = value
        .as_f64()
        .ok_or_else(|| anyhow::anyhow!("invalid '{}' field", field))?;
    if !value.is_finite() || value <= 0.0 {
        anyhow::bail!("'{}' must be a positive finite number", field);
    }
    Ok(Some(value))
}

fn required_rotation(raw: &serde_json::Value, field: &str) -> anyhow::Result<String> {
    let value = required_non_empty_string(raw, field)?;
    match value.as_str() {
        "normal" | "left" | "right" | "inverted" => Ok(value),
        _ => anyhow::bail!("'{}' must be one of: normal, left, right, inverted", field),
    }
}

fn optional_string_array(raw: &serde_json::Value, field: &str) -> anyhow::Result<Vec<String>> {
    let Some(value) = raw.get(field) else {
        return Ok(Vec::new());
    };
    let Some(values) = value.as_array() else {
        anyhow::bail!("'{}' must be an array of strings", field);
    };

    let mut items = Vec::with_capacity(values.len());
    for value in values {
        let Some(item) = value.as_str() else {
            anyhow::bail!("'{}' must be an array of strings", field);
        };
        if item.trim().is_empty() {
            anyhow::bail!("'{}' entries must not be empty", field);
        }
        items.push(item.to_string());
    }
    Ok(items)
}
