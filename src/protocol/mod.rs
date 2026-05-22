// reason: re-export module; logic in parse.rs/serialize.rs/exhaustive match in mod.rs
pub mod events;
pub mod parse;
pub mod serialize;
pub mod types;

// Re-export all types so external imports don't break
pub use events::*;
pub use types::*;

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
    ScreenshotOcr {
        path: Option<String>,
        language: Option<String>,
        psm: Option<u32>,
        bounding_boxes: bool,
        monitor: Option<u32>,
        region: Option<Region>,
        window_id: Option<String>,
    },
    ScreenshotDiff {
        before_path: String,
        after_path: Option<String>,
        tolerance: Option<u8>,
        diff_path: Option<String>,
        save_diff: bool,
        monitor: Option<u32>,
        region: Option<Region>,
        window_id: Option<String>,
    },

    // Audit
    AuditLog {
        limit: Option<usize>,
        action_type: Option<String>,
        status: Option<String>,
    },
    AuditClear,

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
    SystemConfinement,
    SystemRemediate {
        check: String,
        apply: bool,
    },
    SystemNormalizeCoords {
        x: f64,
        y: f64,
        monitor: Option<u32>,
    },
    WaitFor {
        condition: String,
        params: serde_json::Value,
        timeout_ms: u64,
        interval_ms: Option<u64>,
    },
    SystemIdle,
    SystemPower {
        action: String,
    },
    SystemBattery,
    SystemInhibit {
        what: String,
        who: String,
        why: Option<String>,
        mode: Option<String>,
    },
    SystemReleaseInhibit {
        inhibitor_id: u32,
    },
    SystemListSessions,
    SystemLockSession {
        session_id: Option<String>,
    },
    SystemSwitchUser {
        username: String,
    },
    SystemCheckAuth {
        action_id: String,
    },
    SystemElevate {
        action_id: String,
        reason: Option<String>,
    },

    // systemd units, journal, and timers
    ServiceStatus {
        name: String,
    },
    ServiceStart {
        name: String,
    },
    ServiceStop {
        name: String,
    },
    ServiceRestart {
        name: String,
    },
    ServiceEnable {
        name: String,
        runtime: bool,
    },
    ServiceDisable {
        name: String,
        runtime: bool,
    },
    ServiceList {
        unit_type: Option<String>,
    },
    JournalQuery {
        since: Option<u64>,
        until: Option<u64>,
        unit: Option<String>,
        priority: Option<u8>,
        tail: Option<u32>,
    },
    TimerList,
    TimerStart {
        name: String,
    },
    TimerStop {
        name: String,
    },

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

    // Terminal / PTY
    TerminalCreate {
        shell: Option<String>,
        cwd: Option<String>,
        env: Option<std::collections::HashMap<String, String>>,
        rows: Option<u16>,
        cols: Option<u16>,
    },
    TerminalWrite {
        terminal_id: String,
        input: String,
    },
    TerminalRead {
        terminal_id: String,
        max_bytes: Option<u64>,
        flush: bool,
    },
    TerminalResize {
        terminal_id: String,
        rows: u16,
        cols: u16,
    },
    TerminalList,
    TerminalKill {
        terminal_id: String,
        signal: Option<String>,
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
            "screenshot.ocr",
            "screenshot.diff",
            "audit.log",
            "audit.clear",
            "notification.send",
            "notification.close",
            "system.info",
            "system.capabilities",
            "system.health",
            "system.confinement",
            "system.remediate",
            "system.normalize_coords",
            "wait.for",
            "system.idle",
            "system.power",
            "system.battery",
            "system.inhibit",
            "system.release_inhibit",
            "system.sessions",
            "system.lock_session",
            "system.switch_user",
            "system.check_auth",
            "system.elevate",
            "service.status",
            "service.start",
            "service.stop",
            "service.restart",
            "service.enable",
            "service.disable",
            "service.list",
            "journal.query",
            "timer.list",
            "timer.start",
            "timer.stop",
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
            "terminal.create",
            "terminal.write",
            "terminal.read",
            "terminal.resize",
            "terminal.list",
            "terminal.kill",
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

    /// Parse an incoming NDJSON line into an Action plus request-level options.
    pub fn from_json_with_options(line: &str) -> anyhow::Result<(String, Action, RequestOptions)> {
        parse::from_json_with_options(line)
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
        let (_, a3) = Action::from_json(r#"{"type":"system.confinement","id":"z"}"#).unwrap();
        assert!(matches!(a1, Action::SystemCapabilities));
        assert!(matches!(a2, Action::SystemHealth));
        assert!(matches!(a3, Action::SystemConfinement));
    }

    #[test]
    fn public_actions_include_system_capabilities_and_health() {
        let actions = Action::public_action_types();
        assert!(actions.contains(&"system.capabilities"));
        assert!(actions.contains(&"system.health"));
        assert!(actions.contains(&"system.confinement"));
        assert!(actions.contains(&"windows.activate_or_launch"));
        assert!(actions.contains(&"layout_profiles.save"));
        assert!(actions.contains(&"layout_profiles.restore"));
        assert!(actions.contains(&"monitor.set_primary"));
        assert!(actions.contains(&"monitor.set_resolution"));
        assert!(actions.contains(&"monitor.disable"));
        assert!(actions.contains(&"system.inhibit"));
        assert!(actions.contains(&"system.check_auth"));
        assert!(actions.contains(&"service.restart"));
        assert!(actions.contains(&"journal.query"));
        assert!(actions.contains(&"timer.start"));
        assert!(actions.contains(&"terminal.create"));
        assert!(actions.contains(&"terminal.read"));
        assert!(actions.contains(&"wait.for"));
        assert!(actions.contains(&"screenshot.ocr"));
        assert!(actions.contains(&"screenshot.diff"));
        assert!(actions.contains(&"audit.log"));
    }

    #[test]
    fn parses_audit_actions() {
        let (_, log) = Action::from_json(
            r#"{"type":"audit.log","id":"x","limit":10,"action_type":"windows.list","status":"ok"}"#,
        )
        .unwrap();
        assert!(matches!(
            log,
            Action::AuditLog {
                limit: Some(10),
                action_type: Some(action_type),
                status: Some(status),
            } if action_type == "windows.list" && status == "ok"
        ));

        let (_, clear) = Action::from_json(r#"{"type":"audit.clear","id":"x"}"#).unwrap();
        assert!(matches!(clear, Action::AuditClear));
    }

    #[test]
    fn parses_request_options() {
        let (_, action, options) = Action::from_json_with_options(
            r#"{"type":"windows.list","id":"x","dry_run":true,"timeout_ms":250}"#,
        )
        .unwrap();
        assert!(matches!(action, Action::WindowsList));
        assert!(options.dry_run);
        assert_eq!(options.timeout_ms, Some(250));
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

    #[test]
    fn parses_systemd_and_polkit_actions() {
        let (_, inhibit) = Action::from_json(
            r#"{"type":"system.inhibit","id":"x","what":"sleep","who":"deskbrid","why":"test","mode":"block"}"#,
        )
        .unwrap();
        assert!(matches!(
            inhibit,
            Action::SystemInhibit {
                what,
                who,
                why: Some(why),
                mode: Some(mode),
            } if what == "sleep" && who == "deskbrid" && why == "test" && mode == "block"
        ));

        let (_, service) =
            Action::from_json(r#"{"type":"service.restart","id":"x","name":"ssh.service"}"#)
                .unwrap();
        assert!(matches!(
            service,
            Action::ServiceRestart { name } if name == "ssh.service"
        ));

        let (_, journal) = Action::from_json(
            r#"{"type":"journal.query","id":"x","unit":"ssh.service","priority":3,"tail":25}"#,
        )
        .unwrap();
        assert!(matches!(
            journal,
            Action::JournalQuery {
                unit: Some(unit),
                priority: Some(3),
                tail: Some(25),
                ..
            } if unit == "ssh.service"
        ));

        let (_, elevate) = Action::from_json(
            r#"{"type":"system.elevate","id":"x","action_id":"org.deskbrid.system.service-control"}"#,
        )
        .unwrap();
        assert!(matches!(
            elevate,
            Action::SystemElevate { action_id, .. }
                if action_id == "org.deskbrid.system.service-control"
        ));

        assert!(Action::from_json(r#"{"type":"journal.query","id":"x","priority":8}"#).is_err());
        assert!(Action::from_json(r#"{"type":"timer.start","id":"x","name":""}"#).is_err());
    }

    #[test]
    fn parses_terminal_actions() {
        let (_, create) = Action::from_json(
            r#"{"type":"terminal.create","id":"x","shell":"/bin/bash","cwd":"/tmp","rows":30,"cols":120}"#,
        )
        .unwrap();
        assert!(matches!(
            create,
            Action::TerminalCreate {
                shell: Some(shell),
                cwd: Some(cwd),
                rows: Some(30),
                cols: Some(120),
                ..
            } if shell == "/bin/bash" && cwd == "/tmp"
        ));

        let (_, read) = Action::from_json(
            r#"{"type":"terminal.read","id":"x","terminal_id":"term-1","max_bytes":4096,"flush":false}"#,
        )
        .unwrap();
        assert!(matches!(
            read,
            Action::TerminalRead {
                terminal_id,
                max_bytes: Some(4096),
                flush: false,
            } if terminal_id == "term-1"
        ));

        assert!(
            Action::from_json(
                r#"{"type":"terminal.resize","id":"x","terminal_id":"term-1","rows":0,"cols":80}"#
            )
            .is_err()
        );
        assert!(
            Action::from_json(
                r#"{"type":"terminal.write","id":"x","terminal_id":"","input":"ls\n"}"#
            )
            .is_err()
        );
    }

    #[test]
    fn parses_wait_for_action() {
        let (_, action) = Action::from_json(
            r#"{"type":"wait.for","id":"x","condition":"file_exists","params":{"path":"/tmp/ready"},"timeout_ms":5000,"interval_ms":100}"#,
        )
        .unwrap();
        assert!(matches!(
            action,
            Action::WaitFor {
                condition,
                timeout_ms: 5000,
                interval_ms: Some(100),
                ..
            } if condition == "file_exists"
        ));
        assert!(Action::from_json(r#"{"type":"wait.for","id":"x","condition":""}"#).is_err());
    }

    #[test]
    fn parses_screenshot_ocr_action() {
        let (_, action) = Action::from_json(
            r#"{"type":"screenshot.ocr","id":"x","path":"/tmp/s.png","language":"eng","psm":6,"bounding_boxes":true}"#,
        )
        .unwrap();
        assert!(matches!(
            action,
            Action::ScreenshotOcr {
                path: Some(path),
                language: Some(language),
                psm: Some(6),
                bounding_boxes: true,
                ..
            } if path == "/tmp/s.png" && language == "eng"
        ));
        assert!(Action::from_json(r#"{"type":"screenshot.ocr","id":"x","path":""}"#).is_err());
    }

    #[test]
    fn parses_screenshot_diff_action() {
        let (_, action) = Action::from_json(
            r#"{"type":"screenshot.diff","id":"x","before_path":"/tmp/a.png","after_path":"/tmp/b.png","tolerance":5,"save_diff":true}"#,
        )
        .unwrap();
        assert!(matches!(
            action,
            Action::ScreenshotDiff {
                before_path,
                after_path: Some(after_path),
                tolerance: Some(5),
                save_diff: true,
                ..
            } if before_path == "/tmp/a.png" && after_path == "/tmp/b.png"
        ));
        assert!(Action::from_json(r#"{"type":"screenshot.diff","id":"x"}"#).is_err());
    }

    #[test]
    fn serializes_events_with_event_field() {
        let event = crate::protocol::DeskbridEvent::WaitMatched {
            wait_id: "wait-1".into(),
            condition: "file_exists".into(),
            value: serde_json::json!({"path": "/tmp/ready"}),
            elapsed_ms: 25,
            timestamp: 123,
        };
        let value = serde_json::to_value(event).unwrap();
        assert_eq!(value["event"], "wait.matched");
        assert_eq!(value["wait_id"], "wait-1");
    }
}
