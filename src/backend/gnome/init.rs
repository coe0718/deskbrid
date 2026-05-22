use super::*;

impl GnomeBackend {
    pub(super) async fn init_remote_desktop(&mut self) -> anyhow::Result<()> {
        let reply = self
            .conn
            .call_method(
                Some("org.gnome.Mutter.RemoteDesktop"),
                "/org/gnome/Mutter/RemoteDesktop",
                Some("org.gnome.Mutter.RemoteDesktop"),
                "CreateSession",
                &(),
            )
            .await?;
        let path: zbus::zvariant::OwnedObjectPath = reply.body().deserialize()?;
        self.conn
            .call_method(
                Some("org.gnome.Mutter.RemoteDesktop"),
                path.as_str(),
                Some("org.gnome.Mutter.RemoteDesktop.Session"),
                "Start",
                &(),
            )
            .await?;
        self.rd_session_path = path.to_string();
        tracing::info!("RemoteDesktop session started: {}", self.rd_session_path);
        Ok(())
    }

    pub(super) async fn init_screen_cast(&mut self) -> anyhow::Result<()> {
        use std::collections::HashMap;
        let props: HashMap<&str, zbus::zvariant::Value<'_>> = HashMap::new();
        let reply = self
            .conn
            .call_method(
                Some("org.gnome.Mutter.ScreenCast"),
                "/org/gnome/Mutter/ScreenCast",
                Some("org.gnome.Mutter.ScreenCast"),
                "CreateSession",
                &(props,),
            )
            .await?;
        let session_path: zbus::zvariant::OwnedObjectPath = reply.body().deserialize()?;
        self.conn
            .call_method(
                Some("org.gnome.Mutter.ScreenCast"),
                session_path.as_str(),
                Some("org.gnome.Mutter.ScreenCast.Session"),
                "Start",
                &(),
            )
            .await?;

        let mut monitor_candidates = Vec::new();
        if let Ok(monitors) = self.get_monitors().await {
            if let Some(primary) = monitors
                .iter()
                .find(|m| m.primary)
                .or_else(|| monitors.first())
            {
                monitor_candidates.push(primary.name.clone());
            }
            for m in monitors {
                if !monitor_candidates.iter().any(|n| n == &m.name) {
                    monitor_candidates.push(m.name);
                }
            }
        }
        if !monitor_candidates.iter().any(|n| n == "DP-1") {
            monitor_candidates.push("DP-1".to_string());
        }

        let stream_props: HashMap<&str, zbus::zvariant::Value<'_>> = HashMap::new();
        let mut last_err: Option<anyhow::Error> = None;
        for connector in monitor_candidates {
            tracing::info!("Trying ScreenCast monitor: {}", connector);
            match self
                .conn
                .call_method(
                    Some("org.gnome.Mutter.ScreenCast"),
                    session_path.as_str(),
                    Some("org.gnome.Mutter.ScreenCast.Session"),
                    "RecordMonitor",
                    &(connector.as_str(), stream_props.clone()),
                )
                .await
            {
                Ok(reply) => {
                    let sp: zbus::zvariant::OwnedObjectPath = reply.body().deserialize()?;
                    self.sc_stream_path = sp.to_string();
                    tracing::info!("ScreenCast stream created: {}", self.sc_stream_path);
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!("RecordMonitor failed for {}: {}", connector, e);
                    last_err = Some(e.into());
                }
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("failed to record any monitor")))
    }
}
