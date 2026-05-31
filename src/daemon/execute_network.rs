// reason: 540+ lines — NM D-Bus methods plus nmcli fallbacks
// NetworkManager D-Bus integration (#62).
// Uses a hybrid approach: zbus for simple property/method calls,
// nmcli for complex operations (hotspot, VPN, DNS).

use crate::protocol::Action;
use anyhow::Context;
use serde_json::Value;

/// Check if an action is a network-related action.
#[allow(dead_code)]
pub(crate) fn is_network_action(action: &Action) -> bool {
    matches!(
        action,
        Action::NetworkStatus
            | Action::NetworkInterfaces
            | Action::NetworkWifiScan
            | Action::NetworkWifiConnect { .. }
            | Action::NetworkConnectionList
            | Action::NetworkConnectionProfiles
            | Action::NetworkCreateHotspot { .. }
            | Action::NetworkStopHotspot
            | Action::NetworkWifiEnable { .. }
            | Action::NetworkWwanEnable { .. }
            | Action::NetworkDnsSet { .. }
            | Action::NetworkDnsReset
            | Action::NetworkVpnConnect { .. }
            | Action::NetworkVpnDisconnect
    )
}

pub(crate) async fn execute_network(
    action: Action,
    _backend: &dyn crate::backend::DesktopBackend,
    _state: &crate::DaemonState,
) -> anyhow::Result<Value> {
    use Action::*;
    Ok(match action {
        NetworkStatus => nm_status().await?,
        NetworkInterfaces => nm_interfaces().await?,
        NetworkWifiScan => nm_wifi_scan().await?,
        NetworkWifiConnect { ssid, password } => {
            nm_wifi_connect(&ssid, password.as_deref()).await?
        }
        NetworkConnectionList => nm_connection_list().await?,
        NetworkConnectionProfiles => nm_connection_profiles().await?,
        NetworkCreateHotspot { ssid, password } => {
            nm_create_hotspot(&ssid, password.as_deref()).await?
        }
        NetworkStopHotspot => nm_stop_hotspot().await?,
        NetworkWifiEnable { enabled } => nm_wifi_enable(enabled).await?,
        NetworkWwanEnable { enabled } => nm_wwan_enable(enabled).await?,
        NetworkDnsSet { ref dns } => nm_dns_set(dns).await?,
        NetworkDnsReset => nm_dns_reset().await?,
        NetworkVpnConnect { ref profile_name } => nm_vpn_connect(profile_name).await?,
        NetworkVpnDisconnect => nm_vpn_disconnect().await?,
        _ => anyhow::bail!("not a network action"),
    })
}

// ── nmcli-backed helpers (reliable, already proven) ──────

async fn run_nmcli(args: &[&str]) -> anyhow::Result<String> {
    let output = tokio::process::Command::new("nmcli")
        .args(args)
        .output()
        .await
        .context("nmcli not found — is NetworkManager installed?")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("nmcli failed: {}", stderr.trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

async fn nm_status() -> anyhow::Result<Value> {
    let out = run_nmcli(&["-t", "-f", "STATE", "general", "status"]).await?;
    let state = out.trim().to_string();
    Ok(serde_json::json!({"status": state}))
}

async fn nm_interfaces() -> anyhow::Result<Value> {
    let out = run_nmcli(&[
        "-t",
        "-f",
        "DEVICE,TYPE,STATE,CONNECTION",
        "device",
        "status",
    ])
    .await?;
    let ifaces: Vec<Value> = out
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 4 {
                Some(serde_json::json!({
                    "device": parts[0],
                    "type": parts[1],
                    "state": parts[2],
                    "connection": parts[3],
                }))
            } else {
                None
            }
        })
        .collect();
    Ok(serde_json::json!({"interfaces": ifaces}))
}

async fn nm_wifi_scan() -> anyhow::Result<Value> {
    let _ = run_nmcli(&["device", "wifi", "rescan"]).await;
    let out = run_nmcli(&[
        "-t",
        "-f",
        "SSID,BSSID,MODE,CHAN,FREQ,SIGNAL,SECURITY",
        "device",
        "wifi",
        "list",
    ])
    .await?;
    let networks: Vec<Value> = out
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 7 {
                Some(serde_json::json!({
                    "ssid": parts[0],
                    "bssid": parts[1],
                    "mode": parts[2],
                    "channel": parts[3],
                    "freq": parts[4],
                    "signal": parts[5],
                    "security": parts[6],
                }))
            } else {
                None
            }
        })
        .collect();
    Ok(serde_json::json!({"networks": networks}))
}

