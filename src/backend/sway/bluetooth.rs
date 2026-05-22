use super::*;
use crate::protocol;

pub(super) async fn bluetooth_list(
    backend: &SwayBackend,
) -> anyhow::Result<Vec<protocol::BluetoothDeviceInfo>> {
    let out = backend.sh("bluetoothctl", &["devices"]).await?;
    Ok(out
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            if parts.len() >= 3 {
                Some(protocol::BluetoothDeviceInfo {
                    address: parts[1].to_string(),
                    name: parts[2].to_string(),
                    paired: true,
                    connected: false,
                    rssi: None,
                })
            } else {
                None
            }
        })
        .collect())
}

pub(super) async fn bluetooth_scan(
    backend: &SwayBackend,
    _duration: Option<u32>,
) -> anyhow::Result<()> {
    backend
        .sh("bluetoothctl", &["scan", "on"])
        .await
        .map(|_| ())
}

pub(super) async fn bluetooth_stop_scan(backend: &SwayBackend) -> anyhow::Result<()> {
    backend
        .sh("bluetoothctl", &["scan", "off"])
        .await
        .map(|_| ())
}

pub(super) async fn bluetooth_connect(backend: &SwayBackend, address: &str) -> anyhow::Result<()> {
    backend
        .sh("bluetoothctl", &["connect", address])
        .await
        .map(|_| ())
}

pub(super) async fn bluetooth_disconnect(
    backend: &SwayBackend,
    address: &str,
) -> anyhow::Result<()> {
    backend
        .sh("bluetoothctl", &["disconnect", address])
        .await
        .map(|_| ())
}
