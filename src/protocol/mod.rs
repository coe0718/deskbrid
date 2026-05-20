

pub mod types;
pub mod events;
pub mod parse;
pub mod serialize;

// Re-export all types so external imports don't break
pub use types::*;
pub use events::*;

// reason: 90+ Action enum variants — cannot reduce without breaking exhaustiveness
#[derive(Debug, Clone)]
pub enum Action {
    Ping,

    // Windows
    WindowsList,
    WindowsFocus(String),
    WindowsGet(String),
    WindowsClose(String),
    WindowsMinimize(String),
    WindowsMaximize(String),
    WindowsMoveResize {
        window_id: String,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    },
    WindowsActivateOrLaunch {
        app_id: String,
        command: Vec<String>,
        workdir: Option<String>,
        env: Option<std::collections::HashMap<String, String>>,
    },

    // Workspaces
    WorkspacesList,
    WorkspaceSwitch(u32),
    WorkspaceMoveWindow {
        window_id: String,
        workspace_id: u32,
        follow: bool,
    },

    // Layout profiles
    LayoutProfilesList,
    LayoutProfileGet {
        name: String,
    },
    LayoutProfileSave {
        name: String,
        overwrite: bool,
    },
    LayoutProfileDelete {
        name: String,
    },
    LayoutProfileRestore {
        name: String,
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
    SystemCapabilities,
    SystemHealth,
    SystemRemediate {
        check: String,
        apply: bool,
    },
    SystemNormalizeCoords {
        x: f64,
        y: f64,
        monitor: Option<u32>,
    },
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
    FilesRead {
        path: String,
        offset: Option<u64>,
        limit: Option<u64>,
    },
    FilesWrite {
        path: String,
        content: String,
        append: bool,
    },
    FilesCopy {
        source: String,
        destination: String,
    },
    FilesMove {
        source: String,
        destination: String,
    },
    FilesDelete {
        path: String,
        recursive: bool,
    },
    FilesMkdir {
        path: String,
        parents: bool,
    },
    FilesList {
        path: String,
    },

    // Browser (Chrome DevTools Protocol)
    BrowserListTabs,
    BrowserNavigate {
        tab_index: Option<u32>,
        url: String,
    },
    BrowserEvaluate {
        tab_index: Option<u32>,
        expression: String,
        await_promise: bool,
    },
    BrowserScreenshotTab {
        tab_index: Option<u32>,
    },
    BrowserClick {
        tab_index: Option<u32>,
        selector: String,
    },

    // Accessibility (AT-SPI2)
    A11yTree {
        depth: Option<u32>,
    },
    A11yGetElement {
        role: Option<String>,
        name: Option<String>,
        index: Option<u32>,
    },
    A11yClickElement {
        role: Option<String>,
        name: Option<String>,
        index: Option<u32>,
    },
    A11yGetText {
        role: Option<String>,
        name: Option<String>,
        index: Option<u32>,
    },

    // Process
    ProcessList,
    ProcessStart {
        command: Vec<String>,
        workdir: Option<String>,
        env: Option<std::collections::HashMap<String, String>>,
    },
    ProcessStop {
        pid: u32,
        signal: Option<String>,
    },
    ProcessSignal {
        pid: u32,
        signal: String,
    },
    ProcessExists {
        pid: u32,
    },
    ProcessWait {
        pid: u32,
        timeout_ms: Option<u64>,
    },
    CapabilitiesList,

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
    MonitorSetPrimary {
        output: String,
    },
    MonitorSetResolution {
        output: String,
        width: u32,
        height: u32,
        refresh_rate: Option<f64>,
    },
    MonitorSetScale {
        output: String,
        scale: f64,
    },
    MonitorSetRotation {
        output: String,
        rotation: String,
    },
    MonitorEnable {
        output: String,
    },
    MonitorDisable {
        output: String,
    },

    // Location
    LocationGet,
    UiTreeGet,
    UiElementClick {
        selector: String,
    },
    UiElementSetText {
        selector: String,
        text: String,
    },

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
    pub fn public_action_types() -> &'static [&'static str] {
        &[
            "windows.list",
            "windows.focus",
            "windows.get",
            "windows.close",
            "windows.minimize",
            "windows.maximize",
            "windows.move_resize",
            "windows.activate_or_launch",
            "workspaces.list",
            "workspaces.switch",
            "workspaces.move_window",
            "layout_profiles.list",
            "layout_profiles.get",
            "layout_profiles.save",
            "layout_profiles.delete",
            "layout_profiles.restore",
            "input.keyboard",
            "input.mouse",
            "clipboard.read",
            "clipboard.write",
            "screenshot",
            "notification.send",
            "notification.close",
            "system.info",
            "system.capabilities",
            "system.health",
            "system.remediate",
            "system.normalize_coords",
            "system.idle",
            "system.power",
            "system.battery",
            "network.status",
            "network.interfaces",
            "network.wifi.scan",
            "network.wifi.connect",
            "bluetooth.list",
            "bluetooth.scan",
            "bluetooth.scan_stop",
            "bluetooth.connect",
            "bluetooth.disconnect",
            "bluetooth.pair",
            "bluetooth.forget",
            "files.watch",
            "files.unwatch",
            "files.search",
            "files.read",
            "files.write",
            "files.copy",
            "files.move",
            "files.delete",
            "files.mkdir",
            "files.list",
            "browser.list_tabs",
            "browser.navigate",
            "browser.evaluate",
            "browser.screenshot_tab",
            "browser.click",
            "a11y.tree",
            "a11y.get_element",
            "a11y.click_element",
            "a11y.get_text",
            "process.list",
            "process.start",
            "process.stop",
            "process.signal",
            "process.exists",
            "process.wait",
            "hotkeys.register",
            "hotkeys.unregister",
            "audio.list_sinks",
            "audio.set_sink_volume",
            "monitor.list",
            "monitor.set_primary",
            "monitor.set_resolution",
            "monitor.set_scale",
            "monitor.set_rotation",
            "monitor.enable",
            "monitor.disable",
            "location.get",
            "ui.tree.get",
            "ui.element.click",
            "ui.element.set_text",
            "capabilities.list",
        ]
    }



