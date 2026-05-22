use super::*;
use crate::protocol;

pub(super) async fn bluetooth_list(
    backend: &HyprBackend,
) -> anyhow::Result<Vec<protocol::BluetoothDeviceInfo>> {
    let output = backend
        .sh("bluetoothctl", &["devices"])
        .await
        .unwrap_or_default();
    let mut devices = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.splitn(3, ' ').collect();
        if parts.len() >= 3 {
            devices.push(protocol::BluetoothDeviceInfo {
                name: parts[2].to_string(),
                address: parts[1].to_string(),
                connected: false,
                paired: true,
                rssi: None,
            });
        }
    }
    Ok(devices)
}

pub(super) async fn bluetooth_scan(
    backend: &HyprBackend,
    _duration: Option<u32>,
) -> anyhow::Result<()> {
    backend.sh("bluetoothctl", &["scan", "on"]).await.ok();
    Ok(())
}

pub(super) async fn bluetooth_stop_scan(backend: &HyprBackend) -> anyhow::Result<()> {
    backend.sh("bluetoothctl", &["scan", "off"]).await.ok();
    Ok(())
}

pub(super) async fn bluetooth_connect(backend: &HyprBackend, address: &str) -> anyhow::Result<()> {
    backend.sh("bluetoothctl", &["connect", address]).await?;
    Ok(())
}

pub(super) async fn bluetooth_disconnect(
    backend: &HyprBackend,
    address: &str,
) -> anyhow::Result<()> {
    backend.sh("bluetoothctl", &["disconnect", address]).await?;
    Ok(())
}
