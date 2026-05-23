use super::*;
use crate::protocol;

pub(super) async fn network_status(
    backend: &X11Backend,
) -> anyhow::Result<protocol::NetworkStatusInfo> {
    let nmcli_status = backend
        .sh("nmcli", &["-t", "-f", "STATE", "general"])
        .await
        .ok();
    let online = nmcli_status
        .as_deref()
        .map(|state| state.to_lowercase().contains("connected"))
        .unwrap_or(false)
        || backend
            .sh_ok("ping", &["-c", "1", "-W", "2", "8.8.8.8"])
            .await;

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
    backend: &X11Backend,
) -> anyhow::Result<Vec<protocol::NetworkInterfaceInfo>> {
    let output = backend
        .sh(
            "nmcli",
            &["-t", "-f", "DEVICE,TYPE,STATE,IP4.ADDRESS", "device"],
        )
        .await?;
    Ok(output
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(':').collect();
            let name = parts.first()?.to_string();
            if name.is_empty() || name == "lo" {
                return None;
            }
            let state = parts.get(2).copied().unwrap_or("unknown").to_string();
            let ipv4 = parts
                .get(3)
                .filter(|s| !s.is_empty())
                .map(|s| s.split('/').next().unwrap_or(s).to_string());
            Some(protocol::NetworkInterfaceInfo {
                name,
                state,
                ipv4,
                ipv6: None,
            })
        })
        .collect())
}

pub(super) async fn wifi_scan(
    backend: &X11Backend,
) -> anyhow::Result<Vec<protocol::WifiNetworkInfo>> {
    backend.sh("nmcli", &["device", "wifi", "rescan"]).await?;
    let output = backend
        .sh(
            "nmcli",
            &["-t", "-f", "SSID,SIGNAL,SECURITY", "device", "wifi", "list"],
        )
        .await?;
    Ok(output
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(':').collect();
            let ssid = parts.first()?.to_string();
            if ssid.is_empty() {
                return None;
            }
            let security = parts.get(2).copied().unwrap_or("");
            Some(protocol::WifiNetworkInfo {
                ssid,
                strength: parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0),
                secured: !security.is_empty() && security != "--",
                frequency: None,
            })
        })
        .collect())
}

pub(super) async fn wifi_connect(
    backend: &X11Backend,
    ssid: &str,
    password: Option<&str>,
) -> anyhow::Result<()> {
    match password {
        Some(password) => {
            backend
                .sh(
                    "nmcli",
                    &["device", "wifi", "connect", ssid, "password", password],
                )
                .await?;
        }
        None => {
            backend
                .sh("nmcli", &["device", "wifi", "connect", ssid])
                .await?;
        }
    }
    Ok(())
}
