use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::Action;
use serde_json::Value;
use zbus::zvariant;

pub(crate) async fn execute_network(
    action: Action,
    backend: &dyn DesktopBackend,
    _state: &DaemonState,
) -> anyhow::Result<Value> {
    use Action::*;
    Ok(match action {
        NetworkStatus => serde_json::json!(backend.network_status().await?),
        NetworkInterfaces => serde_json::json!(backend.network_interfaces().await?),
        NetworkWifiScan => serde_json::json!(backend.wifi_scan().await?),
        NetworkWifiConnect {
            ref ssid,
            ref password,
        } => {
            backend.wifi_connect(ssid, password.as_deref()).await?;
            serde_json::json!({"connected": ssid})
        }

        // ── D-Bus native operations ──────────────────────
        NetworkConnectionList => nm_connection_list().await?,
        NetworkConnectionProfiles => nm_connection_profiles().await?,
        NetworkCreateHotspot {
            ref ssid,
            ref password,
        } => nm_create_hotspot(ssid, password.as_deref()).await?,
        NetworkStopHotspot => nm_stop_hotspot().await?,
        NetworkWifiEnable { enabled } => nm_wifi_enable(enabled).await?,
        NetworkWwanEnable { enabled } => nm_wwan_enable(enabled).await?,
        NetworkDnsSet { ref dns } => nm_dns_set(dns).await?,
        NetworkDnsReset => nm_dns_reset().await?,
        NetworkVpnConnect { ref profile_name } => nm_vpn_connect(profile_name).await?,
        NetworkVpnDisconnect => nm_vpn_disconnect().await?,

        _ => unreachable!("not a network action"),
    })
}

// ─── Helpers ──────────────────────────────────────────

const NM_SERVICE: &str = "org.freedesktop.NetworkManager";
const NM_PATH: &str = "/org/freedesktop/NetworkManager";
const NM_IFACE: &str = "org.freedesktop.NetworkManager";
const NM_SETTINGS_IFACE: &str = "org.freedesktop.NetworkManager.Settings";
const NM_DEVICE_WIRELESS_IFACE: &str = "org.freedesktop.NetworkManager.Device.Wireless";
const NM_CONNECTION_ACTIVE_IFACE: &str = "org.freedesktop.NetworkManager.Connection.Active";
const NM_ACCESSPOINT_IFACE: &str = "org.freedesktop.NetworkManager.AccessPoint";
const DBUS_PROPERTIES_IFACE: &str = "org.freedesktop.DBus.Properties";

async fn nm_system_connection() -> anyhow::Result<zbus::Connection> {
    zbus::Connection::system()
        .await
        .map_err(|e| anyhow::anyhow!("D-Bus system connection failed: {e}"))
}

/// Call Properties.GetAll on an object path for a given interface, returning the props map
async fn get_all_props(
    conn: &zbus::Connection,
    path: &str,
    iface: &str,
) -> anyhow::Result<std::collections::HashMap<String, zvariant::OwnedValue>> {
    let reply = conn
        .call_method(
            Some(NM_SERVICE),
            path,
            Some(DBUS_PROPERTIES_IFACE),
            "GetAll",
            &(iface,),
        )
        .await?;
    let body = reply.body();
    let props: std::collections::HashMap<String, zvariant::OwnedValue> = body.deserialize()?;
    Ok(props)
}

/// Helper: get a string property value from props map
fn prop_str(props: &std::collections::HashMap<String, zvariant::OwnedValue>, key: &str) -> String {
    props
        .get(key)
        .and_then(|v| v.downcast_ref::<zvariant::Str>().ok())
        .map(|s| s.to_string())
        .unwrap_or_default()
}

/// Helper: get a u32 property value from props map
fn prop_u32(props: &std::collections::HashMap<String, zvariant::OwnedValue>, key: &str) -> u32 {
    props
        .get(key)
        .and_then(|v| v.downcast_ref::<u32>().ok())
        .unwrap_or(0)
}

