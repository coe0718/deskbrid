use super::Action;
use serde_json::json;

pub(super) fn serialize_system(action: &Action, id: &str) -> serde_json::Value {
    match action {
        // System
        Action::SystemInfo => json!({"type": "system.info", "id": id}),
        Action::SystemCapabilities => json!({"type": "system.capabilities", "id": id}),
        Action::SystemHealth => json!({"type": "system.health", "id": id}),
        Action::SystemConfinement => json!({"type": "system.confinement", "id": id}),
        Action::SystemRemediate { check, apply } => {
            json!({"type": "system.remediate", "id": id, "check": check, "apply": apply})
        }
        Action::SystemNormalizeCoords { x, y, monitor } => {
            let mut obj = json!({"type":"system.normalize_coords","id":id,"x":x,"y":y});
            if let Some(m) = monitor {
                obj["monitor"] = json!(m);
            }
            obj
        }
        Action::WaitFor {
            condition,
            params,
            timeout_ms,
            interval_ms,
        } => {
            let mut obj = json!({"type": "wait.for", "id": id, "condition": condition, "params": params, "timeout_ms": timeout_ms});
            if let Some(interval_ms) = interval_ms {
                obj["interval_ms"] = json!(interval_ms);
            }
            obj
        }
        Action::SystemIdle => json!({"type": "system.idle", "id": id}),
        Action::PresenceGet => json!({"type": "system.presence.get", "id": id}),
        Action::PresenceConfig {
            idle_threshold_secs,
            away_threshold_secs,
        } => {
            let mut obj = json!({"type":"system.presence.config","id":id});
            if let Some(v) = idle_threshold_secs {
                obj["idle_threshold_secs"] = json!(v);
            }
            if let Some(v) = away_threshold_secs {
                obj["away_threshold_secs"] = json!(v);
            }
            obj
        }
        Action::SystemPower { action } => {
            json!({"type": "system.power", "id": id, "action": action})
        }
        Action::SystemBattery => json!({"type": "system.battery", "id": id}),
        Action::SystemBacklightList => json!({"type": "system.backlight_list", "id": id}),
        Action::SystemBacklightGet { device } => {
            let mut obj = json!({"type": "system.backlight_get", "id": id});
            if let Some(device) = device {
                obj["device"] = json!(device);
            }
            obj
        }
        Action::SystemBacklightSet { device, value } => {
            let mut obj = json!({"type": "system.backlight_set", "id": id, "value": value});
            if let Some(device) = device {
                obj["device"] = json!(device);
            }
            obj
        }
        Action::SystemPrintList => json!({"type": "system.print_list", "id": id}),
        Action::SystemPrintDefault { printer } => {
            let mut obj = json!({"type": "system.print_default", "id": id});
            if let Some(printer) = printer {
                obj["printer"] = json!(printer);
            }
            obj
        }
        Action::SystemPrintFile { printer, path } => {
            json!({"type": "system.print_file", "id": id, "printer": printer, "path": path})
        }
        Action::SystemPrintJobList => json!({"type": "system.print_jobs", "id": id}),
        Action::SystemPrintJobCancel { job_id } => {
            json!({"type": "system.print_job_cancel", "id": id, "job_id": job_id})
        }
        Action::SystemPrintJobPause { job_id } => {
            json!({"type": "system.print_job_pause", "id": id, "job_id": job_id})
        }
        Action::SystemPrintJobResume { job_id } => {
            json!({"type": "system.print_job_resume", "id": id, "job_id": job_id})
        }
        Action::SystemPressure => json!({"type": "system.pressure", "id": id}),
        Action::SystemThermalGet => json!({"type": "system.thermal", "id": id}),
        Action::SystemCpuFrequency => json!({"type": "system.cpu.frequency", "id": id}),
        Action::SystemCpuGovernor => json!({"type": "system.cpu.governor", "id": id}),
        Action::SystemCpuSetGovernor { governor } => {
            json!({"type": "system.cpu.set_governor", "id": id, "governor": governor})
        }
        Action::SystemInhibit {
            what,
            who,
            why,
            mode,
        } => {
            let mut obj = json!({"type": "system.inhibit", "id": id, "what": what, "who": who});
            if let Some(why) = why {
                obj["why"] = json!(why);
            }
            if let Some(mode) = mode {
                obj["mode"] = json!(mode);
            }
            obj
        }
        Action::SystemReleaseInhibit { inhibitor_id } => {
            json!({"type": "system.release_inhibit", "id": id, "inhibitor_id": inhibitor_id})
        }
        Action::SystemListSessions => json!({"type": "system.sessions", "id": id}),
        Action::SystemLockSession { session_id } => {
            let mut obj = json!({"type": "system.lock_session", "id": id});
            if let Some(session_id) = session_id {
                obj["session_id"] = json!(session_id);
            }
            obj
        }
        Action::SystemSwitchUser { username } => {
            json!({"type": "system.switch_user", "id": id, "username": username})
        }
        Action::SystemCheckAuth { action_id } => {
            json!({"type": "system.check_auth", "id": id, "action_id": action_id})
        }
        Action::SystemElevate { action_id, reason } => {
            let mut obj = json!({"type": "system.elevate", "id": id, "action_id": action_id});
            if let Some(r) = reason {
                obj["reason"] = json!(r);
            }
            obj
        }
        Action::SystemUpdate { check, force } => {
            json!({"type": "system.update", "id": id, "check": check, "force": force})
        }
        Action::DbusCall {
            bus,
            service,
            path,
            interface,
            method,
            args,
        } => {
            let mut obj = json!({"type": "dbus.call", "id": id, "service": service, "path": path, "interface": interface, "method": method});
            if let Some(b) = bus {
                obj["bus"] = json!(b);
            }
            if let Some(a) = args {
                obj["args"] = a.clone();
            }
            obj
        }
        Action::ScheduleList => json!({"type": "schedule.list", "id": id}),
        Action::ScheduleAdd {
            name,
            interval_secs,
            action_type,
            action_params,
        } => {
            let mut obj = json!({"type": "schedule.add", "id": id, "name": name, "interval_secs": interval_secs, "action_type": action_type});
            if let Some(p) = action_params {
                obj["action_params"] = p.clone();
            }
            obj
        }
        Action::ScheduleRemove { name } => {
            json!({"type": "schedule.remove", "id": id, "name": name})
        }
        Action::ServiceStatus { name } => {
            json!({"type": "service.status", "id": id, "name": name})
        }
        Action::ServiceStart { name } => {
            json!({"type": "service.start", "id": id, "name": name})
        }
        Action::ServiceStop { name } => {
            json!({"type": "service.stop", "id": id, "name": name})
        }
        Action::ServiceRestart { name } => {
            json!({"type": "service.restart", "id": id, "name": name})
        }
        Action::ServiceEnable { name, runtime } => {
            json!({"type": "service.enable", "id": id, "name": name, "runtime": runtime})
        }
        Action::ServiceDisable { name, runtime } => {
            json!({"type": "service.disable", "id": id, "name": name, "runtime": runtime})
        }
        Action::ServiceList { unit_type } => {
            let mut obj = json!({"type": "service.list", "id": id});
            if let Some(unit_type) = unit_type {
                obj["unit_type"] = json!(unit_type);
            }
            obj
        }
        Action::JournalQuery {
            since,
            until,
            unit,
            priority,
            tail,
        } => {
            let mut obj = json!({"type": "journal.query", "id": id});
            if let Some(since) = since {
                obj["since"] = json!(since);
            }
            if let Some(until) = until {
                obj["until"] = json!(until);
            }
            if let Some(unit) = unit {
                obj["unit"] = json!(unit);
            }
            if let Some(priority) = priority {
                obj["priority"] = json!(priority);
            }
            if let Some(tail) = tail {
                obj["tail"] = json!(tail);
            }
            obj
        }
        Action::TimerList => json!({"type": "timer.list", "id": id}),
        Action::TimerStart { name } => json!({"type": "timer.start", "id": id, "name": name}),
        Action::TimerStop { name } => json!({"type": "timer.stop", "id": id, "name": name}),

        // Network
        Action::NetworkStatus => json!({"type": "network.status", "id": id}),
        Action::NetworkInterfaces => json!({"type": "network.interfaces", "id": id}),
        Action::NetworkWifiScan => json!({"type": "network.wifi.scan", "id": id}),
        Action::NetworkWifiConnect { ssid, password } => {
            let mut obj = json!({"type": "network.wifi.connect", "id": id, "ssid": ssid});
            if let Some(pw) = password {
                obj["password"] = json!(pw);
            }
            obj
        }
        Action::NetworkConnectionList => json!({"type": "network.connections.list", "id": id}),
        Action::NetworkConnectionProfiles => {
            json!({"type": "network.connections.profiles", "id": id})
        }
        Action::NetworkCreateHotspot { ssid, password } => {
            let mut obj = json!({"type": "network.hotspot.start", "id": id, "ssid": ssid});
            if let Some(pw) = password {
                obj["password"] = json!(pw);
            }
            obj
        }
        Action::NetworkStopHotspot => json!({"type": "network.hotspot.stop", "id": id}),
        Action::NetworkWifiEnable { enabled } => {
            json!({"type": "network.wifi.enable", "id": id, "enabled": enabled})
        }
        Action::NetworkWwanEnable { enabled } => {
            json!({"type": "network.wwan.enable", "id": id, "enabled": enabled})
        }
        Action::NetworkDnsSet { dns } => {
            json!({"type": "network.dns.set", "id": id, "dns": dns})
        }
        Action::NetworkDnsReset => json!({"type": "network.dns.reset", "id": id}),
        Action::NetworkVpnConnect { profile_name } => {
            json!({"type": "network.vpn.connect", "id": id, "profile_name": profile_name})
        }
        Action::NetworkVpnDisconnect => json!({"type": "network.vpn.disconnect", "id": id}),

        // Clients
        Action::ClientsList => json!({"type": "clients.list", "id": id}),
        _ => serde_json::json!({"error": "not a system action"}),
    }
}
