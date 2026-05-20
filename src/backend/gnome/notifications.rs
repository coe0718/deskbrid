use super::GnomeBackend;
use zbus::zvariant;

impl GnomeBackend {
    pub(super) async fn notification_send_inner(
        &self,
        app_name: &str,
        title: &str,
        body: &str,
        urgency: &str,
    ) -> anyhow::Result<u32> {
        let urgency_byte = match urgency {
            "low" => 0u8,
            "normal" => 1u8,
            "critical" => 2u8,
            _ => 1u8,
        };

        let reply = self
            .conn
            .call_method(
                Some("org.freedesktop.Notifications"),
                "/org/freedesktop/Notifications",
                Some("org.freedesktop.Notifications"),
                "Notify",
                &(
                    app_name,
                    0u32,
                    "",
                    title,
                    body,
                    &[] as &[&str],
                    &[("urgency", zvariant::Value::U8(urgency_byte))]
                        as &[(&str, zvariant::Value)],
                    5000i32,
                ),
            )
            .await?;
        let id: u32 = reply.body().deserialize()?;
        Ok(id)
    }

    pub(super) async fn notification_close_inner(&self, id: u32) -> anyhow::Result<()> {
        self.conn
            .call_method(
                Some("org.freedesktop.Notifications"),
                "/org/freedesktop/Notifications",
                Some("org.freedesktop.Notifications"),
                "CloseNotification",
                &(id,),
            )
            .await?;
        Ok(())
    }
}
