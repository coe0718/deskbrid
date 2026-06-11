use super::Action;
use serde_json::json;

pub(super) fn serialize_bluetooth(action: &Action, id: &str) -> serde_json::Value {
    match action {
        // Bluetooth
        Action::BluetoothList => json!({"type": "bluetooth.list", "id": id}),
        Action::BluetoothScan { duration } => {
            let mut obj = json!({"type": "bluetooth.scan", "id": id});
            if let Some(d) = duration {
                obj["duration"] = json!(d);
            }
            obj
        }
        Action::BluetoothStopScan => json!({"type": "bluetooth.scan_stop", "id": id}),
        Action::BluetoothConnect { address } => {
            json!({"type": "bluetooth.connect", "id": id, "address": address})
        }
        Action::BluetoothDisconnect { address } => {
            json!({"type": "bluetooth.disconnect", "id": id, "address": address})
        }
        Action::BluetoothPair { address } => {
            json!({"type": "bluetooth.pair", "id": id, "address": address})
        }
        Action::BluetoothForget { address } => {
            json!({"type": "bluetooth.forget", "id": id, "address": address})
        }
        _ => serde_json::json!({"error": "not a bluetooth action"}),
    }
}
