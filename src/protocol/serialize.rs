use uuid::Uuid;

use super::Action;

mod a11y;
mod a11y_location;
mod action_type;
mod apps;
mod audio;
mod audit;
mod blackboard;
mod bluetooth;
mod color_pick;
mod connection;
mod desktop;
mod extensions;
mod files;
mod input;
mod macro_cmd;
mod process;
mod rules;
mod screenshot;
mod sessions;
mod system;
mod watch;
mod windows;

pub use action_type::action_type;

pub fn to_json(action: &Action) -> anyhow::Result<String> {
    let id = Uuid::new_v4().to_string();
    let envelope = match action {
        Action::Ping
        | Action::Subscribe { .. }
        | Action::Unsubscribe { .. }
        | Action::Disconnect => connection::serialize_connection(action, &id),

        // Windows / Workspaces / Layout profiles
        Action::WindowsList
        | Action::WindowsFocus(..)
        | Action::WindowsGet(..)
        | Action::WindowsClose(..)
        | Action::WindowsMinimize(..)
        | Action::WindowsMaximize(..)
        | Action::WindowsMoveResize { .. }
        | Action::WindowsTile { .. }
        | Action::WindowsActivateOrLaunch { .. }
        | Action::WorkspacesList
        | Action::WorkspaceSwitch(..)
        | Action::WorkspaceMoveWindow { .. }
        | Action::LayoutProfilesList
        | Action::LayoutProfileGet { .. }
        | Action::LayoutProfileSave { .. }
        | Action::LayoutProfileDelete { .. }
        | Action::LayoutProfileRestore { .. } => windows::serialize_windows(action, &id),

        // Input / Clipboard
        Action::InputKeyboardType { .. }
        | Action::InputKeyboardKey { .. }
        | Action::InputKeyboardCombo { .. }
        | Action::InputMouse { .. }
        | Action::InputMouseDrag { .. }
        | Action::InputListLayouts
        | Action::InputGetLayout
        | Action::InputSetLayout { .. }
        | Action::InputAddLayout { .. }
        | Action::InputRemoveLayout { .. }
        | Action::ClipboardRead
        | Action::ClipboardWrite { .. }
        | Action::ClipboardHistoryList { .. }
        | Action::ClipboardHistoryClear => input::serialize_input(action, &id),

        // Apps / MPRIS
        Action::AppList { .. }
        | Action::AppSearch { .. }
        | Action::AppGet { .. }
        | Action::MprisList
        | Action::MprisGet { .. }
        | Action::MprisControl { .. } => apps::serialize_apps(action, &id),

        // Color picker
        Action::ColorPick { .. } => color_pick::serialize_color_pick(action, &id),

        // Screenshot
        Action::Screenshot { .. }
        | Action::ScreenshotOcr { .. }
        | Action::ScreenshotDiff { .. } => screenshot::serialize_screenshot(action, &id),

        // Screen/text watching
        Action::RegionWatchCreate { .. }
        | Action::RegionWatchUpdate { .. }
        | Action::RegionWatchRemove { .. }
        | Action::RegionWatchList
        | Action::TextWatchCreate { .. }
        | Action::TextWatchRemove { .. }
        | Action::TextWatchList => watch::serialize_watch(action, &id),

        // Screencast / portal
        Action::ScreencastStart { .. }
        | Action::ScreencastStop
        | Action::PortalScreenshot { .. }
        | Action::PortalScreencastStart { .. }
        | Action::PortalScreencastStop => screenshot::serialize_screenshot(action, &id),

        // Audit / Notifications
        Action::AuditLog { .. }
        | Action::AuditClear
        | Action::NotificationSend { .. }
        | Action::NotificationClose { .. }
        | Action::NotificationHistory { .. }
        | Action::NotificationAction { .. }
        | Action::NotificationClearHistory
        | Action::NotificationWatch => audit::serialize_audit(action, &id),

        // System / Network
        Action::SystemInfo
        | Action::SystemCapabilities
        | Action::SystemHealth
        | Action::SystemConfinement
        | Action::SystemRemediate { .. }
        | Action::SystemNormalizeCoords { .. }
        | Action::WaitFor { .. }
        | Action::SystemIdle
        | Action::SystemPower { .. }
        | Action::SystemBattery
        | Action::SystemBacklightList
        | Action::SystemBacklightGet { .. }
        | Action::SystemBacklightSet { .. }
        | Action::SystemPrintList
        | Action::SystemPrintDefault { .. }
        | Action::SystemPrintFile { .. }
        | Action::SystemPrintJobList
        | Action::SystemPrintJobCancel { .. }
        | Action::SystemPrintJobPause { .. }
        | Action::SystemPrintJobResume { .. }
        | Action::SystemPressure
        | Action::SystemThermalGet
        | Action::SystemCpuFrequency
        | Action::SystemCpuGovernor
        | Action::SystemCpuSetGovernor { .. }
        | Action::SystemInhibit { .. }
        | Action::SystemReleaseInhibit { .. }
        | Action::SystemListSessions
        | Action::SystemLockSession { .. }
        | Action::SystemSwitchUser { .. }
        | Action::SystemCheckAuth { .. }
        | Action::SystemElevate { .. }
        | Action::SystemUpdate { .. }
        | Action::DbusCall { .. }
        | Action::ScheduleList
        | Action::ScheduleAdd { .. }
        | Action::ScheduleRemove { .. }
        | Action::ServiceStatus { .. }
        | Action::ServiceStart { .. }
        | Action::ServiceStop { .. }
        | Action::ServiceRestart { .. }
        | Action::ServiceEnable { .. }
        | Action::ServiceDisable { .. }
        | Action::ServiceList { .. }
        | Action::JournalQuery { .. }
        | Action::TimerList
        | Action::TimerStart { .. }
        | Action::TimerStop { .. }
        | Action::NetworkStatus
        | Action::NetworkInterfaces
        | Action::NetworkWifiScan
        | Action::NetworkWifiConnect { .. }
        | Action::NetworkConnectionList
        | Action::NetworkConnectionProfiles
        | Action::NetworkCreateHotspot { .. }
        | Action::NetworkStopHotspot
        | Action::NetworkWifiEnable { .. }
        | Action::NetworkWwanEnable { .. }
        | Action::NetworkDnsSet { .. }
        | Action::NetworkDnsReset
        | Action::NetworkVpnConnect { .. }
        | Action::NetworkVpnDisconnect => system::serialize_system(action, &id),

        // Bluetooth
        Action::BluetoothList
        | Action::BluetoothScan { .. }
        | Action::BluetoothStopScan
        | Action::BluetoothConnect { .. }
        | Action::BluetoothDisconnect { .. }
        | Action::BluetoothPair { .. }
        | Action::BluetoothForget { .. } => bluetooth::serialize_bluetooth(action, &id),

        // Files
        Action::FilesWatch { .. }
        | Action::FilesUnwatch { .. }
        | Action::FilesSearch { .. }
        | Action::FilesRead { .. }
        | Action::FilesWrite { .. }
        | Action::FilesCopy { .. }
        | Action::FilesMove { .. }
        | Action::FilesDelete { .. }
        | Action::FilesMkdir { .. }
        | Action::FilesList { .. }
        | Action::BrowserListTabs
        | Action::BrowserNavigate { .. }
        | Action::BrowserEvaluate { .. }
        | Action::BrowserScreenshotTab { .. }
        | Action::BrowserClick { .. } => files::serialize_files(action, &id),

        // Accessibility / Location
        Action::A11yTree { .. }
        | Action::A11yGetElement { .. }
        | Action::A11yClickElement { .. }
        | Action::A11yGetText { .. }
        | Action::A11ySnapshotTree { .. }
        | Action::A11yPerformAction { .. }
        | Action::A11ySetValue { .. }
        | Action::A11yGetElementText { .. }
        | Action::A11yListApps { .. }
        | Action::A11yDoctor
        | Action::A11ySetupAccessibility
        | Action::A11yClickElementByRef { .. }
        | Action::LocationGet
        | Action::UiTreeGet
        | Action::UiElementClick { .. }
        | Action::UiElementSetText { .. } => match action {
            Action::LocationGet
            | Action::UiTreeGet
            | Action::UiElementClick { .. }
            | Action::UiElementSetText { .. } => {
                a11y_location::serialize_a11y_location(action, &id)
            }
            _ => a11y::serialize_a11y(action, &id),
        },

        // Process / Hotkeys / Terminal / Capabilities
        Action::ProcessList
        | Action::ProcessStart { .. }
        | Action::ProcessStop { .. }
        | Action::ProcessSignal { .. }
        | Action::ProcessExists { .. }
        | Action::ProcessWait { .. }
        | Action::HotkeysRegister { .. }
        | Action::HotkeysUnregister { .. }
        | Action::TerminalCreate { .. }
        | Action::TerminalWrite { .. }
        | Action::TerminalRead { .. }
        | Action::TerminalResize { .. }
        | Action::TerminalList
        | Action::TerminalKill { .. }
        | Action::CapabilitiesList => process::serialize_process(action, &id),

        // Audio / Monitor
        Action::AudioListSinks
        | Action::AudioSetSinkVolume { .. }
        | Action::AudioListSources
        | Action::AudioGetVolume { .. }
        | Action::AudioSetVolume { .. }
        | Action::AudioMute { .. }
        | Action::AudioSetDefault { .. }
        | Action::MonitorList
        | Action::MonitorSetPrimary { .. }
        | Action::MonitorSetResolution { .. }
        | Action::MonitorSetScale { .. }
        | Action::MonitorSetRotation { .. }
        | Action::MonitorEnable { .. }
        | Action::MonitorDisable { .. } => audio::serialize_audio(action, &id),

        // Clients
        Action::ClientsList => system::serialize_system(action, &id),

        // Macro
        Action::MacroRecordStart { .. }
        | Action::MacroRecordStop
        | Action::MacroReplay { .. }
        | Action::MacroList
        | Action::MacroGet { .. }
        | Action::MacroDelete { .. }
        | Action::MacroExport { .. }
        | Action::MacroImport { .. } => macro_cmd::serialize_macro(action, &id),

        // Sessions
        Action::SessionCreate { .. }
        | Action::SessionDestroy { .. }
        | Action::SessionList
        | Action::SessionSwitch { .. }
        | Action::SessionSuspend { .. }
        | Action::SessionResume { .. }
        | Action::SessionVarSet { .. }
        | Action::SessionVarGet { .. }
        | Action::SessionVarList => sessions::serialize_sessions(action, &id),

        // Rules
        Action::RuleCreate { .. }
        | Action::RuleList
        | Action::RuleGet { .. }
        | Action::RuleDelete { .. }
        | Action::RulePause { .. }
        | Action::RuleResume { .. } => rules::serialize_rules(action, &id),

        // Blackboard
        Action::BlackboardSet { .. }
        | Action::BlackboardGet { .. }
        | Action::BlackboardDelete { .. }
        | Action::BlackboardList { .. } => blackboard::serialize_blackboard(action, &id),

        // Desktop Settings
        Action::DesktopGetSetting { .. }
        | Action::DesktopSetSetting { .. }
        | Action::DesktopListSchemas => desktop::serialize_desktop(action, &id),

        // Extension actions (confirmation, agent messaging, search)
        Action::ConfirmAction { .. }
        | Action::DenyAction { .. }
        | Action::ConfirmationList
        | Action::AgentMessage { .. }
        | Action::AgentBroadcast { .. }
        | Action::AgentMailbox
        | Action::AgentRegister { .. }
        | Action::AgentList
        | Action::AgentGet { .. }
        | Action::AgentHeartbeat { .. }
        | Action::LockAcquire { .. }
        | Action::LockRelease { .. }
        | Action::LockList
        | Action::UnifiedSearch { .. }
        | Action::UnifiedIndex
        | Action::SecretsListCollections
        | Action::SecretsGetSecret { .. }
        | Action::SecretsStoreSecret { .. } => {
            let at = action.action_type();
            if at.starts_with("agent.") {
                extensions::serialize_agent(action, &id)
            } else if at.starts_with("lock.") {
                extensions::serialize_lock(action, &id)
            } else if at.starts_with("search.") {
                extensions::serialize_search(action, &id)
            } else if at.starts_with("secrets.") {
                extensions::serialize_secrets(action, &id)
            } else {
                extensions::serialize_confirmation(action, &id)
            }
        }
    };

    Ok(serde_json::to_string(&envelope)?)
}
