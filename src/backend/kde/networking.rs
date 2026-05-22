use super::*;
use crate::protocol;

pub(super) async fn network_status(
    backend: &KdeBackend,
) -> anyhow::Result<protocol::NetworkStatusInfo> {
    let out = backend
        .sh("nmcli", &["-t", "-f", "STATE", "general"])
        .await?;
    Ok(protocol::NetworkStatusInfo {
        online: out.trim().contains("connected"),
        net_type: String::new(),
    })
}

pub(super) async fn network_interfaces(
    backend: &KdeBackend,
) -> anyhow::Result<Vec<protocol::NetworkInterfaceInfo>> {
    let out = backend
        .sh(
            "nmcli",
            &["-t", "-f", "NAME,TYPE,DEVICE,STATE,IP4", "device", "status"],
        )
        .await?;
    let mut interfaces = Vec::new();
    for line in out.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 3 {
            interfaces.push(protocol::NetworkInterfaceInfo {
                name: parts.first().unwrap_or(&"").to_string(),
                state: parts.get(3).unwrap_or(&"").to_string(),
                ipv4: parts
                    .get(4)
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty()),
                ipv6: None,
            });
        }
    }
    Ok(interfaces)
}

pub(super) async fn wifi_scan(
    backend: &KdeBackend,
) -> anyhow::Result<Vec<protocol::WifiNetworkInfo>> {
    let out = backend
        .sh(
            "nmcli",
            &["-t", "-f", "SSID,SIGNAL,SECURITY", "device", "wifi", "list"],
        )
        .await?;
    let mut networks = Vec::new();
    for line in out.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 3 {
            networks.push(protocol::WifiNetworkInfo {
                ssid: parts[0].to_string(),
                strength: parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0),
                secured: !parts
                    .get(2)
                    .map(|s| s.is_empty() || s.contains("--"))
                    .unwrap_or(true),
                frequency: None,
            });
        }
    }
    Ok(networks)
}

pub(super) async fn wifi_connect(
    backend: &KdeBackend,
    ssid: &str,
    password: Option<&str>,
) -> anyhow::Result<()> {
    let ssid_escaped = ssid.replace('\\', "\\\\").replace('\'', "\\'");
    if let Some(pass) = password {
        backend
            .sh(
                "nmcli",
                &["device", "wifi", "connect", &ssid_escaped, "password", pass],
            )
            .await?;
    } else {
        backend
            .sh("nmcli", &["device", "wifi", "connect", &ssid_escaped])
            .await?;
    }
    Ok(())
}

pub(super) async fn bluetooth_list(
    backend: &KdeBackend,
) -> anyhow::Result<Vec<protocol::BluetoothDeviceInfo>> {
    let out = backend.sh("bluetoothctl", &["devices"]).await?;
    let mut devices = Vec::new();
    for line in out.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 && parts[0] == "Device" {
            devices.push(protocol::BluetoothDeviceInfo {
                address: parts[1].to_string(),
                name: parts[2..].join(" "),
                connected: false,
                paired: false,
                rssi: None,
            });
        }
    }
    Ok(devices)
}

pub(super) async fn bluetooth_scan(
    backend: &KdeBackend,
    _duration: Option<u32>,
) -> anyhow::Result<()> {
    backend.sh("bluetoothctl", &["scan", "on"]).await?;
    Ok(())
}

pub(super) async fn bluetooth_stop_scan(backend: &KdeBackend) -> anyhow::Result<()> {
    backend.sh("bluetoothctl", &["scan", "off"]).await?;
    Ok(())
}

pub(super) async fn bluetooth_connect(backend: &KdeBackend, address: &str) -> anyhow::Result<()> {
    backend.sh("bluetoothctl", &["connect", address]).await?;
    Ok(())
}

pub(super) async fn bluetooth_disconnect(
    backend: &KdeBackend,
    address: &str,
) -> anyhow::Result<()> {
    backend.sh("bluetoothctl", &["disconnect", address]).await?;
    Ok(())
}
