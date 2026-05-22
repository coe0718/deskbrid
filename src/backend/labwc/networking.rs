use super::*;
use crate::protocol;

pub(super) async fn network_status(
    backend: &LabwcBackend,
) -> anyhow::Result<protocol::NetworkStatusInfo> {
    let o = backend
        .sh("nmcli", &["-t", "-f", "STATE", "general"])
        .await?;
    Ok(protocol::NetworkStatusInfo {
        online: o.to_lowercase().contains("connected"),
        net_type: String::new(),
    })
}

pub(super) async fn network_interfaces(
    backend: &LabwcBackend,
) -> anyhow::Result<Vec<protocol::NetworkInterfaceInfo>> {
    let o = backend
        .sh("nmcli", &["-t", "-f", "DEVICE,TYPE,STATE", "device"])
        .await?;
    Ok(o.lines()
        .filter_map(|l| {
            let p: Vec<&str> = l.split(':').collect();
            if p.len() >= 2 {
                Some(protocol::NetworkInterfaceInfo {
                    name: p[0].to_string(),
                    state: p.get(1).unwrap_or(&"").to_string(),
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
    backend: &LabwcBackend,
) -> anyhow::Result<Vec<protocol::WifiNetworkInfo>> {
    let _ = backend.sh("nmcli", &["device", "wifi", "rescan"]).await;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    let o = backend
        .sh(
            "nmcli",
            &["-t", "-f", "SSID,SIGNAL,SECURITY", "device", "wifi", "list"],
        )
        .await?;
    Ok(o.lines()
        .filter_map(|l| {
            let p: Vec<&str> = l.split(':').collect();
            if p.len() >= 2 && !p[0].is_empty() {
                Some(protocol::WifiNetworkInfo {
                    ssid: p[0].to_string(),
                    strength: p.get(1).and_then(|s| s.parse().ok()).unwrap_or(0),
                    secured: p.get(2).map(|s| !s.is_empty() && s != &"").unwrap_or(false),
                    frequency: None,
                })
            } else {
                None
            }
        })
        .collect())
}

pub(super) async fn wifi_connect(
    backend: &LabwcBackend,
    ssid: &str,
    pw: Option<&str>,
) -> anyhow::Result<()> {
    let mut a = vec!["device", "wifi", "connect", ssid];
    if let Some(p) = pw {
        a.push("password");
        a.push(p);
    }
    backend.sh("nmcli", &a).await.map(|_| ())
}
