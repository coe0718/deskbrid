use super::*;
use crate::protocol;
use zbus::zvariant;

/// Probe DRM connectors for connected monitors.
/// Returns monitor info with real connector names and resolutions.
fn probe_drm_monitors() -> Vec<protocol::MonitorInfo> {
    let mut monitors = Vec::new();
    let drm_path = std::path::Path::new("/sys/class/drm");
    let dir = match std::fs::read_dir(drm_path) {
        Ok(d) => d,
        Err(_) => return monitors,
    };
    let mut id: u32 = 0;
    for entry in dir.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.contains('-') {
            continue;
        }
        let connector = name.split_once('-').map(|x| x.1).unwrap_or(&name);
        let status_path = entry.path().join("status");
        let status = match std::fs::read_to_string(&status_path) {
            Ok(s) => s.trim().to_string(),
            Err(_) => continue,
        };
        if status != "connected" {
            continue;
        }
        let modes_path = entry.path().join("modes");
        let mut width = 1920u32;
        let mut height = 1080u32;
        if let Ok(modes) = std::fs::read_to_string(&modes_path)
            && let Some(first_mode) = modes.lines().next()
            && let Some(x_pos) = first_mode.find('x')
            && let (Ok(w), Ok(h)) = (
                first_mode[..x_pos].parse::<u32>(),
                first_mode[x_pos + 1..].parse::<u32>(),
            )
        {
            width = w;
            height = h;
        }
        let is_primary = id == 0;
        monitors.push(protocol::MonitorInfo {
            id,
            name: connector.to_string(),
            width,
            height,
            scale: 1.0,
            primary: is_primary,
            enabled: true,
            x: 0,
            y: 0,
            refresh_rate: None,
            rotation: "normal".into(),
        });
        id += 1;
    }
    monitors
}

impl GnomeBackend {
    pub(super) async fn idle_seconds_inner(&self) -> anyhow::Result<u64> {
        let reply = self
            .conn
            .call_method(
                Some("org.gnome.Mutter.IdleMonitor"),
                "/org/gnome/Mutter/IdleMonitor/Core",
                Some("org.gnome.Mutter.IdleMonitor"),
                "GetIdletime",
                &(),
            )
            .await?;
        let ms: u64 = reply.body().deserialize()?;
        Ok(ms / 1000)
    }

    pub(super) async fn get_monitors(&self) -> anyhow::Result<Vec<protocol::MonitorInfo>> {
        let mut monitors = Vec::new();
        if let Ok(out) = self.sh("gnome-randr", &[]).await {
            super::parsers::parse_gnome_randr(&out, &mut monitors);
            if !monitors.is_empty() {
                return Ok(monitors);
            }
        }
        if let Ok(out) = self.sh("wlr-randr", &[]).await {
            super::parsers::parse_wlr_randr(&out, &mut monitors);
            if !monitors.is_empty() {
                return Ok(monitors);
            }
        }
        // Fallback: probe DRM connectors (works on any Linux)
        let drm_monitors = probe_drm_monitors();
        if !drm_monitors.is_empty() {
            return Ok(drm_monitors);
        }
        monitors.push(protocol::MonitorInfo {
            id: 0,
            name: "Unknown".into(),
            width: 1920,
            height: 1080,
            scale: 1.0,
            primary: true,
            enabled: true,
            x: 0,
            y: 0,
            refresh_rate: None,
            rotation: "normal".into(),
        });
        Ok(monitors)
    }

    pub(super) async fn get_workspace_count(&self) -> anyhow::Result<u32> {
        if let Ok(raw) = self.ext_call_parsed("WorkspacesList", &[]).await {
            let count = raw.matches("('").count() as u32;
            if count > 0 {
                return Ok(count);
            }
        }
        Ok(1)
    }

    pub(super) async fn get_current_workspace(&self) -> anyhow::Result<u32> {
        if let Ok(raw) = self.ext_call_parsed("ActiveWorkspace", &[]).await
            && let Some(start) = raw.find("uint32 ")
        {
            let num_str = &raw[start + 7..];
            if let Some(end) = num_str.find(|c: char| !c.is_ascii_digit()) {
                return Ok(num_str[..end].parse().unwrap_or(0));
            }
        }
        Ok(0)
    }

    pub(super) async fn get_upower_property<
        T: serde::de::DeserializeOwned + zbus::zvariant::Type,
    >(
        &self,
        path: &str,
        prop: &str,
    ) -> anyhow::Result<T> {
        let reply = self
            .conn
            .call_method(
                Some("org.freedesktop.UPower"),
                path,
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &("org.freedesktop.UPower.Device", prop),
            )
            .await?;
        Ok(reply.body().deserialize()?)
    }

    pub(super) async fn get_nm_property<T: serde::de::DeserializeOwned + zbus::zvariant::Type>(
        &self,
        path: &str,
        prop: &str,
    ) -> anyhow::Result<T> {
        let reply = self
            .conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                path,
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &("org.freedesktop.NetworkManager.Device", prop),
            )
            .await?;
        Ok(reply.body().deserialize()?)
    }

    pub(super) async fn get_nm_ip4_address(&self, config_path: &str) -> Option<String> {
        let reply = self
            .conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                config_path,
                Some("org.freedesktop.DBus.Properties"),
                "GetAll",
                &("org.freedesktop.NetworkManager.IP4Config",),
            )
            .await
            .ok()?;
        let props: std::collections::HashMap<String, zvariant::OwnedValue> =
            reply.body().deserialize().ok()?;
        let addresses = props.get("AddressData")?;
        let arr = addresses.downcast_ref::<zvariant::Array>().ok()?;
        for entry in arr.iter() {
            if let Ok(inner) = entry.downcast_ref::<zvariant::Structure>()
                && let Some(v) = inner.fields().first()
                && let Ok(s) = v.downcast_ref::<zvariant::Str>()
            {
                return Some(s.to_string());
            }
        }
        None
    }

    pub(super) async fn find_bluetooth_adapter(&self) -> anyhow::Result<String> {
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
        for (path, ifaces) in &managed {
            if ifaces.contains_key("org.bluez.Adapter1") {
                return Ok(path.as_str().to_string());
            }
        }
        anyhow::bail!("no Bluetooth adapter found")
    }

    pub(super) fn device_path(&self, address: &str) -> String {
        format!(
            "/org/bluez/hci0/dev_{}",
            address.replace(':', "_").to_uppercase()
        )
    }
}
