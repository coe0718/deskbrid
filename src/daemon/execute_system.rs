use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::Action;
use serde_json::Value;

use super::{
    backlight_get, backlight_set, build_system_health, cpu_frequency, cpu_governor,
    cpu_set_governor, normalize_coords, thermal_get,
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
        SystemBacklightGet { ref device } => backlight_get(device.as_deref()).await?,
        SystemBacklightSet {
            percent,
            ref device,
        } => backlight_set(percent, device.as_deref()).await?,
        SystemThermalGet => thermal_get().await?,
        SystemCpuFrequency => cpu_frequency().await?,
        SystemCpuGovernor => cpu_governor().await?,
        SystemCpuSetGovernor { ref governor } => cpu_set_governor(governor).await?,
        SystemUpdate { check, force } => crate::cmd::update::run_json(check, force).await?,

        DbusCall {
            bus,
            service,
            path,
            interface,
            method,
            args,
        } => {
            // Raw D-Bus call via dbus-send subprocess.
            // This is the escape hatch — dbus-send handles all type marshalling.
            let bus_flag = match bus.as_deref() {
                Some("system") => "--system",
                _ => "--session",
            };

            let mut cmd = tokio::process::Command::new("dbus-send");
            cmd.arg(bus_flag)
                .arg("--print-reply")
                .arg("--dest=".to_string() + &service)
                .arg(&path)
                .arg(format!("{}.{}", interface, method));

            // Append args as dbus-send typed parameters
            if let Some(ref args) = args {
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

            serde_json::json!({
                "service": service,
                "path": path,
                "interface": interface,
                "method": method,
                "bus": bus.as_deref().unwrap_or("session"),
                "reply": stdout.trim(),
            })
        }

        _ => unreachable!("not a system action"),
    })
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
            // Recursively format array elements — dbus-send doesn't handle nested arrays well
            // so we just join the first layer
            arr.iter().map(dbus_send_arg).collect::<Vec<_>>().join(" ")
        }
        _ => format!("string:{}", value),
    }
}
