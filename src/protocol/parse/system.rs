use super::helpers::*;
use crate::protocol::Action;
use serde_json::Value;

pub(super) fn parse_system(raw: &Value, _id: &str, type_str: &str) -> anyhow::Result<Action> {
    Ok(match type_str {
        // System
        "system.info" => Action::SystemInfo,
        "system.capabilities" => Action::SystemCapabilities,
        "system.health" => Action::SystemHealth,
        "system.confinement" => Action::SystemConfinement,
        "system.remediate" => Action::SystemRemediate {
            check: raw["check"].as_str().unwrap_or("").into(),
            apply: raw["apply"].as_bool().unwrap_or(false),
        },
        "system.normalize_coords" => Action::SystemNormalizeCoords {
            x: raw["x"].as_f64().unwrap_or(0.0),
            y: raw["y"].as_f64().unwrap_or(0.0),
            monitor: raw["monitor"].as_u64().map(|m| m as u32),
        },
        "wait.for" => Action::WaitFor {
            condition: required_non_empty_string(raw, "condition")?,
            params: raw
                .get("params")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({})),
            timeout_ms: raw["timeout_ms"]
                .as_u64()
                .or_else(|| raw["timeout"].as_u64())
                .unwrap_or(30_000),
            interval_ms: raw["interval_ms"].as_u64(),
        },
        "system.idle" => Action::SystemIdle,
        "system.presence.get" => Action::PresenceGet,
        "system.presence.config" => Action::PresenceConfig {
            idle_threshold_secs: raw.get("idle_threshold_secs").and_then(|v| v.as_u64()),
            away_threshold_secs: raw.get("away_threshold_secs").and_then(|v| v.as_u64()),
        },
        "system.time_of_day" => Action::TimeOfDay,
        "system.time_of_day.config" => Action::TimeOfDayConfig {
            latitude: raw.get("latitude").and_then(|v| v.as_f64()),
            longitude: raw.get("longitude").and_then(|v| v.as_f64()),
            format_24h: raw.get("format_24h").and_then(|v| v.as_bool()),
        },
        "power.profile.list" => Action::PowerProfileList,
        "power.profile.get" => Action::PowerProfileGet,
        "power.profile.set" => Action::PowerProfileSet {
            profile: required_non_empty_string(raw, "profile")?,
        },
        "system.power" => Action::SystemPower {
            action: raw["action"].as_str().unwrap_or("").into(),
        },
        "system.battery" => Action::SystemBattery,
        "battery.threshold.get" => Action::BatteryThresholdGet,
        "battery.threshold.set" => Action::BatteryThresholdSet {
            start: raw.get("start").and_then(|v| v.as_u64()).map(|n| n as u32),
            end: raw.get("end").and_then(|v| v.as_u64()).map(|n| n as u32),
            profile: raw
                .get("profile")
                .and_then(|v| v.as_str())
                .map(String::from),
        },
        "locale.get" => Action::LocaleGet,
        "locale.set" => Action::LocaleSet {
            vars: parse_locale_vars(raw)?,
        },
        "timezone.get" => Action::TimezoneGet,
        "timezone.set" => Action::TimezoneSet {
            timezone: raw["timezone"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("timezone.set requires 'timezone'"))?
                .to_string(),
        },
        "system.backlight_list" => Action::SystemBacklightList,
        "system.backlight_get" => Action::SystemBacklightGet {
            device: raw["device"].as_str().map(String::from),
        },
        "system.backlight_set" => Action::SystemBacklightSet {
            device: raw["device"].as_str().map(String::from),
            value: raw["value"].as_str().unwrap_or("").into(),
        },
        "system.print_list" => Action::SystemPrintList,
        "system.print_default" => Action::SystemPrintDefault {
            printer: raw["printer"].as_str().map(String::from),
        },
        "system.print_file" => Action::SystemPrintFile {
            printer: required_non_empty_string(raw, "printer")?,
            path: required_non_empty_string(raw, "path")?,
        },
        "system.print_jobs" => Action::SystemPrintJobList,
        "system.print_job_cancel" => Action::SystemPrintJobCancel {
            job_id: required_non_empty_string(raw, "job_id")?,
        },
        "system.print_job_pause" => Action::SystemPrintJobPause {
            job_id: required_non_empty_string(raw, "job_id")?,
        },
        "system.print_job_resume" => Action::SystemPrintJobResume {
            job_id: required_non_empty_string(raw, "job_id")?,
        },
        "system.pressure" => Action::SystemPressure,
        "system.thermal" => Action::SystemThermalGet,
        "system.cpu.frequency" => Action::SystemCpuFrequency,
        "system.cpu.governor" => Action::SystemCpuGovernor,
        "system.cpu.set_governor" => Action::SystemCpuSetGovernor {
            governor: required_non_empty_string(raw, "governor")?,
        },
        "system.inhibit" => Action::SystemInhibit {
            what: required_non_empty_string(raw, "what")?,
            who: required_non_empty_string(raw, "who")?,
            why: raw["why"].as_str().map(String::from),
            mode: raw["mode"].as_str().map(String::from),
        },
        "system.release_inhibit" => Action::SystemReleaseInhibit {
            inhibitor_id: required_positive_u32(raw, "inhibitor_id")?,
        },
        "system.sessions" => Action::SystemListSessions,
        "system.lock_session" => Action::SystemLockSession {
            session_id: optional_non_empty_string(raw, "session_id")?,
        },
        "system.switch_user" => Action::SystemSwitchUser {
            username: required_non_empty_string(raw, "username")?,
        },
        "system.check_auth" => Action::SystemCheckAuth {
            action_id: required_non_empty_string(raw, "action_id")?,
        },
        "system.elevate" => Action::SystemElevate {
            action_id: required_non_empty_string(raw, "action_id")?,
            reason: raw["reason"].as_str().map(String::from),
        },
        "system.update" => Action::SystemUpdate {
            check: raw["check"].as_bool().unwrap_or(false),
            force: raw["force"].as_bool().unwrap_or(false),
        },
        "dbus.call" => Action::DbusCall {
            bus: raw["bus"].as_str().map(String::from),
            service: required_non_empty_string(raw, "service")?,
            path: required_non_empty_string(raw, "path")?,
            interface: required_non_empty_string(raw, "interface")?,
            method: required_non_empty_string(raw, "method")?,
            args: raw.get("args").cloned(),
        },
        "schedule.list" => Action::ScheduleList,
        "schedule.add" => Action::ScheduleAdd {
            name: required_non_empty_string(raw, "name")?,
            interval_secs: raw["interval_secs"].as_u64().unwrap_or(3600),
            action_type: required_non_empty_string(raw, "action_type")?,
            action_params: raw.get("action_params").cloned(),
        },
        "schedule.remove" => Action::ScheduleRemove {
            name: required_non_empty_string(raw, "name")?,
        },
        "service.status" => Action::ServiceStatus {
            name: required_non_empty_string(raw, "name")?,
        },
        "service.start" => Action::ServiceStart {
            name: required_non_empty_string(raw, "name")?,
        },
        "service.stop" => Action::ServiceStop {
            name: required_non_empty_string(raw, "name")?,
        },
        "service.restart" => Action::ServiceRestart {
            name: required_non_empty_string(raw, "name")?,
        },
        "service.enable" => Action::ServiceEnable {
            name: required_non_empty_string(raw, "name")?,
            runtime: raw["runtime"].as_bool().unwrap_or(false),
        },
        "service.disable" => Action::ServiceDisable {
            name: required_non_empty_string(raw, "name")?,
            runtime: raw["runtime"].as_bool().unwrap_or(false),
        },
        "service.list" => Action::ServiceList {
            unit_type: raw["unit_type"].as_str().map(String::from),
        },
        "journal.query" => Action::JournalQuery {
            since: raw["since"].as_u64(),
            until: raw["until"].as_u64(),
            unit: optional_non_empty_string(raw, "unit")?,
            priority: optional_priority(raw, "priority")?,
            tail: optional_u32(raw, "tail")?,
        },
        "timer.list" => Action::TimerList,
        "timer.start" => Action::TimerStart {
            name: required_non_empty_string(raw, "name")?,
        },
        "timer.stop" => Action::TimerStop {
            name: required_non_empty_string(raw, "name")?,
        },
        "clients.list" => Action::ClientsList,
        _ => anyhow::bail!("unknown system type: {type_str}"),
    })
}

