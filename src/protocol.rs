use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

// ─── Common Types ───────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct WindowInfo {
    pub id: String,
    pub title: String,
    pub app_id: String,
    pub workspace_id: u32,
    pub is_focused: bool,
    pub is_minimized: bool,
    pub geometry: Option<Geometry>,
    pub pid: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Geometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct WorkspaceInfo {
    pub id: u32,
    pub name: String,
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemInfo {
    pub desktop: String,
    pub desktop_version: String,
    pub compositor: String,
    pub session_type: String,
    pub monitors: Vec<MonitorInfo>,
    pub workspace_count: u32,
    pub current_workspace: u32,
    pub idle_seconds: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonitorInfo {
    pub id: u32,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
    pub primary: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatteryInfo {
    pub source: String,
    pub percentage: f64,
    pub state: String,
    pub time_remaining_minutes: Option<u32>,
}

// ─── Envelope ───────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Envelope {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorBody>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

// ─── Actions (Client → Server) ──────────────────────────

#[derive(Debug, Clone)]
pub enum Action {
    Ping,

    // Windows
    WindowsList,
    WindowsFocus(String),
    WindowsGet(String),

    // Workspaces
    WorkspacesList,
    WorkspaceSwitch(u32),
    WorkspaceMoveWindow {
        window_id: String,
        workspace_id: u32,
        follow: bool,
    },

    // Input
    InputKeyboardType {
        text: String,
    },
    InputKeyboardKey {
        key: String,
    },
    InputKeyboardCombo {
        keys: Vec<String>,
    },
    InputMouse {
        action: String,
        x: Option<f64>,
        y: Option<f64>,
        button: Option<String>,
        dx: Option<f64>,
        dy: Option<f64>,
    },

    // Clipboard
    ClipboardRead,
    ClipboardWrite {
        text: String,
    },

    // Screenshot
    Screenshot {
        monitor: Option<u32>,
        region: Option<Region>,
        window_id: Option<String>,
    },

    // Notifications
    NotificationSend {
        app_name: String,
        title: String,
        body: String,
        urgency: String,
    },
    NotificationClose {
        notification_id: u32,
    },

    // System
    SystemInfo,
    SystemIdle,
    SystemPower {
        action: String,
    },
    SystemBattery,

    // Network
    NetworkStatus,
    NetworkInterfaces,
    NetworkWifiScan,
    NetworkWifiConnect {
        ssid: String,
        password: Option<String>,
    },

    // Bluetooth
    BluetoothList,
    BluetoothScan {
        duration: Option<u32>,
    },
    BluetoothStopScan,
    BluetoothConnect {
        address: String,
    },
    BluetoothDisconnect {
        address: String,
    },
    BluetoothPair {
        address: String,
    },
    BluetoothForget {
        address: String,
    },

    // Files
    FilesWatch {
        path: String,
        recursive: bool,
        patterns: Option<Vec<String>>,
    },
    FilesUnwatch {
        path: String,
    },
    FilesSearch {
        pattern: String,
        root: Option<String>,
        max_results: u32,
    },

    // Process
    ProcessList,
    ProcessStart {
        command: Vec<String>,
        workdir: Option<String>,
        env: Option<std::collections::HashMap<String, String>>,
    },

    // Hotkeys
    HotkeysRegister {
        hotkey_id: String,
        keys: Vec<String>,
    },
    HotkeysUnregister {
        hotkey_id: String,
    },

    // Audio
    AudioListSinks,
    AudioSetSinkVolume {
        sink_id: u32,
        volume: f64,
    },

    // Monitor
    MonitorList,

    // Location
    LocationGet,

    // Connection
    Subscribe {
        events: Vec<String>,
    },
    Unsubscribe {
        events: Vec<String>,
    },
    Disconnect,
}

impl Action {
    /// Parse an incoming NDJSON line into an Action.
    pub fn from_json(line: &str) -> anyhow::Result<(String, Action)> {
        let raw: serde_json::Value = serde_json::from_str(line)?;
        let msg_type = raw["type"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'type' field"))?
            .to_string();
        let id = raw["id"].as_str().unwrap_or("?").to_string();

        let action = match msg_type.as_str() {
            "ping" => Action::Ping,

            // Windows
            "windows.list" => Action::WindowsList,
            "windows.focus" => Action::WindowsFocus(raw["window_id"].as_str().unwrap_or("").into()),
            "windows.get" => Action::WindowsGet(raw["window_id"].as_str().unwrap_or("").into()),

            // Workspaces
            "workspaces.list" => Action::WorkspacesList,
            "workspaces.switch" => {
                Action::WorkspaceSwitch(raw["workspace_id"].as_u64().unwrap_or(0) as u32)
            }
            "workspaces.move_window" => Action::WorkspaceMoveWindow {
                window_id: raw["window_id"].as_str().unwrap_or("").into(),
                workspace_id: raw["workspace_id"].as_u64().unwrap_or(0) as u32,
                follow: raw["follow"].as_bool().unwrap_or(false),
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
            "system.idle" => Action::SystemIdle,
            "system.power" => Action::SystemPower {
                action: raw["action"].as_str().unwrap_or("").into(),
            },
            "system.battery" => Action::SystemBattery,

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

            // Location
            "location.get" => Action::LocationGet,

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

        Ok((id, action))
    }

    /// Convert action to a JSON envelope string
    pub fn to_json(&self) -> anyhow::Result<String> {
        let _msg_type = self.action_type();
        let id = Uuid::new_v4().to_string();
        let envelope = match self {
            Action::Ping => json!({"type": "ping", "id": id}),

            // Windows
            Action::WindowsList => json!({"type": "windows.list", "id": id}),
            Action::WindowsFocus(window_id) => {
                json!({"type": "windows.focus", "id": id, "window_id": window_id})
            }
            Action::WindowsGet(window_id) => {
                json!({"type": "windows.get", "id": id, "window_id": window_id})
            }

            // Workspaces
            Action::WorkspacesList => json!({"type": "workspaces.list", "id": id}),
            Action::WorkspaceSwitch(workspace_id) => {
                json!({"type": "workspaces.switch", "id": id, "workspace_id": workspace_id})
            }
            Action::WorkspaceMoveWindow {
                window_id,
                workspace_id,
                follow,
            } => {
                json!({"type": "workspaces.move_window", "id": id, "window_id": window_id, "workspace_id": workspace_id, "follow": follow})
            }

            // Input
            Action::InputKeyboardType { text } => {
                json!({"type": "input.keyboard", "id": id, "action": "type", "text": text})
            }
            Action::InputKeyboardKey { key } => {
                json!({"type": "input.keyboard", "id": id, "action": "key", "key": key})
            }
            Action::InputKeyboardCombo { keys } => {
                json!({"type": "input.keyboard", "id": id, "action": "combo", "keys": keys})
            }
            Action::InputMouse {
                action,
                x,
                y,
                button,
                dx,
                dy,
            } => {
                let mut obj = json!({"type": "input.mouse", "id": id, "action": action});
                if let Some(x) = x {
                    obj["x"] = json!(x);
                }
                if let Some(y) = y {
                    obj["y"] = json!(y);
                }
                if let Some(button) = button {
                    obj["button"] = json!(button);
                }
                if let Some(dx) = dx {
                    obj["dx"] = json!(dx);
                }
                if let Some(dy) = dy {
                    obj["dy"] = json!(dy);
                }
                obj
            }

            // Clipboard
            Action::ClipboardRead => json!({"type": "clipboard.read", "id": id}),
            Action::ClipboardWrite { text } => {
                json!({"type": "clipboard.write", "id": id, "text": text})
            }

            // Screenshot
            Action::Screenshot {
                monitor,
                region,
                window_id,
            } => {
                let mut obj = json!({"type": "screenshot", "id": id});
                if let Some(m) = monitor {
                    obj["monitor"] = json!(m);
                }
                if let Some(r) = region {
                    obj["region"] = json!(r);
                }
                if let Some(w) = window_id {
                    obj["window_id"] = json!(w);
                }
                obj
            }

            // Notifications
            Action::NotificationSend {
                app_name,
                title,
                body,
                urgency,
            } => {
                json!({"type": "notification.send", "id": id, "app_name": app_name, "title": title, "body": body, "urgency": urgency})
            }
            Action::NotificationClose { notification_id } => {
                json!({"type": "notification.close", "id": id, "notification_id": notification_id})
            }

            // System
            Action::SystemInfo => json!({"type": "system.info", "id": id}),
            Action::SystemIdle => json!({"type": "system.idle", "id": id}),
            Action::SystemPower { action } => {
                json!({"type": "system.power", "id": id, "action": action})
            }
            Action::SystemBattery => json!({"type": "system.battery", "id": id}),

            // Network
            Action::NetworkStatus => json!({"type": "network.status", "id": id}),
            Action::NetworkInterfaces => json!({"type": "network.interfaces", "id": id}),
            Action::NetworkWifiScan => json!({"type": "network.wifi.scan", "id": id}),
            Action::NetworkWifiConnect { ssid, password } => {
                let mut obj = json!({"type": "network.wifi.connect", "id": id, "ssid": ssid});
                if let Some(pw) = password {
                    obj["password"] = json!(pw);
                }
                obj
            }

            // Bluetooth
            Action::BluetoothList => json!({"type": "bluetooth.list", "id": id}),
            Action::BluetoothScan { duration } => {
                let mut obj = json!({"type": "bluetooth.scan", "id": id});
                if let Some(d) = duration {
                    obj["duration"] = json!(d);
                }
                obj
            }
            Action::BluetoothStopScan => json!({"type": "bluetooth.scan_stop", "id": id}),
            Action::BluetoothConnect { address } => {
                json!({"type": "bluetooth.connect", "id": id, "address": address})
            }
            Action::BluetoothDisconnect { address } => {
                json!({"type": "bluetooth.disconnect", "id": id, "address": address})
            }
            Action::BluetoothPair { address } => {
                json!({"type": "bluetooth.pair", "id": id, "address": address})
            }
            Action::BluetoothForget { address } => {
                json!({"type": "bluetooth.forget", "id": id, "address": address})
            }

            // Files
            Action::FilesWatch {
                path,
                recursive,
                patterns,
            } => {
                let mut obj =
                    json!({"type": "files.watch", "id": id, "path": path, "recursive": recursive});
                if let Some(p) = patterns {
                    obj["patterns"] = json!(p);
                }
                obj
            }
            Action::FilesUnwatch { path } => {
                json!({"type": "files.unwatch", "id": id, "path": path})
            }
            Action::FilesSearch {
                pattern,
                root,
                max_results,
            } => {
                let mut obj = json!({"type": "files.search", "id": id, "pattern": pattern, "max_results": max_results});
                if let Some(r) = root {
                    obj["root"] = json!(r);
                }
                obj
            }

            // Process
            Action::ProcessList => json!({"type": "process.list", "id": id}),
            Action::ProcessStart {
                command,
                workdir,
                env,
            } => {
                let mut obj = json!({"type": "process.start", "id": id, "command": command});
                if let Some(wd) = workdir {
                    obj["workdir"] = json!(wd);
                }
                if let Some(e) = env {
                    obj["env"] = json!(e);
                }
                obj
            }

            // Hotkeys
            Action::HotkeysRegister { hotkey_id, keys } => {
                json!({"type": "hotkeys.register", "id": id, "hotkey_id": hotkey_id, "keys": keys})
            }
            Action::HotkeysUnregister { hotkey_id } => {
                json!({"type": "hotkeys.unregister", "id": id, "hotkey_id": hotkey_id})
            }

            // Audio
            Action::AudioListSinks => json!({"type": "audio.list_sinks", "id": id}),
            Action::AudioSetSinkVolume { sink_id, volume } => {
                json!({"type": "audio.set_sink_volume", "id": id, "sink_id": sink_id, "volume": volume})
            }

            // Monitor
            Action::MonitorList => json!({"type": "monitor.list", "id": id}),

            // Location
            Action::LocationGet => json!({"type": "location.get", "id": id}),

            // Connection
            Action::Subscribe { events } => {
                json!({"type": "subscribe", "id": id, "events": events})
            }
            Action::Unsubscribe { events } => {
                json!({"type": "unsubscribe", "id": id, "events": events})
            }
            Action::Disconnect => json!({"type": "disconnect", "id": id}),
        };

        Ok(serde_json::to_string(&envelope)?)
    }

    fn action_type(&self) -> &'static str {
        match self {
            Action::Ping => "ping",
            Action::WindowsList => "windows.list",
            Action::WindowsFocus(_) => "windows.focus",
            Action::WindowsGet(_) => "windows.get",
            Action::WorkspacesList => "workspaces.list",
            Action::WorkspaceSwitch(_) => "workspaces.switch",
            Action::WorkspaceMoveWindow { .. } => "workspaces.move_window",
            Action::InputKeyboardType { .. } => "input.keyboard",
            Action::InputKeyboardKey { .. } => "input.keyboard",
            Action::InputKeyboardCombo { .. } => "input.keyboard",
            Action::InputMouse { .. } => "input.mouse",
            Action::ClipboardRead => "clipboard.read",
            Action::ClipboardWrite { .. } => "clipboard.write",
            Action::Screenshot { .. } => "screenshot",
            Action::NotificationSend { .. } => "notification.send",
            Action::NotificationClose { .. } => "notification.close",
            Action::SystemInfo => "system.info",
            Action::SystemIdle => "system.idle",
            Action::SystemPower { .. } => "system.power",
            Action::SystemBattery => "system.battery",
            Action::NetworkStatus => "network.status",
            Action::NetworkInterfaces => "network.interfaces",
            Action::NetworkWifiScan => "network.wifi.scan",
            Action::NetworkWifiConnect { .. } => "network.wifi.connect",
            Action::BluetoothList => "bluetooth.list",
            Action::BluetoothScan { .. } => "bluetooth.scan",
            Action::BluetoothStopScan => "bluetooth.scan_stop",
            Action::BluetoothConnect { .. } => "bluetooth.connect",
            Action::BluetoothDisconnect { .. } => "bluetooth.disconnect",
            Action::BluetoothPair { .. } => "bluetooth.pair",
            Action::BluetoothForget { .. } => "bluetooth.forget",
            Action::FilesWatch { .. } => "files.watch",
            Action::FilesUnwatch { .. } => "files.unwatch",
            Action::FilesSearch { .. } => "files.search",
            Action::ProcessList => "process.list",
            Action::ProcessStart { .. } => "process.start",
            Action::HotkeysRegister { .. } => "hotkeys.register",
            Action::HotkeysUnregister { .. } => "hotkeys.unregister",
            Action::AudioListSinks => "audio.list_sinks",
            Action::AudioSetSinkVolume { .. } => "audio.set_sink_volume",
            Action::MonitorList => "monitor.list",
            Action::LocationGet => "location.get",
            Action::Subscribe { .. } => "subscribe",
            Action::Unsubscribe { .. } => "unsubscribe",
            Action::Disconnect => "disconnect",
        }
    }
}

// ─── Event Data Types (for subscription events) ─────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScreenshotResult {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub format: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkStatusInfo {
    pub online: bool,
    #[serde(rename = "type")]
    pub net_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkInterfaceInfo {
    pub name: String,
    pub state: String,
    pub ipv4: Option<String>,
    pub ipv6: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WifiNetworkInfo {
    pub ssid: String,
    pub strength: u32,
    pub secured: bool,
    pub frequency: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BluetoothDeviceInfo {
    pub address: String,
    pub name: String,
    pub paired: bool,
    pub connected: bool,
    pub rssi: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AudioSinkInfo {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub volume: f64,
    pub muted: bool,
}
