use super::*;
use crate::protocol::Action;

pub fn into_system_action(cmd: Command) -> anyhow::Result<Action> {
    Ok(match cmd {
        Command::Notify { cmd } => match cmd {
            NotifyCmd::Send {
                title,
                body,
                urgency,
            } => Action::NotificationSend {
                app_name: "deskbrid-cli".into(),
                title,
                body,
                urgency,
            },
            NotifyCmd::Close { notification_id } => Action::NotificationClose { notification_id },
        },

        Command::System { cmd } => match cmd {
            SystemCmd::Info => Action::SystemInfo,
            SystemCmd::Idle => Action::SystemIdle,
            SystemCmd::Power { action } => Action::SystemPower { action },
            SystemCmd::Battery => Action::SystemBattery,
            SystemCmd::BacklightGet { device } => Action::SystemBacklightGet { device },
            SystemCmd::BacklightSet { percent, device } => {
                Action::SystemBacklightSet { percent, device }
            }
            SystemCmd::Inhibit {
                what,
                who,
                why,
                mode,
            } => Action::SystemInhibit {
                what,
                who,
                why,
                mode,
            },
            SystemCmd::ReleaseInhibit { inhibitor_id } => {
                Action::SystemReleaseInhibit { inhibitor_id }
            }
            SystemCmd::Sessions => Action::SystemListSessions,
            SystemCmd::LockSession { session_id } => Action::SystemLockSession { session_id },
            SystemCmd::SwitchUser { username } => Action::SystemSwitchUser { username },
            SystemCmd::CheckAuth { action_id } => Action::SystemCheckAuth { action_id },
            SystemCmd::Elevate { action_id, reason } => Action::SystemElevate { action_id, reason },
        },

        Command::Service { cmd } => match cmd {
            ServiceCmd::Status { name } => Action::ServiceStatus { name },
            ServiceCmd::Start { name } => Action::ServiceStart { name },
            ServiceCmd::Stop { name } => Action::ServiceStop { name },
            ServiceCmd::Restart { name } => Action::ServiceRestart { name },
            ServiceCmd::Enable { name, runtime } => Action::ServiceEnable { name, runtime },
            ServiceCmd::Disable { name, runtime } => Action::ServiceDisable { name, runtime },
            ServiceCmd::List { unit_type } => Action::ServiceList { unit_type },
        },

        Command::Journal { cmd } => match cmd {
            JournalCmd::Query {
                since,
                until,
                unit,
                priority,
                tail,
            } => Action::JournalQuery {
                since,
                until,
                unit,
                priority,
                tail,
            },
        },

        Command::Timer { cmd } => match cmd {
            TimerCmd::List => Action::TimerList,
            TimerCmd::Start { name } => Action::TimerStart { name },
            TimerCmd::Stop { name } => Action::TimerStop { name },
        },

        Command::Audit { cmd } => match cmd {
            AuditCmd::Log {
                limit,
                action_type,
                status,
            } => Action::AuditLog {
                limit,
                action_type,
                status,
            },
            AuditCmd::Clear => Action::AuditClear,
        },

        _ => bail!(
            "unexpected command in client mode: {:?}",
            std::mem::discriminant(&cmd)
        ),
    })
}
