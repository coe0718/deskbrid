use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::Action;
use serde_json::Value;

pub(crate) async fn execute_network(
    action: Action,
    backend: &dyn DesktopBackend,
    _state: &DaemonState,
) -> anyhow::Result<Value> {
    use Action::*;
    Ok(match action {
        NetworkStatus => serde_json::json!(backend.network_status().await?),
        NetworkInterfaces => serde_json::json!(backend.network_interfaces().await?),
        NetworkWifiScan => serde_json::json!(backend.wifi_scan().await?),
        NetworkWifiConnect {
            ref ssid,
            ref password,
        } => {
            backend.wifi_connect(ssid, password.as_deref()).await?;
            serde_json::json!({"connected": ssid})
        }

        _ => unreachable!("not a network action"),
    })
}
