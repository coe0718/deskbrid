use super::*;
use crate::protocol;

pub(super) async fn network_status(
    backend: &HyprBackend,
) -> anyhow::Result<protocol::NetworkStatusInfo> {
    let online = if backend
        .sh_ok("nmcli", &["networking", "connectivity", "check"])
        .await
    {
        true
    } else {
        backend
            .sh_ok("ping", &["-c", "1", "-W", "2", "8.8.8.8"])
            .await
    };
    Ok(protocol::NetworkStatusInfo {
        online,
        net_type: if online {
            "wifi_or_ethernet".into()
        } else {
            "offline".into()
        },
    })
}

pub(super) async fn network_interfaces(
    backend: &HyprBackend,
) -> anyhow::Result<Vec<protocol::NetworkInterfaceInfo>> {
    let output = backend
        .sh(
            "nmcli",
            &["-t", "-f", "DEVICE,STATE,IP4.ADDRESS", "dev", "status"],
        )
        .await
        .unwrap_or_default();
    let mut ifaces = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() < 2 {
            continue;
        }
        let name = parts[0].to_string();
        if name == "lo" || name.is_empty() {
            continue;
        }
        let state = match *parts.get(1).unwrap_or(&"") {
            "connected" => "connected".to_string(),
            "connecting" => "connecting".to_string(),
            _ => "disconnected".to_string(),
        };
        let ipv4 = parts
            .get(2)
            .filter(|s| !s.is_empty())
            .map(|s| s.split('/').next().unwrap_or(s).to_string());
        ifaces.push(protocol::NetworkInterfaceInfo {
            name,
            state,
            ipv4,
            ipv6: None,
        });
    }
    Ok(ifaces)
}

pub(super) async fn wifi_scan(
    backend: &HyprBackend,
) -> anyhow::Result<Vec<protocol::WifiNetworkInfo>> {
    backend.sh("nmcli", &["dev", "wifi", "rescan"]).await.ok();
    let output = backend
        .sh(
            "nmcli",
            &["-t", "-f", "SSID,SIGNAL,SECURITY", "dev", "wifi", "list"],
        )
        .await
        .unwrap_or_default();
    let mut networks = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.is_empty() || parts[0].is_empty() {
            continue;
        }
        let ssid = parts[0].to_string();
        let signal: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let security = parts.get(2).unwrap_or(&"").to_string();
        networks.push(protocol::WifiNetworkInfo {
            ssid,
            strength: signal,
            secured: !security.is_empty() && security != "--",
            frequency: None,
        });
    }
    Ok(networks)
}

pub(super) async fn wifi_connect(
    backend: &HyprBackend,
    ssid: &str,
    password: Option<&str>,
) -> anyhow::Result<()> {
    match password {
        Some(pw) => {
            backend
                .sh("nmcli", &["dev", "wifi", "connect", ssid, "password", pw])
                .await?;
        }
        None => {
            backend
                .sh("nmcli", &["dev", "wifi", "connect", ssid])
                .await?;
        }
    }
    Ok(())
}
