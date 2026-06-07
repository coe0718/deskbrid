use serde::{Deserialize, Serialize};

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AudioSourceInfo {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub volume: f64,
    pub muted: bool,
}

// ─── Event Types (Server → Client push) ────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "event")]
pub enum DeskbridEvent {
    #[serde(rename = "file.created")]
    FileCreated { path: String, timestamp: u64 },
    #[serde(rename = "file.modified")]
    FileModified { path: String, timestamp: u64 },
    #[serde(rename = "file.deleted")]
    FileDeleted { path: String, timestamp: u64 },
    #[serde(rename = "file.renamed")]
    FileRenamed {
        old_path: String,
        new_path: String,
        timestamp: u64,
    },
    #[serde(rename = "window.focused")]
    WindowFocused { window_id: String, timestamp: u64 },
    #[serde(rename = "window.opened")]
    WindowOpened {
        window_id: String,
        app_id: Option<String>,
        timestamp: u64,
    },
    #[serde(rename = "window.closed")]
    WindowClosed {
        window_id: String,
        app_id: Option<String>,
        timestamp: u64,
    },
    #[serde(rename = "workspace.changed")]
    WorkspaceChanged { workspace_id: u32, timestamp: u64 },
    #[serde(rename = "workspace.window_moved")]
    WorkspaceWindowMoved {
        window_id: String,
        workspace_id: u32,
        timestamp: u64,
    },
    #[serde(rename = "wait.matched")]
    WaitMatched {
        wait_id: String,
        condition: String,
        value: serde_json::Value,
        elapsed_ms: u128,
        timestamp: u64,
    },
    #[serde(rename = "screencast.frame")]
    ScreencastFrame {
        path: String,
        timestamp: u64,
        frame_number: u32,
    },
    #[serde(rename = "screencast.stopped")]
    ScreencastStopped {
        frames: u32,
        duration_secs: u64,
        output_path: Option<String>,
    },
    #[serde(rename = "update.available")]
    UpdateAvailable {
        current_version: String,
        latest_version: String,
    },
    #[serde(rename = "notification.received")]
    NotificationReceived {
        id: u32,
        app_name: String,
        title: String,
        body: Option<String>,
        urgency: String,
        actions: Option<Vec<String>>,
        timestamp: u64,
    },
    #[serde(rename = "notification.acted")]
    NotificationActed {
        id: u32,
        action_key: String,
        timestamp: u64,
    },
}