/// Parse the `vars` field of a `locale.set` request.
/// Accepts either:
///
/// - `"vars": {"LANG": "en_US.UTF-8", "LC_TIME": "en_DK.UTF-8"}`  (object)
/// - `"vars": [["LANG","en_US.UTF-8"], ...]`  (array of pairs)
///
/// Returns an empty Vec if `vars` is missing.
fn parse_locale_vars(raw: &serde_json::Value) -> anyhow::Result<Vec<(String, String)>> {
    let Some(v) = raw.get("vars") else {
        return Ok(Vec::new());
    };
    if v.is_null() {
        return Ok(Vec::new());
    }
    if let Some(obj) = v.as_object() {
        let mut out = Vec::with_capacity(obj.len());
        for (k, val) in obj {
            let s = val
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("locale.set: var {:?} must be a string", k))?;
            out.push((k.clone(), s.to_string()));
        }
        return Ok(out);
    }
    if let Some(arr) = v.as_array() {
        let mut out = Vec::with_capacity(arr.len());
        for (i, item) in arr.iter().enumerate() {
            let pair = item
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("locale.set: vars[{}] must be [name, value]", i))?;
            if pair.len() != 2 {
                anyhow::bail!("locale.set: vars[{}] must be exactly [name, value]", i);
            }
            let name = pair[0]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("locale.set: vars[{}][0] must be a string", i))?;
            let value = pair[1]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("locale.set: vars[{}][1] must be a string", i))?;
            out.push((name.to_string(), value.to_string()));
        }
        return Ok(out);
    }
    anyhow::bail!("locale.set: 'vars' must be an object or array of pairs")
}
