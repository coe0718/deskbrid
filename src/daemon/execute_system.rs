use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::Action;
use serde_json::Value;

use super::{
    build_system_health, cpu_frequency, cpu_governor, cpu_set_governor, expand_path,
    normalize_coords, thermal_get,
};

pub(crate) async fn execute_system(
    action: Action,
    backend: &dyn DesktopBackend,
    _state: &DaemonState,
) -> anyhow::Result<Value> {
    use Action::*;
    Ok(match action {
        SystemHealth => serde_json::json!(build_system_health(backend).await?),
        SystemNormalizeCoords { x, y, monitor } => {
            let info = backend.system_info().await?;
            serde_json::json!(normalize_coords(&info, x, y, monitor))
        }
        SystemPower { ref action } => {
            backend.power_action(action).await?;
            serde_json::json!({"power": action})
        }
        SystemBattery => serde_json::json!(backend.battery_status().await?),
        EnvGet { ref name } => serde_json::json!(crate::daemon::env::env_get(name.as_deref())),
        EnvSet {
            ref name,
            ref value,
        } => serde_json::json!(crate::daemon::env::env_set(name, value)),
        EnvPersist { ref vars } => serde_json::json!(crate::daemon::env::env_persist(vars)),
        EnvUnset { ref names } => serde_json::json!(crate::daemon::env::env_unset(names)),
        EnvListPersisted => serde_json::json!(crate::daemon::env::env_list_persisted()),
        SystemBacklightList => serde_json::json!(backend.backlight_list().await?),
        SystemBacklightGet { ref device } => {
            serde_json::json!(backend.backlight_get(device.as_deref()).await?)
        }
        SystemBacklightSet {
            ref device,
            ref value,
        } => serde_json::json!(backend.backlight_set(device.as_deref(), value).await?),
        SystemPrintList => serde_json::json!(backend.print_list().await?),
        SystemPrintDefault { ref printer } => {
            serde_json::json!(backend.print_default(printer.as_deref()).await?)
        }
        SystemPrintFile {
            ref printer,
            ref path,
        } => {
            let safe_path = expand_path(path)?;
            serde_json::json!(
                backend
                    .print_file(printer, &safe_path.to_string_lossy())
                    .await?
            )
        }
        SystemPrintJobList => serde_json::json!(backend.print_jobs().await?),
        SystemPrintJobCancel { ref job_id } => {
            backend.print_job_cancel(job_id).await?;
            serde_json::json!({"cancelled": job_id})
        }
        SystemPrintJobPause { ref job_id } => {
            backend.print_job_pause(job_id).await?;
            serde_json::json!({"paused": job_id})
        }
        SystemPrintJobResume { ref job_id } => {
            backend.print_job_resume(job_id).await?;
            serde_json::json!({"resumed": job_id})
        }
        SystemPressure => system_pressure().await?,
        SystemThermalGet => thermal_get().await?,
        SystemCpuFrequency => cpu_frequency().await?,
        SystemCpuGovernor => cpu_governor().await?,
        SystemCpuSetGovernor { ref governor } => cpu_set_governor(governor).await?,
        SystemUpdate { check, force } => crate::cmd::update::run_json(check, force).await?,
        DbusCall { .. } => execute_dbus_call(&action).await?,
        _ => anyhow::bail!("internal dispatch error: not a system action"),
    })
}

/// Execute a raw D-Bus method call without requiring a desktop backend.
/// Uses dbus-send subprocess — works anywhere D-Bus is available.
pub(crate) async fn execute_dbus_call(action: &Action) -> anyhow::Result<Value> {
    let (bus, service, path, interface, method, args) = match action {
        Action::DbusCall {
            bus,
            service,
            path,
            interface,
            method,
            args,
        } => (bus, service, path, interface, method, args),
        _ => anyhow::bail!("not a dbus.call action"),
    };

    let bus_flag = match bus.as_deref() {
        Some("system") => "--system",
        _ => "--session",
    };

    let mut cmd = tokio::process::Command::new("dbus-send");
    cmd.arg(bus_flag)
        .arg("--print-reply")
        .arg("--dest=".to_string() + service)
        .arg(path)
        .arg(format!("{}.{}", interface, method));

    if let Some(args) = args {
        match args {
            serde_json::Value::Array(arr) => {
                for val in arr {
                    cmd.arg(dbus_send_arg(val));
                }
            }
            other => {
                cmd.arg(dbus_send_arg(other));
            }
        }
    }

    let output = cmd.output().await?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        anyhow::bail!("dbus-send failed: {}", stderr.trim());
    }

    Ok(serde_json::json!({
        "service": service,
        "path": path,
        "interface": interface,
        "method": method,
        "bus": bus.as_deref().unwrap_or("session"),
        "reply": stdout.trim(),
    }))
}

