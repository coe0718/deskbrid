use super::*;
use crate::protocol;

pub(super) async fn network_status(
    backend: &SwayBackend,
) -> anyhow::Result<protocol::NetworkStatusInfo> {
    let out = backend
        .sh("nmcli", &["-t", "-f", "STATE", "general"])
        .await?;
    let online = out.to_lowercase().contains("connected");
    Ok(protocol::NetworkStatusInfo {
        online,
        net_type: String::new(),
    })
}

pub(super) async fn network_interfaces(
    backend: &SwayBackend,
) -> anyhow::Result<Vec<protocol::NetworkInterfaceInfo>> {
    let out = backend
        .sh("nmcli", &["-t", "-f", "DEVICE,TYPE,STATE", "device"])
        .await?;
    Ok(out
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 {
                Some(protocol::NetworkInterfaceInfo {
                    name: parts[0].to_string(),
                    state: parts.get(1).unwrap_or(&"").to_string(),
                    ipv4: None,
                    ipv6: None,
                })
            } else {
                None
            }
        })
        .collect())
}

pub(super) async fn wifi_scan(
    backend: &SwayBackend,
) -> anyhow::Result<Vec<protocol::WifiNetworkInfo>> {
    let _ = backend.sh("nmcli", &["device", "wifi", "rescan"]).await;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    let out = backend
        .sh(
            "nmcli",
            &["-t", "-f", "SSID,SIGNAL,SECURITY", "device", "wifi", "list"],
        )
        .await?;
    Ok(out
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 && !parts[0].is_empty() {
                Some(protocol::WifiNetworkInfo {
                    ssid: parts[0].to_string(),
                    strength: parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0),
                    secured: parts
                        .get(2)
                        .map(|s| !s.is_empty() && s != &"")
                        .unwrap_or(false),
                    frequency: None,
                })
            } else {
                None
            }
        })
        .collect())
}

pub(super) async fn wifi_connect(
    backend: &SwayBackend,
    ssid: &str,
    password: Option<&str>,
) -> anyhow::Result<()> {
    let mut args = vec!["device", "wifi", "connect", ssid];
    if let Some(pw) = password {
        args.push("password");
        args.push(pw);
    }
    backend.sh("nmcli", &args).await.map(|_| ())
}
