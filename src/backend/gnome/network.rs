use super::GnomeBackend;
use crate::protocol;
use zbus::zvariant;

impl GnomeBackend {
    pub(super) async fn network_status_inner(&self) -> anyhow::Result<protocol::NetworkStatusInfo> {
        let online = match self
            .conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                "/org/freedesktop/NetworkManager",
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &("org.freedesktop.NetworkManager", "State"),
            )
            .await
        {
            Ok(reply) => {
                let state: u32 = reply.body().deserialize().unwrap_or(0);
                state >= 70
            }
            Err(_) => self.sh_ok("ping", &["-c", "1", "-W", "2", "8.8.8.8"]).await,
        };

        Ok(protocol::NetworkStatusInfo {
            online,
            net_type: if online {
                "ethernet_or_wifi".into()
            } else {
                "offline".into()
            },
        })
    }

    pub(super) async fn network_interfaces_inner(
        &self,
    ) -> anyhow::Result<Vec<protocol::NetworkInterfaceInfo>> {
        let reply = match self
            .conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                "/org/freedesktop/NetworkManager",
                Some("org.freedesktop.NetworkManager"),
                "GetDevices",
                &(),
            )
            .await
        {
            Ok(r) => r,
            Err(_) => {
                let out = self.sh("cat", &["/proc/net/dev"]).await.unwrap_or_default();
                let mut ifaces = Vec::new();
                for line in out.lines().skip(2) {
                    let name = line.split(':').next().unwrap_or("").trim();
                    if name.is_empty() || name == "lo" {
                        continue;
                    }
                    ifaces.push(protocol::NetworkInterfaceInfo {
                        name: name.to_string(),
                        state: "up".into(),
                        ipv4: None,
                        ipv6: None,
                    });
                }
                return Ok(ifaces);
            }
        };

        let paths: Vec<zvariant::OwnedObjectPath> = reply.body().deserialize()?;
        let mut ifaces = Vec::new();

        for path in &paths {
            let path_str = path.as_str();
            let props: std::collections::HashMap<String, zvariant::OwnedValue> = match self
                .conn
                .call_method(
                    Some("org.freedesktop.NetworkManager"),
                    path_str,
                    Some("org.freedesktop.DBus.Properties"),
                    "GetAll",
                    &("org.freedesktop.NetworkManager.Device",),
                )
                .await
            {
                Ok(r) => r.body().deserialize().unwrap_or_default(),
                Err(_) => continue,
            };

            let name = if let Some(v) = props.get("Interface") {
                if let Ok(s) = v.downcast_ref::<zvariant::Str>() {
                    s.to_string()
                } else {
                    path_str.to_string()
                }
            } else {
                path_str.to_string()
            };

            let state_num: u32 = props
                .get("State")
                .and_then(|v| v.downcast_ref::<u32>().ok())
                .unwrap_or(0);
            let state = match state_num {
                100 => "connected",
                70 => "connecting",
                50 | 60 => "disconnected",
                _ => "unknown",
            };

            let ipv4 = match props.get("Ip4Config") {
                Some(v) => {
                    if let Ok(obj) = v.downcast_ref::<zvariant::ObjectPath>() {
                        self.get_nm_ip4_address(obj.as_str()).await
                    } else {
                        None
                    }
                }
                None => None,
            };

            ifaces.push(protocol::NetworkInterfaceInfo {
                name,
                state: state.into(),
                ipv4,
                ipv6: None,
            });
        }
        Ok(ifaces)
    }

    pub(super) async fn wifi_scan_inner(&self) -> anyhow::Result<Vec<protocol::WifiNetworkInfo>> {
        let reply = self
            .conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                "/org/freedesktop/NetworkManager",
                Some("org.freedesktop.NetworkManager"),
                "GetDevices",
                &(),
            )
            .await?;
        let all_paths: Vec<zvariant::OwnedObjectPath> = reply.body().deserialize()?;
        let mut networks = Vec::new();

        for path in &all_paths {
            let path_str = path.as_str();
            let device_type: u32 = match self.get_nm_property(path_str, "DeviceType").await {
                Ok(t) => t,
                Err(_) => continue,
            };
            if device_type != 2 {
                continue;
            }

            let _ = self
                .conn
                .call_method(
                    Some("org.freedesktop.NetworkManager"),
                    path_str,
                    Some("org.freedesktop.NetworkManager.Device.Wireless"),
                    "RequestScan",
                    &(std::collections::HashMap::<&str, zvariant::Value>::new(),),
                )
                .await;

            let ap_reply = self
                .conn
                .call_method(
                    Some("org.freedesktop.NetworkManager"),
                    path_str,
                    Some("org.freedesktop.NetworkManager.Device.Wireless"),
                    "GetAccessPoints",
                    &(),
                )
                .await?;
            let ap_paths: Vec<zvariant::OwnedObjectPath> = ap_reply.body().deserialize()?;

            for ap_path in &ap_paths {
                let props: std::collections::HashMap<String, zvariant::OwnedValue> = match self
                    .conn
                    .call_method(
                        Some("org.freedesktop.NetworkManager"),
                        ap_path.as_str(),
                        Some("org.freedesktop.DBus.Properties"),
                        "GetAll",
                        &("org.freedesktop.NetworkManager.AccessPoint",),
                    )
                    .await
                {
                    Ok(r) => r.body().deserialize().unwrap_or_default(),
                    Err(_) => continue,
                };

                let ssid = if let Some(v) = props.get("Ssid") {
                    if let Ok(arr) = v.downcast_ref::<zvariant::Array>() {
                        let bytes: Vec<u8> = arr
                            .iter()
                            .filter_map(|v| v.downcast_ref::<u8>().ok())
                            .collect();
                        String::from_utf8_lossy(&bytes).to_string()
                    } else {
                        "(hidden)".into()
                    }
                } else {
                    "(hidden)".into()
                };

                let strength: u32 = props
                    .get("Strength")
                    .and_then(|v| v.downcast_ref::<u8>().ok())
                    .map(|s| s as u32)
                    .unwrap_or(0);
                let flags: u32 = props
                    .get("Flags")
                    .and_then(|v| v.downcast_ref::<u32>().ok())
                    .unwrap_or(0);
                let secured = (flags & 0x1) != 0;
                let frequency: Option<u32> = props
                    .get("Frequency")
                    .and_then(|v| v.downcast_ref::<u32>().ok());

                networks.push(protocol::WifiNetworkInfo {
                    ssid,
                    strength,
                    secured,
                    frequency,
                });
            }
        }
        Ok(networks)
    }

    pub(super) async fn wifi_connect_inner(
        &self,
        ssid: &str,
        password: Option<&str>,
    ) -> anyhow::Result<()> {
        let mut args = vec!["device", "wifi", "connect", ssid];
        if let Some(pw) = password {
            args.push("password");
            args.push(pw);
        }
        self.sh("nmcli", &args).await?;
        Ok(())
    }
}
