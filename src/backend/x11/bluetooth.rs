use super::*;
use crate::protocol;

pub(super) async fn bluetooth_list(
    backend: &X11Backend,
) -> anyhow::Result<Vec<protocol::BluetoothDeviceInfo>> {
    let output = backend.sh("bluetoothctl", &["devices"]).await?;
    let mut devices = Vec::new();

    for line in output.lines() {
        let parts: Vec<&str> = line.splitn(3, ' ').collect();
        if parts.len() < 3 {
            continue;
        }
        let address = parts[1].to_string();
        let info = backend
            .sh("bluetoothctl", &["info", &address])
            .await
            .unwrap_or_default();
        devices.push(protocol::BluetoothDeviceInfo {
            address,
            name: parts[2].to_string(),
            paired: info.lines().any(|line| line.trim() == "Paired: yes"),
            connected: info.lines().any(|line| line.trim() == "Connected: yes"),
            rssi: parse_rssi(&info),
        });
    }

    Ok(devices)
}

pub(super) async fn bluetooth_scan(
    backend: &X11Backend,
    _duration: Option<u32>,
) -> anyhow::Result<()> {
    backend.sh("bluetoothctl", &["scan", "on"]).await?;
    Ok(())
}

pub(super) async fn bluetooth_stop_scan(backend: &X11Backend) -> anyhow::Result<()> {
    backend.sh("bluetoothctl", &["scan", "off"]).await?;
    Ok(())
}

pub(super) async fn bluetooth_connect(backend: &X11Backend, address: &str) -> anyhow::Result<()> {
    backend.sh("bluetoothctl", &["connect", address]).await?;
    Ok(())
}

pub(super) async fn bluetooth_disconnect(
    backend: &X11Backend,
    address: &str,
) -> anyhow::Result<()> {
    backend.sh("bluetoothctl", &["disconnect", address]).await?;
    Ok(())
}

fn parse_rssi(info: &str) -> Option<i32> {
    info.lines().find_map(|line| {
        line.trim()
            .strip_prefix("RSSI: ")
            .and_then(|value| value.parse().ok())
    })
}