/// List active connections with details
async fn nm_connection_list() -> anyhow::Result<Value> {
    let conn = nm_system_connection().await?;
    let reply = conn
        .call_method(
            Some(NM_SERVICE),
            NM_PATH,
            Some(DBUS_PROPERTIES_IFACE),
            "Get",
            &(NM_IFACE, "ActiveConnections"),
        )
        .await?;
    let body = reply.body();
    let val: zvariant::Value = body.deserialize()?;
    let paths: Vec<zvariant::OwnedObjectPath> = val.try_into()?;

    let mut connections = Vec::new();
    for path in &paths {
        let props = match get_all_props(&conn, path.as_str(), NM_CONNECTION_ACTIVE_IFACE).await {
            Ok(p) => p,
            Err(_) => continue,
        };

        let id = prop_str(&props, "Id");
        let uuid = prop_str(&props, "Uuid");
        let dev_type = prop_str(&props, "Type");
        let state_val = prop_u32(&props, "State");
        let state = match state_val {
            0 => "unknown",
            1 => "activating",
            2 => "activated",
            3 => "deactivating",
            4 => "deactivated",
            _ => "unknown",
        };

        connections.push(serde_json::json!({
            "id": id,
            "uuid": uuid,
            "type": dev_type,
            "state": state,
            "path": path.as_str(),
        }));
    }
    Ok(serde_json::json!({ "connections": connections }))
}

/// List saved connection profiles (from Settings)
async fn nm_connection_profiles() -> anyhow::Result<Value> {
    let conn = nm_system_connection().await?;
    let reply = conn
        .call_method(
            Some(NM_SERVICE),
            "/org/freedesktop/NetworkManager/Settings",
            Some(NM_SETTINGS_IFACE),
            "ListConnections",
            &(),
        )
        .await?;
    let body = reply.body();
    let val: zvariant::Value = body.deserialize()?;
    let paths: Vec<zvariant::OwnedObjectPath> = val.try_into()?;

    let mut profiles = Vec::new();
    for path in &paths {
        let get_settings: Result<zbus::Message, _> = conn
            .call_method(
                Some(NM_SERVICE),
                path.as_str(),
                Some("org.freedesktop.NetworkManager.Settings.Connection"),
                "GetSettings",
                &(),
            )
            .await;

        let settings_str = match get_settings {
            Ok(reply) => {
                let body = reply.body();
                let bv: zvariant::Value = body.deserialize()?;
                format!("{bv:?}")
            }
            Err(_) => String::new(),
        };

        profiles.push(serde_json::json!({
            "path": path.as_str(),
            "settings": settings_str,
        }));
    }
    Ok(serde_json::json!({ "profiles": profiles }))
}

/// Create a WiFi hotspot by adding and activating a new connection
async fn nm_create_hotspot(ssid: &str, password: Option<&str>) -> anyhow::Result<Value> {
    let conn = nm_system_connection().await?;

    // Find a WiFi device
    let wifi_dev = find_wifi_device(&conn).await?;

    // Build connection settings
    let ssid_bytes: Vec<u8> = ssid.bytes().collect();
    let mut settings: std::collections::HashMap<
        &str,
        std::collections::HashMap<&str, zvariant::Value>,
    > = std::collections::HashMap::new();

    // connection section
    let mut conn_section = std::collections::HashMap::new();
    conn_section.insert("type", zvariant::Value::new("802-11-wireless"));
    conn_section.insert("id", zvariant::Value::new(ssid));
    conn_section.insert("autoconnect", zvariant::Value::new(false));
    settings.insert("connection", conn_section);

    // 802-11-wireless section
    let mut wifi_section = std::collections::HashMap::new();
    wifi_section.insert(
        "ssid",
        zvariant::Value::new(zvariant::Array::from(ssid_bytes)),
    );
    wifi_section.insert("mode", zvariant::Value::new("ap"));
    settings.insert("802-11-wireless", wifi_section);

    // ipv4 section — share network
    let mut ipv4_section = std::collections::HashMap::new();
    ipv4_section.insert("method", zvariant::Value::new("shared"));
    settings.insert("ipv4", ipv4_section);

    // 802-11-wireless-security section (if password)
    if let Some(pw) = password {
        let mut sec_section = std::collections::HashMap::new();
        sec_section.insert("key-mgmt", zvariant::Value::new("wpa-psk"));
        sec_section.insert("psk", zvariant::Value::new(pw));
        settings.insert("802-11-wireless-security", sec_section);
    }

    let reply = conn
        .call_method(
            Some(NM_SERVICE),
            NM_PATH,
            Some(NM_IFACE),
            "AddAndActivateConnection",
            &(settings, wifi_dev, "/"),
        )
        .await?;
    let body = reply.body();
    let result_val: zvariant::Value = body.deserialize()?;

    Ok(serde_json::json!({
        "hotspot": ssid,
        "created": true,
        "result": format!("{result_val:?}"),
    }))
}

