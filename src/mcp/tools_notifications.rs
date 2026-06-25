#[macro_export]
macro_rules! tools_notifications {
    () => {

    #[tool(
        name = "send_notification",
        description = "Send a desktop notification via D-Bus.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn send_notification(
        &self,
        Parameters(NotificationSend {
            app_name,
            title,
            body,
            urgency,
        }): Parameters<NotificationSend>,
    ) -> String {
        self.exec("notification.send", json!({"app_name": app_name, "title": title, "body": body, "urgency": urgency}),).await
    }

    #[tool(
        name = "close_notification",
        description = "Close a desktop notification by ID.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn close_notification(
        &self,
        Parameters(NotificationClose { notification_id }): Parameters<NotificationClose>,
    ) -> String {
        self.exec("notification.close", json!({"notification_id": notification_id}),).await
    }
    };
}
