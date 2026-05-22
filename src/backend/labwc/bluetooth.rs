use super::*;
use crate::protocol;

pub(super) async fn bluetooth_list(
    backend: &LabwcBackend,
) -> anyhow::Result<Vec<protocol::BluetoothDeviceInfo>> {
    let o = backend.sh("bluetoothctl", &["devices"]).await?;
    Ok(o.lines()
        .filter_map(|l| {
            let p: Vec<&str> = l.splitn(3, ' ').collect();
            if p.len() >= 3 {
                Some(protocol::BluetoothDeviceInfo {
                    address: p[1].to_string(),
                    name: p[2].to_string(),
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
    backend: &LabwcBackend,
    _unused: Option<u32>,
) -> anyhow::Result<()> {
    backend
        .sh("bluetoothctl", &["scan", "on"])
        .await
        .map(|_| ())
}

pub(super) async fn bluetooth_stop_scan(backend: &LabwcBackend) -> anyhow::Result<()> {
    backend
        .sh("bluetoothctl", &["scan", "off"])
        .await
        .map(|_| ())
}

pub(super) async fn bluetooth_connect(backend: &LabwcBackend, a: &str) -> anyhow::Result<()> {
    backend
        .sh("bluetoothctl", &["connect", a])
        .await
        .map(|_| ())
}

pub(super) async fn bluetooth_disconnect(backend: &LabwcBackend, a: &str) -> anyhow::Result<()> {
    backend
        .sh("bluetoothctl", &["disconnect", a])
        .await
        .map(|_| ())
}
