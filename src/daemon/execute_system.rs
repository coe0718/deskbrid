use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::Action;
use serde_json::Value;

use super::{backlight_get, backlight_set, build_system_health, normalize_coords};

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

        _ => unreachable!("not a system action"),
    })
}
