use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::Action;
use serde_json::Value;

use super::{build_system_capabilities, run_system_remediation};

pub(crate) async fn execute_stubs(
    action: Action,
    backend: &dyn DesktopBackend,
    _state: &DaemonState,
) -> anyhow::Result<Value> {
    use Action::*;
    Ok(match action {
        SystemInfo => serde_json::json!(backend.system_info().await?),
        SystemCapabilities => serde_json::json!(build_system_capabilities(backend).await?),
        SystemConfinement => serde_json::json!(crate::daemon::build_confinement_report().await?),

        SystemIdle => serde_json::json!({"idle_seconds": backend.idle_seconds().await?}),
        SystemRemediate { ref check, apply } => {
            serde_json::json!(run_system_remediation(check, apply).await?)
        }
        A11yGetElement {
            role,
            ref name,
            index,
        } => crate::a11y::get_element(role.as_deref(), name.as_deref(), index).await?,
        A11yClickElement {
            role,
            ref name,
            index,
        } => crate::a11y::click_element(role.as_deref(), name.as_deref(), index).await?,
        LocationGet => serde_json::json!({"location": "not yet implemented"}),
        UiTreeGet => {
            serde_json::json!({"supported": false, "reason":"AT-SPI not integrated yet", "nodes":[]})
        }
        UiElementClick { ref selector } => {
            serde_json::json!({"supported": false, "reason":"AT-SPI not integrated yet", "selector": selector})
        }
        UiElementSetText {
            ref selector,
            ref text,
        } => {
            serde_json::json!({"supported": false, "reason":"AT-SPI not integrated yet", "selector": selector, "text": text})
        }

        // Handled before desktop-backend dispatch
        Ping
        | SystemInhibit { .. }
        | SystemReleaseInhibit { .. }
        | SystemListSessions
        | SystemLockSession { .. }
        | SystemSwitchUser { .. }
        | SystemCheckAuth { .. }
        | SystemElevate { .. }
        | ServiceStatus { .. }
        | ServiceStart { .. }
        | ServiceStop { .. }
        | ServiceRestart { .. }
        | ServiceEnable { .. }
        | ServiceDisable { .. }
        | ServiceList { .. }
        | JournalQuery { .. }
        | TimerList
        | TimerStart { .. }
        | TimerStop { .. }
        | WaitFor { .. }
        | TerminalCreate { .. }
        | TerminalWrite { .. }
        | TerminalRead { .. }
        | TerminalResize { .. }
        | TerminalList
        | TerminalKill { .. }
        | Subscribe { .. }
        | Unsubscribe { .. }
        | A11ySnapshotTree { .. }
        | A11yPerformAction { .. }
        | A11ySetValue { .. }
        | A11yGetElementText { .. }
        | A11yListApps { .. }
        | A11yDoctor
        | A11ySetupAccessibility
        | A11yClickElementByRef { .. }
        | Disconnect => unreachable!(),
        _ => unreachable!("not a stubs action"),
    })
}