async fn nm_wifi_connect(ssid: &str, password: Option<&str>) -> anyhow::Result<Value> {
    let mut args = vec!["device", "wifi", "connect", ssid];
    if let Some(pw) = password {
        args.push("password");
        args.push(pw);
    }
    run_nmcli(&args).await?;
    Ok(serde_json::json!({"connected": ssid}))
}

// ── New NetworkManager features (#62) ────────────────────

async fn nm_connection_list() -> anyhow::Result<Value> {
    let out = run_nmcli(&[
        "-t",
        "-f",
        "NAME,UUID,TYPE,DEVICE",
        "connection",
        "show",
        "--active",
    ])
    .await?;
    let connections: Vec<Value> = out
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                Some(serde_json::json!({
                    "name": parts[0],
                    "uuid": parts[1],
                    "type": parts[2],
                    "device": parts.get(3).unwrap_or(&""),
                }))
            } else {
                None
            }
        })
        .collect();
    Ok(serde_json::json!({"connections": connections}))
}

async fn nm_connection_profiles() -> anyhow::Result<Value> {
    let out = run_nmcli(&[
        "-t",
        "-f",
        "NAME,UUID,TYPE,AUTOCONNECT",
        "connection",
        "show",
    ])
    .await?;
    let profiles: Vec<Value> = out
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                Some(serde_json::json!({
                    "name": parts[0],
                    "uuid": parts[1],
                    "type": parts[2],
                    "autoconnect": parts.get(3).unwrap_or(&"no"),
                }))
            } else {
                None
            }
        })
        .collect();
    Ok(serde_json::json!({"profiles": profiles}))
}

async fn nm_create_hotspot(ssid: &str, password: Option<&str>) -> anyhow::Result<Value> {
    let mut args = vec!["device", "wifi", "hotspot", "ifname", "wlan0", "ssid", ssid];
    if let Some(pw) = password {
        args.push("password");
        args.push(pw);
    }
    run_nmcli(&args).await?;
    Ok(serde_json::json!({"hotspot": ssid, "created": true}))
}

async fn nm_stop_hotspot() -> anyhow::Result<Value> {
    // Find and turn off hotspot connection
    let _ = tokio::process::Command::new("nmcli")
        .args(["connection", "down", "Hotspot"])
        .output()
        .await;
    Ok(serde_json::json!({"stopped": true}))
}

async fn nm_wifi_enable(enabled: bool) -> anyhow::Result<Value> {
    let action = if enabled { "on" } else { "off" };
    run_nmcli(&["radio", "wifi", action]).await?;
    Ok(serde_json::json!({"wireless_enabled": enabled}))
}

async fn nm_wwan_enable(enabled: bool) -> anyhow::Result<Value> {
    let action = if enabled { "on" } else { "off" };
    run_nmcli(&["radio", "wwan", action]).await?;
    Ok(serde_json::json!({"wwan_enabled": enabled}))
}

async fn nm_dns_set(dns: &[String]) -> anyhow::Result<Value> {
    let active = nm_connection_list().await?;
    let conns = active["connections"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let dns_str = dns.join(",");
    let mut applied = Vec::new();
    for conn in &conns {
        let name = conn["name"].as_str().unwrap_or("");
        if name.is_empty() {
            continue;
        }
        run_nmcli(&[
            "connection",
            "modify",
            name,
            "ipv4.dns",
            &dns_str,
            "ipv4.ignore-auto-dns",
            "yes",
        ])
        .await?;
        applied.push(name.to_string());
    }
    // Re-activate
    for name in &applied {
        let _ = run_nmcli(&["connection", "up", name]).await;
    }
    Ok(serde_json::json!({"dns": dns, "applied_to": applied}))
}

async fn nm_dns_reset() -> anyhow::Result<Value> {
    let active = nm_connection_list().await?;
    let conns = active["connections"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut applied = Vec::new();
    for conn in &conns {
        let name = conn["name"].as_str().unwrap_or("");
        if name.is_empty() {
            continue;
        }
        run_nmcli(&[
            "connection",
            "modify",
            name,
            "ipv4.dns",
            "",
            "ipv4.ignore-auto-dns",
            "no",
        ])
        .await?;
        applied.push(name.to_string());
    }
    Ok(serde_json::json!({"dns_reset": true, "applied_to": applied}))
}

async fn nm_vpn_connect(profile_name: &str) -> anyhow::Result<Value> {
    run_nmcli(&["connection", "up", profile_name]).await?;
    Ok(serde_json::json!({"vpn": profile_name, "connected": true}))
}

async fn nm_vpn_disconnect() -> anyhow::Result<Value> {
    let _ = run_nmcli(&["connection", "down", "--type", "vpn"]).await;
    Ok(serde_json::json!({"vpn_disconnected": true}))
}
