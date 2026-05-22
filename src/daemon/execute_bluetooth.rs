use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::Action;
use serde_json::Value;

pub(crate) async fn execute_bluetooth(
    action: Action,
    backend: &dyn DesktopBackend,
    _state: &DaemonState,
) -> anyhow::Result<Value> {
    use Action::*;
    Ok(match action {
        BluetoothList => serde_json::json!(backend.bluetooth_list().await?),
        BluetoothScan { duration } => {
            backend.bluetooth_scan(duration).await?;
            serde_json::json!({"scanning": true})
        }
        BluetoothStopScan => {
            backend.bluetooth_stop_scan().await?;
            serde_json::json!({"scanning": false})
        }
        BluetoothConnect { ref address } => {
            backend.bluetooth_connect(address).await?;
            serde_json::json!({"connected": address})
        }
        BluetoothDisconnect { ref address } => {
            backend.bluetooth_disconnect(address).await?;
            serde_json::json!({"disconnected": address})
        }

        // BT pair/forget not in trait yet — stub
        BluetoothPair { ref address } => {
            serde_json::json!({"paired": address, "note": "not yet supported"})
        }
        BluetoothForget { ref address } => {
            serde_json::json!({"forgotten": address, "note": "not yet supported"})
        }

        _ => unreachable!("not a bluetooth action"),
    })
}