/// Stop an active hotspot by deactivating the AP-mode connection
async fn nm_stop_hotspot() -> anyhow::Result<Value> {
    let conn = nm_system_connection().await?;

    // Get active connections
    let reply = conn
        .call_method(
            Some(NM_SERVICE),
            NM_PATH,
            Some(DBUS_PROPERTIES_IFACE),
            "Get",
            &(NM_IFACE, "ActiveConnections"),
        )
        .await?;
    let body = reply.body();
    let val: zvariant::Value = body.deserialize()?;
    let paths: Vec<zvariant::OwnedObjectPath> = val.try_into()?;

    let mut stopped = Vec::new();
    for path in &paths {
        let props = match get_all_props(&conn, path.as_str(), NM_CONNECTION_ACTIVE_IFACE).await {
            Ok(p) => p,
            Err(_) => continue,
        };

        let conn_type = prop_str(&props, "Type");
        if conn_type != "802-11-wireless" {
            continue;
        }

        // Deactivate this connection
        let _ = conn
            .call_method(
                Some(NM_SERVICE),
                NM_PATH,
                Some(NM_IFACE),
                "DeactivateConnection",
                &(path.as_str(),),
            )
            .await;
        stopped.push(path.as_str().to_string());
    }

    Ok(serde_json::json!({
        "stopped": stopped,
        "count": stopped.len(),
    }))
}

/// Enable or disable WiFi
async fn nm_wifi_enable(enabled: bool) -> anyhow::Result<Value> {
    let conn = nm_system_connection().await?;
    conn.call_method(
        Some(NM_SERVICE),
        NM_PATH,
        Some(DBUS_PROPERTIES_IFACE),
        "Set",
        &(NM_IFACE, "WirelessEnabled", &zvariant::Value::new(enabled)),
    )
    .await?;
    Ok(serde_json::json!({
        "wireless_enabled": enabled,
    }))
}

/// Enable or disable WWAN (mobile broadband)
async fn nm_wwan_enable(enabled: bool) -> anyhow::Result<Value> {
    let conn = nm_system_connection().await?;
    conn.call_method(
        Some(NM_SERVICE),
        NM_PATH,
        Some(DBUS_PROPERTIES_IFACE),
        "Set",
        &(NM_IFACE, "WwanEnabled", &zvariant::Value::new(enabled)),
    )
    .await?;
    Ok(serde_json::json!({
        "wwan_enabled": enabled,
    }))
}

/// Set DNS servers via nmcli (global DNS config is complex via raw D-Bus)
async fn nm_dns_set(dns: &[String]) -> anyhow::Result<Value> {
    let active_conns = get_active_connection_names().await?;

    if active_conns.is_empty() {
        anyhow::bail!("no active connections to modify DNS for");
    }

    let dns_str = dns.join(",");
    for name in &active_conns {
        let output = tokio::process::Command::new("nmcli")
            .arg("connection")
            .arg("modify")
            .arg(name)
            .arg("ipv4.dns")
            .arg(&dns_str)
            .arg("ipv4.ignore-auto-dns")
            .arg("yes")
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("failed to set DNS for '{name}': {stderr}");
        }
    }

    // Re-activate connections to apply
    for name in &active_conns {
        let _ = tokio::process::Command::new("nmcli")
            .arg("connection")
            .arg("up")
            .arg(name)
            .output()
            .await;
    }

    Ok(serde_json::json!({
        "dns": dns,
        "applied_to": active_conns,
    }))
}

/// Reset DNS to auto-configuration
async fn nm_dns_reset() -> anyhow::Result<Value> {
    let active_conns = get_active_connection_names().await?;

    for name in &active_conns {
        let output = tokio::process::Command::new("nmcli")
            .arg("connection")
            .arg("modify")
            .arg(name)
            .arg("ipv4.dns")
            .arg("")
            .arg("ipv4.ignore-auto-dns")
            .arg("no")
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("failed to reset DNS for '{name}': {stderr}");
        }
    }

    Ok(serde_json::json!({
        "dns_reset": true,
        "applied_to": active_conns,
    }))
}

