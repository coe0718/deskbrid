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

        // ── 1. CreateSession ──
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

        // ── 2. RecordMonitor (before Start — Mutter requires this order) ──
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

        let stream_props: HashMap<&str, zbus::zvariant::Value<'_>> = HashMap::new();
        let mut last_err: Option<anyhow::Error> = None;
        for connector in monitor_candidates {
            tracing::info!("ScreenCast RecordMonitor: {}", connector);
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
                    break;
                }
                Err(e) => {
                    tracing::warn!("RecordMonitor failed for {}: {}", connector, e);
                    last_err = Some(e.into());
                }
            }
        }
        if self.sc_stream_path.is_empty() {
            return Err(last_err.unwrap_or_else(|| anyhow::anyhow!("failed to record any monitor")));
        }

        // ── 3. Listen for PipeWireStreamAdded signal, then Start ──
        // Mutter emits this signal (carrying the uint32 PipeWire node ID)
        // on the stream object after Start completes.
        use zbus::proxy::Proxy;
        let stream_proxy = Proxy::new(
            &self.conn,
            "org.gnome.Mutter.ScreenCast",
            self.sc_stream_path.as_str(),
            "org.gnome.Mutter.ScreenCast.Stream",
        )
        .await?;

        let mut signal_rx = stream_proxy.receive_signal("PipeWireStreamAdded").await?;

        // ── 4. Start (after signal listener is set up) ──
        self.conn
            .call_method(
                Some("org.gnome.Mutter.ScreenCast"),
                session_path.as_str(),
                Some("org.gnome.Mutter.ScreenCast.Session"),
                "Start",
                &(),
            )
            .await?;

        // ── 5. Wait for PipeWireStreamAdded signal (2s timeout) ──
        let pw_node: u32 = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            use futures_util::StreamExt;
            while let Some(msg) = signal_rx.next().await {
                if let Ok(node_id) = msg.body().deserialize::<u32>() {
                    return node_id;
                }
            }
            0
        })
        .await
        .unwrap_or(0);

        if pw_node == 0 {
            tracing::warn!(
                "PipeWireStreamAdded signal not received after Start — screenshots will fall back"
            );
        } else {
            tracing::info!("ScreenCast PipeWire node: {}", pw_node);
        }

        self.sc_pw_node = pw_node;
        self.sc_session_path = session_path.to_string();
        tracing::info!(
            "ScreenCast active: session={}, stream={}, pw_node={}",
            self.sc_session_path,
            self.sc_stream_path,
            pw_node
        );
        Ok(())
    }
}
