use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::Action;
use serde_json::Value;

pub(crate) async fn execute_monitor(
    action: Action,
    backend: &dyn DesktopBackend,
    _state: &DaemonState,
) -> anyhow::Result<Value> {
    use Action::*;
    Ok(match action {
        MonitorList => serde_json::json!(backend.system_info().await?.monitors),
        MonitorSetPrimary { ref output } => {
            backend.monitor_set_primary(output).await?;
            serde_json::json!({"output": output, "primary": true})
        }
        MonitorSetResolution {
            ref output,
            width,
            height,
            refresh_rate,
        } => {
            backend
                .monitor_set_resolution(output, width, height, refresh_rate)
                .await?;
            serde_json::json!({
                "output": output,
                "width": width,
                "height": height,
                "refresh_rate": refresh_rate
            })
        }
        MonitorSetScale { ref output, scale } => {
            backend.monitor_set_scale(output, scale).await?;
            serde_json::json!({"output": output, "scale": scale})
        }
        MonitorSetRotation {
            ref output,
            ref rotation,
        } => {
            backend.monitor_set_rotation(output, rotation).await?;
            serde_json::json!({"output": output, "rotation": rotation})
        }
        MonitorEnable { ref output } => {
            backend.monitor_set_enabled(output, true).await?;
            serde_json::json!({"output": output, "enabled": true})
        }
        MonitorDisable { ref output } => {
            backend.monitor_set_enabled(output, false).await?;
            serde_json::json!({"output": output, "enabled": false})
        }
        _ => anyhow::bail!("internal dispatch error: not a monitor action"),
    })
}