    /// Parse an incoming NDJSON line into an Action.
    pub fn from_json(line: &str) -> anyhow::Result<(String, Action)> {
        parse::from_json(line)
    }

    /// Convert action to a JSON envelope string.
    pub fn to_json(&self) -> anyhow::Result<String> {
        serialize::to_json(self)
    }

    /// Get the action type string (e.g. "windows.list").
    pub fn action_type(&self) -> &'static str {
        serialize::action_type(self)
    }
}

#[cfg(test)]
mod tests {
    use super::Action;

    #[test]
    fn parses_system_capabilities_and_health() {
        let (_, a1) = Action::from_json(r#"{"type":"system.capabilities","id":"x"}"#).unwrap();
        let (_, a2) = Action::from_json(r#"{"type":"system.health","id":"y"}"#).unwrap();
        assert!(matches!(a1, Action::SystemCapabilities));
        assert!(matches!(a2, Action::SystemHealth));
    }

    #[test]
    fn public_actions_include_system_capabilities_and_health() {
        let actions = Action::public_action_types();
        assert!(actions.contains(&"system.capabilities"));
        assert!(actions.contains(&"system.health"));
        assert!(actions.contains(&"windows.activate_or_launch"));
        assert!(actions.contains(&"layout_profiles.save"));
        assert!(actions.contains(&"layout_profiles.restore"));
        assert!(actions.contains(&"monitor.set_primary"));
        assert!(actions.contains(&"monitor.set_resolution"));
        assert!(actions.contains(&"monitor.disable"));
    }

    #[test]
    fn rejects_empty_window_ids() {
        assert!(Action::from_json(r#"{"type":"windows.close","id":"x"}"#).is_err());
        assert!(Action::from_json(r#"{"type":"windows.close","id":"x","window_id":""}"#).is_err());
        assert!(
            Action::from_json(r#"{"type":"windows.move_resize","id":"x","window_id":" ","x":0,"y":0,"width":1,"height":1}"#)
                .is_err()
        );
    }

    #[test]
    fn parses_windows_activate_or_launch() {
        let (_, action) = Action::from_json(
            r#"{"type":"windows.activate_or_launch","id":"x","app_id":"code","command":["code","."]}"#,
        )
        .unwrap();
        assert!(matches!(
            action,
            Action::WindowsActivateOrLaunch {
                app_id,
                command,
                ..
            } if app_id == "code" && command == vec!["code".to_string(), ".".to_string()]
        ));
        assert!(
            Action::from_json(r#"{"type":"windows.activate_or_launch","id":"x","app_id":""}"#)
                .is_err()
        );
        assert!(
            Action::from_json(
                r#"{"type":"windows.activate_or_launch","id":"x","app_id":"code","command":[""]}"#
            )
            .is_err()
        );
    }

    #[test]
    fn parses_layout_profile_actions() {
        let (_, save) = Action::from_json(
            r#"{"type":"layout_profiles.save","id":"x","name":"coding","overwrite":true}"#,
        )
        .unwrap();
        assert!(matches!(
            save,
            Action::LayoutProfileSave {
                name,
                overwrite: true
            } if name == "coding"
        ));

        let (_, restore) =
            Action::from_json(r#"{"type":"layout_profiles.restore","id":"x","name":"coding"}"#)
                .unwrap();
        assert!(matches!(
            restore,
            Action::LayoutProfileRestore { name } if name == "coding"
        ));
        assert!(
            Action::from_json(r#"{"type":"layout_profiles.save","id":"x","name":""}"#).is_err()
        );
    }

    #[test]
    fn parses_monitor_control_actions() {
        let (_, resolution) = Action::from_json(
            r#"{"type":"monitor.set_resolution","id":"x","output":"DP-1","width":2560,"height":1440,"refresh_rate":144}"#,
        )
        .unwrap();
        assert!(matches!(
            resolution,
            Action::MonitorSetResolution {
                output,
                width: 2560,
                height: 1440,
                refresh_rate: Some(144.0),
            } if output == "DP-1"
        ));

        let (_, rotation) = Action::from_json(
            r#"{"type":"monitor.set_rotation","id":"x","output":"eDP-1","rotation":"left"}"#,
        )
        .unwrap();
        assert!(matches!(
            rotation,
            Action::MonitorSetRotation { output, rotation }
                if output == "eDP-1" && rotation == "left"
        ));

        assert!(
            Action::from_json(r#"{"type":"monitor.set_scale","id":"x","output":"DP-1","scale":0}"#)
                .is_err()
        );
        assert!(
            Action::from_json(
                r#"{"type":"monitor.set_rotation","id":"x","output":"DP-1","rotation":"sideways"}"#
            )
            .is_err()
        );
        assert!(Action::from_json(r#"{"type":"monitor.disable","id":"x","output":""}"#).is_err());
    }
}

