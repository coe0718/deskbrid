use super::*;
use crate::protocol;

pub(super) async fn network_status(
    backend: &CosmicBackend,
) -> anyhow::Result<protocol::NetworkStatusInfo> {
    // Reuse nmcli
    let output = backend
        .sh("nmcli", &["-t", "-f", "STATE", "general"])
        .await?;
    let connected = output.trim().starts_with("connected");
    Ok(protocol::NetworkStatusInfo {
        online: connected,
        net_type: if connected { "ethernet" } else { "none" }.to_string(),
    })
}

pub(super) async fn network_interfaces(
    backend: &CosmicBackend,
) -> anyhow::Result<Vec<protocol::NetworkInterfaceInfo>> {
    let output = backend
        .sh(
            "nmcli",
            &[
                "-t",
                "-f",
                "NAME,TYPE,DEVICE,STATE",
                "connection",
                "show",
                "--active",
            ],
        )
        .await?;
    let interfaces = output
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() < 4 {
                return None;
            }
            Some(protocol::NetworkInterfaceInfo {
                name: parts[0].to_string(),
                state: parts[3].to_string(),
                ipv4: None,
                ipv6: None,
            })
        })
        .collect();
    Ok(interfaces)
}

pub(super) async fn wifi_scan(
    backend: &CosmicBackend,
) -> anyhow::Result<Vec<protocol::WifiNetworkInfo>> {
    let output = backend
        .sh(
            "nmcli",
            &[
                "-t",
                "-f",
                "SSID,BSSID,SIGNAL,SECURITY",
                "device",
                "wifi",
                "list",
            ],
        )
        .await?;
    let networks = output
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() < 4 {
                return None;
            }
            Some(protocol::WifiNetworkInfo {
                ssid: parts[0].to_string(),
                strength: parts[2].parse().unwrap_or(0),
                secured: !parts[3].is_empty(),
                frequency: None,
            })
        })
        .collect();
    Ok(networks)
}

pub(super) async fn wifi_connect(
    backend: &CosmicBackend,
    ssid: &str,
    password: Option<&str>,
) -> anyhow::Result<()> {
    if let Some(pwd) = password {
        backend
            .sh(
                "nmcli",
                &["device", "wifi", "connect", ssid, "password", pwd],
            )
            .await?;
    } else {
        backend
            .sh("nmcli", &["device", "wifi", "connect", ssid])
            .await?;
    }
    Ok(())
}

// ─── Bluetooth ─────────────────────────────────────
