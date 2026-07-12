use super::rules_types::{EventTrigger, RuleCondition};
use super::types::Region;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VisionStateCheck {
    pub kind: String,
    pub expected: Option<serde_json::Value>,
    pub region: Option<Region>,
    pub template_path: Option<String>,
    pub min_confidence: Option<f64>,
}

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
    WindowsTile {
        window_id: String,
        preset: String,
        monitor: Option<u32>,
        padding: Option<u32>,
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
    InputMouseDrag {
        from_x: f64,
        from_y: f64,
        to_x: f64,
        to_y: f64,
        button: Option<String>,
        duration_ms: Option<u64>,
    },
    InputListLayouts,
    InputGetLayout,
    InputSetLayout {
        index: Option<u32>,
        name: Option<String>,
        variant: Option<String>,
    },
    InputAddLayout {
        name: String,
        variant: Option<String>,
    },
    InputRemoveLayout {
        index: u32,
    },

    // Clipboard
    ClipboardRead,
    ClipboardWrite {
        text: String,
    },
    ClipboardHistoryList {
        limit: Option<usize>,
        query: Option<String>,
    },
    ClipboardHistoryClear,

    // Apps
    AppList {
        categories: Vec<String>,
        mime_types: Vec<String>,
        include_hidden: bool,
        limit: Option<usize>,
    },
    AppSearch {
        query: String,
        limit: Option<usize>,
    },
    AppGet {
        app_id: String,
    },

    // MPRIS media control
    MprisList,
    MprisGet {
        player: Option<String>,
    },
    MprisControl {
        player: Option<String>,
        action: String,
    },

    // Color picker
    ColorPick {
        x: u32,
        y: u32,
        path: Option<String>,
    },

    // Screenshot
    Screenshot {
        monitor: Option<u32>,
        region: Option<Region>,
        window_id: Option<String>,
        output: Option<String>,
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
    /// Find UI element(s) by visual template matching.
    VisionFindElement {
        template_path: String,
        screenshot: Option<String>,
        min_confidence: Option<f64>,
        max_results: Option<u32>,
    },
    /// Find element by text label (hybrid OCR + position).
    VisionFindByText {
        text: String,
        screenshot: Option<String>,
    },
    /// Detect UI state via multiple visual checks.
    VisionDetectState {
        screenshot: Option<String>,
        checks: Vec<VisionStateCheck>,
    },
    RegionWatchCreate {
        name: String,
        monitor: Option<u32>,
        region: Region,
        interval_ms: Option<u64>,
        change_threshold_pct: Option<f64>,
        notify_on_change: bool,
        notify_on_stable: bool,
        stable_duration_ms: Option<u64>,
        auto_save: Option<String>,
        max_changes: Option<u32>,
        tolerance: Option<u8>,
    },
    RegionWatchUpdate {
        name: String,
        monitor: Option<u32>,
        region: Option<Region>,
        interval_ms: Option<u64>,
        change_threshold_pct: Option<f64>,
        notify_on_change: Option<bool>,
        notify_on_stable: Option<bool>,
        stable_duration_ms: Option<u64>,
        auto_save: Option<String>,
        max_changes: Option<u32>,
        tolerance: Option<u8>,
    },
    RegionWatchRemove {
        name: String,
    },
    RegionWatchList,
    TextWatchCreate {
        name: String,
        monitor: Option<u32>,
        region: Region,
        interval_ms: Option<u64>,
        language: Option<String>,
        notify_on_change: bool,
        notify_on_match: Option<String>,
        notify_on_mismatch: Option<String>,
        max_entries: Option<u32>,
        psm: Option<u32>,
    },
    TextWatchRemove {
        name: String,
    },
    TextWatchList,

    // Screencast (video recording)
    ScreencastStart {
        output_path: String,
    },
    ScreencastStop,

    // Desktop Portal
    PortalScreenshot {
        interactive: bool,
    },
    PortalScreencastStart {
        output_path: String,
    },
    PortalScreencastStop,

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
    NotificationHistory {
        limit: Option<u32>,
        app_name: Option<String>,
        since: Option<u64>,
    },
    NotificationAction {
        notification_id: u32,
        action_key: String,
    },
    NotificationClearHistory,
    NotificationWatch,

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
    /// Get current user presence state (active, idle, sleep).
    /// Returns {"state": "active"|"idle"|"sleep", "idle_seconds": u64}
    /// Also available as push event via subscribe "presence.*"
    PresenceGet,
    /// Read/update presence-monitor thresholds. With no args: returns current
    /// config. With `idle_threshold_secs` / `away_threshold_secs`: updates those
    /// thresholds (None preserves the existing value). Returns the new config.
    PresenceConfig {
        idle_threshold_secs: Option<u64>,
        away_threshold_secs: Option<u64>,
    },
    /// Get current time-of-day info (local time, timezone, UTC offset, sunrise/sunset if lat/lon configured).
    TimeOfDay,
    /// Read/update time-of-day config (latitude/longitude for solar times, display format).
    TimeOfDayConfig {
        latitude: Option<f64>,
        longitude: Option<f64>,
        format_24h: Option<bool>,
    },
    /// List available power profiles ("performance", "balanced", "power-saver", etc.)
    /// Reported by power-profiles-daemon via `org.freedesktop.UPower.PowerProfiles`.
    /// Returns: {"profiles": ["performance", "balanced", "power-saver"]}
    PowerProfileList,
    /// Get the active power profile + available profiles.
    /// Returns: {"active": "balanced", "profiles": ["performance", "balanced", "power-saver"]}
    /// `active` is null if power-profiles-daemon isn't running on this machine.
    PowerProfileGet,
    /// Switch the active power profile. `profile` must be one of the values
    /// reported by `power.profile.list`.
    /// Returns: {"active": "performance", "previous": "balanced"}
    PowerProfileSet {
        profile: String,
    },
    SystemPower {
        action: String,
    },
    SystemBattery,
    /// Read current battery charge threshold settings (start/end %).
    /// Returns: {"start": 0, "end": 80, "supported": true, "vendor": "Lenovo", "battery": "BAT0"}
    /// `supported: false` when the daemon can't find a writable threshold
    /// sysfs node (desktop, non-threshold-supporting laptop, etc.).
    BatteryThresholdGet,
    /// Set battery charge start/end thresholds. `start` and `end` are 0-100.
    /// Either is optional. The values are validated against the kernel's
    /// monotonic constraint (start <= end) and the device's accepted range.
    /// `profile` is optional: "daily" → (50, 80), "travel" → (90, 100), "full" → (0, 100).
    /// Returns: same as `Get` after applying.
    BatteryThresholdSet {
        start: Option<u32>,
        end: Option<u32>,
        profile: Option<String>,
    },
    SystemBacklightList,
    SystemBacklightGet {
        device: Option<String>,
    },
    SystemBacklightSet {
        device: Option<String>,
        value: String,
    },
    /// Read current locale settings (lang + all LC_* vars).
    /// Returns: {"lang":"en_US.UTF-8","lc_time":"en_DK.UTF-8","lc_numeric":...,
    ///          "source":"process"|"/etc/locale.conf", "available":[...]}
    /// `source` indicates where the values came from: process env or the
    /// persistent config file. `available` lists well-known keys we check.
    LocaleGet,
    /// Set one or more LC_* / LANG values persistently in /etc/locale.conf.
    /// On most modern systems this requires root; non-root callers get a
    /// clear error. Persistent takes effect on next shell / login session.
    /// Returns: {"written": {"LANG": "en_US.UTF-8"}, "source": "/etc/locale.conf",
    ///          "requires_root": true}
    LocaleSet {
        vars: Vec<(String, String)>,
    },
    /// Read current timezone. Returns: {"timezone": "America/Indiana/Indianapolis",
    ///          "utc_offset_minutes": -240, "dst_active": true,
    ///          "is_utc": false, "symlink": "/usr/share/zoneinfo/..."}
    /// Timezone is read from /etc/localtime (resolved via realpath) and
    /// cross-checked against /etc/timezone when present.
    TimezoneGet,
    /// Set timezone by writing /etc/timezone and replacing the /etc/localtime
    /// symlink. Validates against /usr/share/zoneinfo/{name}. Requires root.
    /// Returns: {"timezone": "...", "previous": "...", "symlink_target": "...",
    ///          "requires_root": true}
    TimezoneSet {
        timezone: String,
    },
    SystemPrintList,
    SystemPrintDefault {
        printer: Option<String>,
    },
    SystemPrintFile {
        printer: String,
        path: String,
    },
    SystemPrintJobList,
    SystemPrintJobCancel {
        job_id: String,
    },
    SystemPrintJobPause {
        job_id: String,
    },
    SystemPrintJobResume {
        job_id: String,
    },
    SystemPressure,
    SystemThermalGet,
    SystemCpuFrequency,
    SystemCpuGovernor,
    SystemCpuSetGovernor {
        governor: String,
    },
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
    SystemUpdate {
        check: bool,
        force: bool,
    },

    // D-Bus
    DbusCall {
        bus: Option<String>,
        service: String,
        path: String,
        interface: String,
        method: String,
        args: Option<serde_json::Value>,
    },

    // Schedule
    ScheduleList,
    ScheduleAdd {
        name: String,
        interval_secs: u64,
        action_type: String,
        action_params: Option<serde_json::Value>,
    },
    ScheduleRemove {
        name: String,
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
    NetworkConnectionList,
    NetworkConnectionProfiles,
    NetworkCreateHotspot {
        ssid: String,
        password: Option<String>,
    },
    NetworkStopHotspot,
    NetworkWifiEnable {
        enabled: bool,
    },
    NetworkWwanEnable {
        enabled: bool,
    },
    NetworkDnsSet {
        dns: Vec<String>,
    },
    NetworkDnsReset,
    NetworkVpnConnect {
        profile_name: String,
    },
    NetworkVpnDisconnect,

    // Clients
    ClientsList,

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

    // Accessibility (AT-SPI2) — expanded computer-use-linux compatible
    A11ySnapshotTree {
        app_name: Option<String>,
        pid: Option<u32>,
        max_nodes: Option<usize>,
        max_depth: Option<u32>,
    },
    A11yPerformAction {
        object_ref: String,
        action_name: Option<String>,
    },
    A11ySetValue {
        object_ref: String,
        value: String,
    },
    A11yGetElementText {
        object_ref: String,
        max_chars: Option<i32>,
    },
    A11yListApps {
        limit: Option<usize>,
    },
    A11yDoctor,
    A11ySetupAccessibility,
    A11yClickElementByRef {
        object_ref: String,
    },

    // Desktop Settings
    DesktopGetSetting {
        schema: String,
        key: String,
    },
    DesktopSetSetting {
        schema: String,
        key: String,
        value: String,
    },
    DesktopListSchemas,

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
    AudioListSources,
    AudioGetVolume {
        target: String, // "sink" or "source"
        id: u32,
    },
    AudioSetVolume {
        target: String,
        id: u32,
        volume: f64,
    },
    AudioMute {
        target: String,
        id: u32,
        mute: bool,
    },
    AudioSetDefault {
        target: String, // "sink" or "source"
        name: String,
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
        tab_index: Option<u32>,
    },
    UiElementSetText {
        selector: String,
        text: String,
        tab_index: Option<u32>,
    },

    // Connection
    Subscribe {
        events: Vec<String>,
    },
    Unsubscribe {
        events: Vec<String>,
    },
    Disconnect,

    // ─── Macro Recording & Replay ───────────────────────
    MacroRecordStart {
        name: String,
        description: Option<String>,
    },
    MacroRecordStop,
    MacroReplay {
        name: String,
        mode: Option<String>,
        loop_count: Option<u32>,
        stop_on_error: Option<bool>,
    },
    MacroList,
    MacroGet {
        name: String,
    },
    MacroDelete {
        name: String,
    },
    MacroExport {
        name: String,
    },
    MacroImport {
        name: String,
        data: String,
    },

    // ─── Named Sessions (#31) ──────────────────────────
    SessionCreate {
        name: String,
        clone_from: Option<String>,
        profile: Option<String>,
    },
    SessionDestroy {
        name: String,
    },
    SessionList,
    SessionSwitch {
        name: String,
    },
    /// Get one or all environment variables from this process.
    /// If `name` is provided, returns just that var. Otherwise returns
    /// a serialized map of the whole environment. Values are strings;
    /// multi-line or non-UTF8 values are returned as base64 strings.
    /// Returns: {"name": "PATH", "value": "...", "found": true}
    /// or for `name` absent: {"vars": {"PATH": "...", ...}, "count": 87}
    /// This reads from `std::env` of the running daemon — which is also
    /// the env that any spawned child process inherits.
    EnvGet {
        name: Option<String>,
    },
    /// Set an environment variable in the daemon's process environment.
    /// Children spawned afterwards will see the new value; the daemon itself
    /// and already-running children will not (Linux limitation).
    /// Returns: {"name": "EDITOR", "previous": "vi", "value": "nvim", "set": true}
    /// `name` and `value` are required; on validation failure or unset
    /// errors (e.g. name contains `=`), returns a clean error.
    EnvSet {
        name: String,
        value: String,
    },
    /// Persist one or more env vars to `~/.config/environment.d/deskbrid.conf`.
    /// This is the systemd user-session standard: shells and apps launched
    /// after the next login inherit the values. Vars are validated the same
    /// way as `env.set` (no `=` in name, non-empty).
    /// Returns: {"written":{"LANG":"en_US.UTF-8"},"source":"...","preserved":N}
    EnvPersist {
        vars: Vec<(String, String)>,
    },
    /// Remove one or more persisted env vars from the deskbrid config file.
    /// Missing vars are reported but do not error.
    /// Returns: {"removed":["FOO","BAR"],"not_found":["BAZ"],"source":"..."}
    EnvUnset {
        names: Vec<String>,
    },
    /// List all persisted env vars from the deskbrid config file.
    /// Returns: {"vars":{"FOO":"bar"},"count":N,"source":"...","exists":true}
    EnvListPersisted,
    SessionSuspend {
        name: String,
        reason: Option<String>,
    },
    SessionResume {
        name: String,
    },
    SessionVarSet {
        name: String,
        value: String,
    },
    SessionVarGet {
        name: String,
    },
    SessionVarList,

    // ─── Rules Engine (#83) ──────────────────────────────
    RuleCreate {
        name: String,
        trigger: EventTrigger,
        condition: Option<RuleCondition>,
        action_type: String,
        action_params: serde_json::Value,
        enabled: bool,
        max_fires: Option<u32>,
        cooldown_ms: Option<u64>,
    },
    RuleList,
    RuleGet {
        rule_id: String,
    },
    RuleDelete {
        rule_id: String,
    },
    RulePause {
        rule_id: String,
    },
    RuleResume {
        rule_id: String,
    },

    // ─── Blackboard (#84) ────────────────────────────────
    BlackboardSet {
        key: String,
        value: String,
        namespace: Option<String>,
    },
    BlackboardGet {
        key: String,
        namespace: Option<String>,
    },
    BlackboardDelete {
        key: String,
        namespace: Option<String>,
    },
    BlackboardList {
        namespace: Option<String>,
    },

    // ─── Secret/Keyring Access (#29) ──────────────────────
    SecretsListCollections,
    SecretsGetSecret {
        attributes: std::collections::HashMap<String, String>,
    },
    SecretsStoreSecret {
        attributes: std::collections::HashMap<String, String>,
        secret: String,
        label: Option<String>,
        collection: Option<String>,
    },

    // ─── Action Confirmation (#37) ───────────────────────
    ConfirmAction {
        id: String,
    },
    DenyAction {
        id: String,
    },
    ConfirmationList,

    // ─── Agent-to-Agent Messaging (#44) ──────────────────
    AgentMessage {
        to_session: String,
        subject: String,
        body: serde_json::Value,
        ttl_ms: Option<u64>,
        reply_to: Option<String>,
    },
    AgentBroadcast {
        subject: String,
        body: serde_json::Value,
        exclude_self: Option<bool>,
    },
    AgentMailbox,
    AgentRegister {
        name: String,
        agent_type: Option<String>,
        capabilities: Vec<String>,
        metadata: Option<serde_json::Value>,
        heartbeat_interval_ms: Option<u64>,
    },
    AgentList,
    AgentGet {
        name: String,
    },
    AgentHeartbeat {
        name: String,
    },

    // ─── Lock / Mutex Primitives (#46) ───────────────────
    LockAcquire {
        resource: String,
        holder: Option<String>,
        ttl_ms: Option<u64>,
        wait_ms: Option<u64>,
        force: bool,
    },
    LockRelease {
        resource: String,
        token: String,
    },
    LockList,

    // ─── Unified Search (#80) ────────────────────────────
    UnifiedSearch {
        query: String,
        categories: Option<Vec<String>>,
        limit: Option<usize>,
    },
    UnifiedIndex,
}