/// Convert a serde_json::Value to a dbus-send argument string.
/// dbus-send expects typed args like: string:hello int32:42 boolean:true
fn dbus_send_arg(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => format!("string:{}", s),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                format!("int32:{}", i)
            } else if let Some(f) = n.as_f64() {
                format!("double:{}", f)
            } else {
                format!("string:{}", n)
            }
        }
        serde_json::Value::Bool(b) => format!("boolean:{}", *b),
        serde_json::Value::Array(arr) => {
            // dbus-send expects array args as space-separated typed values.
            // Nesting arrays isn't supported — flattening would produce malformed args.
            if arr.iter().any(|v| matches!(v, serde_json::Value::Array(_))) {
                return "string:<nested-array-unsupported>".to_string();
            }
            arr.iter().map(dbus_send_arg).collect::<Vec<_>>().join(" ")
        }
        _ => format!("string:{}", value),
    }
}

// ── Pressure Stall Information (PSI) ───────────────────────────────────────

#[derive(serde::Serialize)]
struct PressureStats {
    avg10: f64,
    avg60: f64,
    avg300: f64,
    total: u64,
}

#[derive(serde::Serialize)]
struct ResourcePressure {
    some: PressureStats,
    full: Option<PressureStats>,
}

fn parse_pressure(content: &str) -> anyhow::Result<ResourcePressure> {
    let mut some = None;
    let mut full = None;
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        let stats = PressureStats {
            avg10: parts
                .iter()
                .find(|p| p.starts_with("avg10="))
                .and_then(|p| p.strip_prefix("avg10=")?.parse().ok())
                .unwrap_or(0.0),
            avg60: parts
                .iter()
                .find(|p| p.starts_with("avg60="))
                .and_then(|p| p.strip_prefix("avg60=")?.parse().ok())
                .unwrap_or(0.0),
            avg300: parts
                .iter()
                .find(|p| p.starts_with("avg300="))
                .and_then(|p| p.strip_prefix("avg300=")?.parse().ok())
                .unwrap_or(0.0),
            total: parts
                .iter()
                .find(|p| p.starts_with("total="))
                .and_then(|p| p.strip_prefix("total=")?.parse().ok())
                .unwrap_or(0),
        };
        match parts.first().copied() {
            Some("some") => some = Some(stats),
            Some("full") => full = Some(stats),
            _ => {}
        }
    }
    Ok(ResourcePressure {
        some: some.unwrap_or(PressureStats {
            avg10: 0.0,
            avg60: 0.0,
            avg300: 0.0,
            total: 0,
        }),
        full,
    })
}

/// Read Linux Pressure Stall Information (PSI) from /proc/pressure/.
/// Requires kernel ≥4.20 with CONFIG_PSI. Returns CPU, memory, and IO pressure stats.
async fn system_pressure() -> anyhow::Result<Value> {
    let cpu = tokio::fs::read_to_string("/proc/pressure/cpu")
        .await
        .unwrap_or_default();
    let memory = tokio::fs::read_to_string("/proc/pressure/memory")
        .await
        .unwrap_or_default();
    let io = tokio::fs::read_to_string("/proc/pressure/io")
        .await
        .unwrap_or_default();

    Ok(serde_json::json!({
        "cpu": parse_pressure(&cpu)?,
        "memory": parse_pressure(&memory)?,
        "io": parse_pressure(&io)?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pressure_valid() {
        let input = "some avg10=0.45 avg60=0.30 avg300=0.20 total=123456\nfull avg10=0.10 avg60=0.05 avg300=0.02 total=45678\n";
        let result = parse_pressure(input).unwrap();
        assert!((result.some.avg10 - 0.45).abs() < 0.001);
        assert!((result.some.avg60 - 0.30).abs() < 0.001);
        assert!((result.some.avg300 - 0.20).abs() < 0.001);
        assert_eq!(result.some.total, 123456);
        let full = result.full.unwrap();
        assert!((full.avg10 - 0.10).abs() < 0.001);
        assert_eq!(full.total, 45678);
    }

    #[test]
    fn parse_pressure_full_optional() {
        let input = "some avg10=2.19 avg60=3.46 avg300=3.64 total=92534018857\n";
        let result = parse_pressure(input).unwrap();
        assert!((result.some.avg10 - 2.19).abs() < 0.001);
        assert!(result.full.is_none());
    }
}
