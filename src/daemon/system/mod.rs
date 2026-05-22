mod command;
mod logind;
mod polkit;
mod systemd;

use crate::DaemonState;
use crate::protocol::Action;

use logind::{inhibit, list_sessions, lock_session, release_inhibit, switch_user};
use polkit::check_auth;
use systemd::{journal_query, service_list, service_status, systemctl_enable, systemctl_unit};

pub fn is_system_control_action(action: &Action) -> bool {
    matches!(
        action,
        Action::SystemInhibit { .. }
            | Action::SystemReleaseInhibit { .. }
            | Action::SystemListSessions
            | Action::SystemLockSession { .. }
            | Action::SystemSwitchUser { .. }
            | Action::SystemCheckAuth { .. }
            | Action::SystemElevate { .. }
            | Action::SystemConfinement
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
    )
}

pub async fn execute_system_control_action(
    action: Action,
    state: &DaemonState,
) -> anyhow::Result<serde_json::Value> {
    match action {
        Action::SystemInhibit {
            what,
            who,
            why,
            mode,
        } => inhibit(state, &what, &who, why.as_deref(), mode.as_deref()).await,
        Action::SystemReleaseInhibit { inhibitor_id } => release_inhibit(state, inhibitor_id).await,
        Action::SystemListSessions => list_sessions().await,
        Action::SystemLockSession { session_id } => lock_session(session_id.as_deref()).await,
        Action::SystemSwitchUser { username } => switch_user(&username).await,
        Action::SystemCheckAuth { action_id } => check_auth(&action_id, false, None).await,
        Action::SystemElevate { action_id, reason } => {
            check_auth(&action_id, true, reason.as_deref()).await
        }
        Action::SystemConfinement => crate::daemon::build_confinement_report().await,
        Action::ServiceStatus { name } => service_status(&name).await,
        Action::ServiceStart { name } => systemctl_unit("start", &name).await,
        Action::ServiceStop { name } => systemctl_unit("stop", &name).await,
        Action::ServiceRestart { name } => systemctl_unit("restart", &name).await,
        Action::ServiceEnable { name, runtime } => systemctl_enable("enable", &name, runtime).await,
        Action::ServiceDisable { name, runtime } => {
            systemctl_enable("disable", &name, runtime).await
        }
        Action::ServiceList { unit_type } => service_list(unit_type.as_deref()).await,
        Action::JournalQuery {
            since,
            until,
            unit,
            priority,
            tail,
        } => journal_query(since, until, unit.as_deref(), priority, tail).await,
        Action::TimerList => service_list(Some("timer")).await,
        Action::TimerStart { name } => systemctl_unit("start", &name).await,
        Action::TimerStop { name } => systemctl_unit("stop", &name).await,
        _ => unreachable!("non-system action passed to system control dispatcher"),
    }
}
