use super::GnomeBackend;
use crate::protocol;
use zbus::zvariant;

impl GnomeBackend {
    pub(super) async fn bluetooth_list_inner(
        &self,
    ) -> anyhow::Result<Vec<protocol::BluetoothDeviceInfo>> {
        let reply = self
            .conn
            .call_method(
                Some("org.bluez"),
                "/",
                Some("org.freedesktop.DBus.ObjectManager"),
                "GetManagedObjects",
                &(),
            )
            .await?;

        let managed: std::collections::HashMap<
            zvariant::OwnedObjectPath,
            std::collections::HashMap<String, zvariant::OwnedValue>,
        > = reply.body().deserialize()?;

        let mut devices = Vec::new();
        for ifaces in managed.values() {
            if !ifaces.contains_key("org.bluez.Device1") {
                continue;
            }
            let props = if let Some(v) = ifaces.get("org.bluez.Device1") {
                if let Ok(map) = v.downcast_ref::<zvariant::Dict>() {
                    map
                } else {
                    continue;
                }
            } else {
                continue;
            };

            let mut address = String::new();
            let mut name = "(unknown)".to_string();
            let mut paired = false;
            let mut connected = false;
            let mut rssi: Option<i32> = None;

            for (pk, pv) in props.iter() {
                let key = if let Ok(s) = pk.downcast_ref::<zvariant::Str>() {
                    s.to_string()
                } else {
                    continue;
                };
                match key.as_str() {
                    "Address" => {
                        if let Ok(s) = pv.downcast_ref::<zvariant::Str>() {
                            address = s.to_string();
                        }
                    }
                    "Name" => {
                        if let Ok(s) = pv.downcast_ref::<zvariant::Str>() {
                            name = s.to_string();
                        }
                    }
                    "Paired" => {
                        if let Ok(b) = pv.downcast_ref::<bool>() {
                            paired = b;
                        }
                    }
                    "Connected" => {
                        if let Ok(b) = pv.downcast_ref::<bool>() {
                            connected = b;
                        }
                    }
                    "RSSI" => {
                        if let Ok(v) = pv.downcast_ref::<i16>() {
                            rssi = Some(v as i32);
                        }
                    }
                    _ => {}
                }
            }

            devices.push(protocol::BluetoothDeviceInfo {
                address,
                name,
                paired,
                connected,
                rssi,
            });
        }
        Ok(devices)
    }

    pub(super) async fn bluetooth_scan_inner(&self, _duration: Option<u32>) -> anyhow::Result<()> {
        let adapter = self.find_bluetooth_adapter().await?;
        self.conn
            .call_method(
                Some("org.bluez"),
                adapter.as_str(),
                Some("org.bluez.Adapter1"),
                "StartDiscovery",
                &(),
            )
            .await?;
        Ok(())
    }

    pub(super) async fn bluetooth_stop_scan_inner(&self) -> anyhow::Result<()> {
        if let Ok(adapter) = self.find_bluetooth_adapter().await {
            let _ = self
                .conn
                .call_method(
                    Some("org.bluez"),
                    adapter.as_str(),
                    Some("org.bluez.Adapter1"),
                    "StopDiscovery",
                    &(),
                )
                .await;
        }
        Ok(())
    }

    pub(super) async fn bluetooth_connect_inner(&self, address: &str) -> anyhow::Result<()> {
        let path = self.device_path(address);
        self.conn
            .call_method(
                Some("org.bluez"),
                path.as_str(),
                Some("org.bluez.Device1"),
                "Connect",
                &(),
            )
            .await?;
        Ok(())
    }

    pub(super) async fn bluetooth_disconnect_inner(&self, address: &str) -> anyhow::Result<()> {
        let path = self.device_path(address);
        let _ = self
            .conn
            .call_method(
                Some("org.bluez"),
                path.as_str(),
                Some("org.bluez.Device1"),
                "Disconnect",
                &(),
            )
            .await;
        Ok(())
    }
}
