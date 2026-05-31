use crate::protocol::Action;
use serde_json::Value;

pub(super) fn parse_network(raw: &Value, _id: &str, type_str: &str) -> anyhow::Result<Action> {
    Ok(match type_str {
        // Network
        "network.status" => Action::NetworkStatus,
        "network.interfaces" => Action::NetworkInterfaces,
        "network.wifi.scan" => Action::NetworkWifiScan,
        "network.wifi.connect" => Action::NetworkWifiConnect {
            ssid: raw["ssid"].as_str().unwrap_or("").into(),
            password: raw["password"].as_str().map(String::from),
        },
        "network.wifi.enable" => Action::NetworkWifiEnable {
            enabled: raw["enabled"].as_bool().unwrap_or(true),
        },
        "network.wwan.enable" => Action::NetworkWwanEnable {
            enabled: raw["enabled"].as_bool().unwrap_or(true),
        },
        "network.connections.list" => Action::NetworkConnectionList,
        "network.connections.profiles" => Action::NetworkConnectionProfiles,
        "network.hotspot.start" => Action::NetworkCreateHotspot {
            ssid: raw["ssid"].as_str().unwrap_or("").into(),
            password: raw["password"].as_str().map(String::from),
        },
        "network.hotspot.stop" => Action::NetworkStopHotspot,
        "network.dns.set" => Action::NetworkDnsSet {
            dns: raw["dns"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        },
        "network.dns.reset" => Action::NetworkDnsReset,
        "network.vpn.connect" => Action::NetworkVpnConnect {
            profile_name: raw["profile_name"].as_str().unwrap_or("").into(),
        },
        "network.vpn.disconnect" => Action::NetworkVpnDisconnect,
        _ => anyhow::bail!("unknown network type: {type_str}"),
    })
}