/// Connect to a VPN profile by name (uses nmcli)
async fn nm_vpn_connect(profile_name: &str) -> anyhow::Result<Value> {
    let output = tokio::process::Command::new("nmcli")
        .arg("connection")
        .arg("up")
        .arg(profile_name)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("VPN connect failed: {stderr}");
    }

    Ok(serde_json::json!({
        "vpn": profile_name,
        "connected": true,
    }))
}

/// Disconnect all VPN connections
async fn nm_vpn_disconnect() -> anyhow::Result<Value> {
    let conn = nm_system_connection().await?;
    let reply = conn
        .call_method(
            Some(NM_SERVICE),
            NM_PATH,
            Some(DBUS_PROPERTIES_IFACE),
            "Get",
            &(NM_IFACE, "ActiveConnections"),
        )
        .await?;
    let body = reply.body();
    let val: zvariant::Value = body.deserialize()?;
    let paths: Vec<zvariant::OwnedObjectPath> = val.try_into()?;

    let mut disconnected = Vec::new();
    for path in &paths {
        let props = match get_all_props(&conn, path.as_str(), NM_CONNECTION_ACTIVE_IFACE).await {
            Ok(p) => p,
            Err(_) => continue,
        };

        let conn_type = prop_str(&props, "Type");
        let id = prop_str(&props, "Id");

        if conn_type == "vpn" || conn_type.contains("vpn") {
            let _ = conn
                .call_method(
                    Some(NM_SERVICE),
                    NM_PATH,
                    Some(NM_IFACE),
                    "DeactivateConnection",
                    &(path.as_str(),),
                )
                .await;
            disconnected.push(id);
        }
    }

    // Also try nmcli as fallback
    if disconnected.is_empty() {
        let _ = tokio::process::Command::new("nmcli")
            .arg("connection")
            .arg("down")
            .arg("--type")
            .arg("vpn")
            .output()
            .await;
    }

    Ok(serde_json::json!({
        "vpn_disconnected": disconnected,
        "count": disconnected.len(),
    }))
}

/// Get names of active connections
async fn get_active_connection_names() -> anyhow::Result<Vec<String>> {
    let conn = nm_system_connection().await?;
    let reply = conn
        .call_method(
            Some(NM_SERVICE),
            NM_PATH,
            Some(DBUS_PROPERTIES_IFACE),
            "Get",
            &(NM_IFACE, "ActiveConnections"),
        )
        .await?;
    let body = reply.body();
    let val: zvariant::Value = body.deserialize()?;
    let paths: Vec<zvariant::OwnedObjectPath> = val.try_into()?;

    let mut names = Vec::new();
    for path in &paths {
        let props = match get_all_props(&conn, path.as_str(), NM_CONNECTION_ACTIVE_IFACE).await {
            Ok(p) => p,
            Err(_) => continue,
        };
        let id = prop_str(&props, "Id");
        if !id.is_empty() {
            names.push(id);
        }
    }
    Ok(names)
}

/// Find a WiFi device path
async fn find_wifi_device(conn: &zbus::Connection) -> anyhow::Result<String> {
    let reply = conn
        .call_method(Some(NM_SERVICE), NM_PATH, Some(NM_IFACE), "GetDevices", &())
        .await?;

    let body = reply.body();
    let val: zvariant::Value = body.deserialize()?;
    let paths: Vec<zvariant::OwnedObjectPath> = val.try_into()?;

    for path in &paths {
        let dev_type: u32 = conn
            .call_method(
                Some(NM_SERVICE),
                path.as_str(),
                Some(DBUS_PROPERTIES_IFACE),
                "Get",
                &("org.freedesktop.NetworkManager.Device", "DeviceType"),
            )
            .await
            .map(|r| {
                let body = r.body();
                let v: zvariant::Value = body.deserialize().unwrap_or(zvariant::Value::U32(0));
                u32::try_from(v).unwrap_or(0)
            })
            .unwrap_or(0);

        // NM_DEVICE_TYPE_WIFI = 2
        if dev_type == 2 {
            return Ok(path.as_str().to_string());
        }
    }

    anyhow::bail!("no WiFi device found")
}
