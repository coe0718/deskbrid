use super::GnomeBackend;
use crate::protocol;

impl GnomeBackend {
    pub(super) async fn system_info_inner(&self) -> anyhow::Result<protocol::SystemInfo> {
        let _hostname = self.sh("hostname", &[]).await.unwrap_or_default();
        let version = self
            .sh("gnome-shell", &["--version"])
            .await
            .unwrap_or_else(|_| "unknown".into());
        let version = version
            .strip_prefix("GNOME Shell ")
            .unwrap_or(&version)
            .to_string();

        let session_type = if std::env::var("WAYLAND_DISPLAY").is_ok() {
            "wayland"
        } else if std::env::var("DISPLAY").is_ok() {
            "x11"
        } else {
            "unknown"
        };

        let monitors = self.get_monitors().await?;
        let workspace_count = self.get_workspace_count().await?;
        let current_workspace = self.get_current_workspace().await?;
        let idle_seconds = self.idle_seconds_inner().await.unwrap_or(0);

        Ok(protocol::SystemInfo {
            desktop: "GNOME".into(),
            desktop_version: version,
            compositor: "mutter".into(),
            session_type: session_type.into(),
            monitors,
            workspace_count,
            current_workspace,
            idle_seconds,
        })
    }

    pub(super) async fn power_action_inner(&self, action: &str) -> anyhow::Result<()> {
        match action {
            "suspend" => {
                self.sh("systemctl", &["suspend"]).await?;
            }
            "hibernate" => {
                self.sh("systemctl", &["hibernate"]).await?;
            }
            "shutdown" | "poweroff" => {
                self.sh("systemctl", &["poweroff"]).await?;
            }
            "reboot" | "restart" => {
                self.sh("systemctl", &["reboot"]).await?;
            }
            "lock" => {
                self.sh("loginctl", &["lock-session"]).await?;
            }
            "logout" => {
                self.sh("gnome-session-quit", &["--logout", "--no-prompt"])
                    .await?;
            }
            _ => anyhow::bail!("unsupported power action: {}", action),
        }
        Ok(())
    }

    pub(super) async fn battery_status_inner(&self) -> anyhow::Result<Vec<protocol::BatteryInfo>> {
        let reply = self
            .conn
            .call_method(
                Some("org.freedesktop.UPower"),
                "/org/freedesktop/UPower",
                Some("org.freedesktop.UPower"),
                "EnumerateDevices",
                &(),
            )
            .await?;
        let paths: Vec<zbus::zvariant::OwnedObjectPath> = reply.body().deserialize()?;

        let mut batteries = Vec::new();
        for path in &paths {
            let path_str = path.as_str();

            let type_reply = self
                .conn
                .call_method(
                    Some("org.freedesktop.UPower"),
                    path_str,
                    Some("org.freedesktop.DBus.Properties"),
                    "Get",
                    &("org.freedesktop.UPower.Device", "Type"),
                )
                .await;

            if let Ok(reply) = type_reply {
                let type_val: u32 = reply.body().deserialize().unwrap_or(0);
                if type_val != 2 {
                    continue;
                }
            } else {
                continue;
            }

            let pct: f64 = self
                .get_upower_property(path_str, "Percentage")
                .await
                .unwrap_or(0.0);
            let state_val: u32 = self
                .get_upower_property(path_str, "State")
                .await
                .unwrap_or(0);
            let energy_rate: f64 = self
                .get_upower_property(path_str, "EnergyRate")
                .await
                .unwrap_or(0.0);
            let energy: f64 = self
                .get_upower_property(path_str, "Energy")
                .await
                .unwrap_or(0.0);

            let state = match state_val {
                1 => "charging",
                2 => "discharging",
                4 => "fully_charged",
                _ => "unknown",
            };

            let time_remaining = if state == "discharging" && energy_rate > 0.0 {
                Some(((energy / energy_rate) * 60.0) as u32)
            } else if state == "charging" && energy_rate > 0.0 {
                let rem = energy * (100.0 - pct) / 100.0;
                Some(((rem / energy_rate) * 60.0) as u32)
            } else {
                None
            };

            batteries.push(protocol::BatteryInfo {
                source: path_str.to_string(),
                percentage: pct,
                state: state.into(),
                time_remaining_minutes: time_remaining,
            });
        }
        Ok(batteries)
    }
}
