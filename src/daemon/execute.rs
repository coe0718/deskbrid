use crate::DaemonState;
use crate::protocol::Action;

use super::execute_audio;
use super::execute_audit;
use super::execute_bluetooth;
use super::execute_browser;
use super::execute_capabilities;
use super::execute_clipboard;
use super::execute_color;
use super::execute_delegated;
use super::execute_desktop;
use super::execute_files;
use super::execute_hotkeys;
use super::execute_input;
use super::execute_monitor;
use super::execute_network;
use super::execute_notification;
use super::execute_process;
use super::execute_screenshot;
use super::execute_search;
use super::execute_stubs;
use super::execute_system;
use super::execute_system::execute_dbus_call;
use super::execute_vision;
use super::execute_windows;
use super::execute_workspace;
use super::region_watch;

pub async fn execute_action(
    action: Action,
    backend: &dyn crate::backend::DesktopBackend,
    state: &crate::DaemonState,
) -> anyhow::Result<serde_json::Value> {
    use Action::*;

    Ok(match action {
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

        NetworkStatus
        | NetworkInterfaces
        | NetworkWifiScan
        | NetworkWifiConnect { .. }
        | NetworkConnectionList
        | NetworkConnectionProfiles
        | NetworkCreateHotspot { .. }
        | NetworkStopHotspot
        | NetworkWifiEnable { .. }
        | NetworkWwanEnable { .. }
        | NetworkDnsSet { .. }
        | NetworkDnsReset
        | NetworkVpnConnect { .. }
        | NetworkVpnDisconnect => execute_network::execute_network(action).await?,

        ClientsList => serde_json::json!({"clients": [], "count": 0}),

        NotificationSend { .. }
        | NotificationClose { .. }
        | NotificationHistory { .. }
        | NotificationAction { .. }
        | NotificationClearHistory
        | NotificationWatch => {
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

        VisionFindElement { .. } | VisionFindByText { .. } | VisionDetectState { .. } => {
            execute_vision::execute_vision(action, backend, state).await?
        }

        RegionWatchCreate { .. }
        | RegionWatchUpdate { .. }
        | RegionWatchRemove { .. }
        | RegionWatchList
        | TextWatchCreate { .. }
        | TextWatchRemove { .. }
        | TextWatchList => region_watch::execute_watch_action(action, state).await?,

        ScreencastStart { .. } | ScreencastStop => {
            execute_screenshot::execute_screencast(action, backend).await?
        }

        PortalScreenshot { .. } | PortalScreencastStart { .. } | PortalScreencastStop => {
            execute_portal(action, state).await?
        }

        SystemInfo
        | SystemCapabilities
        | SystemConfinement
        | SystemIdle
        | PresenceGet
        | PresenceConfig { .. }
        | SystemRemediate { .. }
        | LocationGet
        | UiTreeGet
        | UiElementClick { .. }
        | UiElementSetText { .. }
        | Ping => execute_stubs::execute_stubs(action, backend, state).await?,

        DesktopGetSetting { .. } | DesktopSetSetting { .. } | DesktopListSchemas => {
            execute_desktop::execute_desktop(action, backend, state).await?
        }

        SystemHealth
        | SystemNormalizeCoords { .. }
        | SystemPower { .. }
        | SystemBattery
        | EnvGet { .. }
        | EnvSet { .. }
        | EnvPersist { .. }
        | EnvUnset { .. }
        | EnvListPersisted
        | LocaleGet
        | LocaleSet { .. }
        | TimezoneGet
        | TimezoneSet { .. }
        | SystemBacklightList
        | SystemBacklightGet { .. }
        | SystemBacklightSet { .. }
        | SystemPrintList
        | SystemPrintDefault { .. }
        | SystemPrintJobList
        | SystemPrintJobCancel { .. }
        | SystemPrintJobPause { .. }
        | SystemPrintJobResume { .. }
        | SystemPressure
        | SystemThermalGet
        | SystemCpuFrequency
        | SystemCpuGovernor
        | SystemCpuSetGovernor { .. }
        | SystemUpdate { .. } => execute_system::execute_system(action, backend, state).await?,
        DbusCall { .. } => execute_dbus_call(&action).await?,

        ScheduleList | ScheduleAdd { .. } | ScheduleRemove { .. } => {
            execute_schedule(action, state).await?
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

        // Confirmation / agent / search actions are now dispatched directly
        // from dispatch_action_with_options before reaching execute_action.
        ConfirmAction { .. } | DenyAction { .. } | ConfirmationList => {
            anyhow::bail!(
                "internal dispatch error: confirmation actions must be handled in dispatch, not execute"
            )
        }
        AgentMessage { .. }
        | AgentBroadcast { .. }
        | AgentMailbox
        | AgentRegister { .. }
        | AgentList
        | AgentGet { .. }
        | AgentHeartbeat { .. }
        | LockAcquire { .. }
        | LockRelease { .. }
        | LockList => {
            anyhow::bail!(
                "internal dispatch error: agent and lock actions must be handled in dispatch, not execute"
            )
        }
        UnifiedSearch { .. } | UnifiedIndex => {
            execute_search::execute_search(action, state, backend).await?
        }

        _ => execute_stubs::execute_stubs(action, backend, state).await?,
    })
}

async fn execute_portal(action: Action, state: &DaemonState) -> anyhow::Result<serde_json::Value> {
    use Action::*;
    Ok(match action {
        PortalScreenshot { interactive } => super::portal::portal_screenshot(interactive).await?,
        PortalScreencastStart { output_path } => {
            super::portal::portal_screencast_start(&output_path, &state.screencast_process).await?
        }
        PortalScreencastStop => {
            super::portal::portal_screencast_stop(&state.screencast_process).await?
        }
        _ => anyhow::bail!("internal dispatch error: not a portal action"),
    })
}

async fn execute_schedule(
    action: Action,
    state: &DaemonState,
) -> anyhow::Result<serde_json::Value> {
    let mut sched = state.schedule.schedule.lock().await;
    match action {
        Action::ScheduleList => Ok(serde_json::json!({ "entries": *sched.entries })),
        Action::ScheduleAdd {
            name,
            interval_secs,
            action_type,
            action_params,
        } => {
            // Don't allow duplicates
            if sched.entries.iter().any(|e| e.name == name) {
                anyhow::bail!("schedule entry '{}' already exists", name);
            }
            let entry = crate::daemon::schedule::ScheduleEntry {
                name: name.clone(),
                interval_secs,
                action_type: action_type.clone(),
                action_params: action_params.unwrap_or(serde_json::json!({})),
                last_run: 0,
            };
            sched.entries.push(entry);
            sched.save().await?;
            Ok(serde_json::json!({ "added": name }))
        }
        Action::ScheduleRemove { name } => {
            let len_before = sched.entries.len();
            sched.entries.retain(|e| e.name != name);
            if sched.entries.len() == len_before {
                anyhow::bail!("schedule entry '{}' not found", name);
            }
            sched.save().await?;
            Ok(serde_json::json!({ "removed": name }))
        }
        _ => anyhow::bail!("internal dispatch error: not a schedule action"),
    }
}
