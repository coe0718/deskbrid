use crate::protocol::Action;

use super::execute_a11y;
use super::execute_audio;
use super::execute_audit;
use super::execute_bluetooth;
use super::execute_browser;
use super::execute_capabilities;
use super::execute_clipboard;
use super::execute_color;
use super::execute_delegated;
use super::execute_files;
use super::execute_hotkeys;
use super::execute_input;
use super::execute_monitor;
use super::execute_network;
use super::execute_notification;
use super::execute_process;
use super::execute_screenshot;
use super::execute_stubs;
use super::execute_system;
use super::execute_windows;
use super::execute_workspace;

pub async fn execute_action(
    action: Action,
    backend: &dyn crate::backend::DesktopBackend,
    state: &crate::DaemonState,
) -> anyhow::Result<serde_json::Value> {
    use Action::*;

    Ok(match action {
        A11yTree { .. }
        | A11yGetText { .. }
        | A11ySnapshotTree { .. }
        | A11yPerformAction { .. }
        | A11ySetValue { .. }
        | A11yGetElementText { .. }
        | A11yListApps { .. }
        | A11yDoctor
        | A11ySetupAccessibility
        | A11yClickElementByRef { .. } => {
            execute_a11y::execute_a11y(action, backend, state).await?
        }

        AudioListSinks
        | AudioSetSinkVolume { .. }
        | AudioListSources
        | AudioGetVolume { .. }
        | AudioSetVolume { .. }
        | AudioMute { .. }
        | AudioSetDefault { .. } => execute_audio::execute_audio(action, backend, state).await?,

        AuditLog { .. } | AuditClear => {
            execute_audit::execute_audit(action, backend, state).await?
        }

        BluetoothList
        | BluetoothScan { .. }
        | BluetoothStopScan
        | BluetoothConnect { .. }
        | BluetoothDisconnect { .. }
        | BluetoothPair { .. }
        | BluetoothForget { .. } => {
            execute_bluetooth::execute_bluetooth(action, backend, state).await?
        }

        BrowserListTabs
        | BrowserNavigate { .. }
        | BrowserEvaluate { .. }
        | BrowserScreenshotTab { .. }
        | BrowserClick { .. } => execute_browser::execute_browser(action, backend, state).await?,

        CapabilitiesList => {
            execute_capabilities::execute_capabilities(action, backend, state).await?
        }

        ClipboardRead
        | ClipboardWrite { .. }
        | ClipboardHistoryList { .. }
        | ClipboardHistoryClear => {
            execute_clipboard::execute_clipboard(action, backend, state).await?
        }

        ColorPick { .. } => execute_color::execute_color(action, backend, state).await?,

        AppList { .. }
        | AppSearch { .. }
        | AppGet { .. }
        | MprisList
        | MprisGet { .. }
        | MprisControl { .. } => {
            execute_delegated::execute_delegated(action, backend, state).await?
        }

        FilesWatch { .. }
        | FilesUnwatch { .. }
        | FilesSearch { .. }
        | FilesRead { .. }
        | FilesWrite { .. }
        | FilesCopy { .. }
        | FilesMove { .. }
        | FilesDelete { .. }
        | FilesMkdir { .. }
        | FilesList { .. } => execute_files::execute_files(action, backend, state).await?,

        HotkeysRegister { .. } | HotkeysUnregister { .. } => {
            execute_hotkeys::execute_hotkeys(action, backend, state).await?
        }

        InputKeyboardType { .. }
        | InputKeyboardKey { .. }
        | InputKeyboardCombo { .. }
        | InputMouse { .. }
        | InputMouseDrag { .. }
        | InputListLayouts
        | InputGetLayout
        | InputSetLayout { .. }
        | InputAddLayout { .. }
        | InputRemoveLayout { .. } => execute_input::execute_input(action, backend, state).await?,

        MonitorList
        | MonitorSetPrimary { .. }
        | MonitorSetResolution { .. }
        | MonitorSetScale { .. }
        | MonitorSetRotation { .. }
        | MonitorEnable { .. }
        | MonitorDisable { .. } => execute_monitor::execute_monitor(action, backend, state).await?,

        NetworkStatus | NetworkInterfaces | NetworkWifiScan | NetworkWifiConnect { .. } => {
            execute_network::execute_network(action, backend, state).await?
        }

        ClientsList => serde_json::json!({"clients": [], "count": 0}),

        NotificationSend { .. } | NotificationClose { .. } => {
            execute_notification::execute_notification(action, backend, state).await?
        }

        ProcessList
        | ProcessStart { .. }
        | ProcessStop { .. }
        | ProcessSignal { .. }
        | ProcessExists { .. }
        | ProcessWait { .. } => execute_process::execute_process(action, backend, state).await?,

        Screenshot { .. } | ScreenshotOcr { .. } | ScreenshotDiff { .. } => {
            execute_screenshot::execute_screenshot(action, backend, state).await?
        }

        ScreencastStart { .. } | ScreencastStop => {
            execute_screenshot::execute_screencast(action, backend).await?
        }

        PortalScreenshot { .. } | PortalScreencastStart { .. } | PortalScreencastStop => {
            execute_portal(action).await?
        }

        SystemInfo
        | SystemCapabilities
        | SystemConfinement
        | SystemIdle
        | SystemRemediate { .. }
        | A11yGetElement { .. }
        | A11yClickElement { .. }
        | LocationGet
        | UiTreeGet
        | UiElementClick { .. }
        | UiElementSetText { .. }
        | Ping => execute_stubs::execute_stubs(action, backend, state).await?,

        SystemHealth
        | SystemNormalizeCoords { .. }
        | SystemPower { .. }
        | SystemBattery
        | SystemBacklightGet { .. }
        | SystemBacklightSet { .. }
        | SystemThermalGet
        | SystemCpuFrequency
        | SystemCpuGovernor
        | SystemCpuSetGovernor { .. } => {
            execute_system::execute_system(action, backend, state).await?
        }

        WindowsList
        | WindowsFocus(..)
        | WindowsGet(..)
        | WindowsClose(..)
        | WindowsMinimize(..)
        | WindowsMaximize(..)
        | WindowsMoveResize { .. }
        | WindowsTile { .. }
        | WindowsActivateOrLaunch { .. } => {
            execute_windows::execute_windows(action, backend, state).await?
        }

        WorkspacesList
        | WorkspaceSwitch(..)
        | WorkspaceMoveWindow { .. }
        | LayoutProfilesList
        | LayoutProfileGet { .. }
        | LayoutProfileSave { .. }
        | LayoutProfileDelete { .. }
        | LayoutProfileRestore { .. } => {
            execute_workspace::execute_workspace(action, backend, state).await?
        }

        _ => execute_stubs::execute_stubs(action, backend, state).await?,
    })
}

async fn execute_portal(action: Action) -> anyhow::Result<serde_json::Value> {
    use Action::*;
    Ok(match action {
        PortalScreenshot { interactive } => super::portal::portal_screenshot(interactive).await?,
        PortalScreencastStart { output_path } => {
            super::portal::portal_screencast_start(&output_path).await?
        }
        PortalScreencastStop => super::portal::portal_screencast_stop().await?,
        _ => unreachable!("not a portal action"),
    })
}
