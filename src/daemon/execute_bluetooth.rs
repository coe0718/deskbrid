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

        // BT pair/forget using bluetoothctl (universal across distros)
        BluetoothPair { ref address } => {
            let output = tokio::process::Command::new("bluetoothctl")
                .args(["pair", address])
                .output()
                .await?;
            let ok = output.status.success();
            let note = if ok {
                "paired"
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.contains("not available") || stderr.contains("No default controller") {
                    "no bluetooth adapter available"
                } else {
                    "pair failed"
                }
            };
            serde_json::json!({"paired": address, "ok": ok, "note": note})
        }
        BluetoothForget { ref address } => {
            let output = tokio::process::Command::new("bluetoothctl")
                .args(["remove", address])
                .output()
                .await?;
            let ok = output.status.success();
            let note = if ok { "forgotten" } else { "forget failed" };
            serde_json::json!({"forgotten": address, "ok": ok, "note": note})
        }

        _ => unreachable!("not a bluetooth action"),
    })
}
