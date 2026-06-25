#[macro_export]
macro_rules! tools_clipboard {
    () => {
        #[tool(
            name = "clipboard_read",
            description = "Read the current clipboard contents.",
            annotations(
                read_only_hint = true,
                destructive_hint = false,
                idempotent_hint = true,
                open_world_hint = true
            )
        )]
        async fn clipboard_read(&self) -> String {
            self.call(do_execute(&self.state, "clipboard.read", json!({})),).await
        }

        #[tool(
            name = "clipboard_write",
            description = "Write text to the system clipboard.",
            annotations(
                read_only_hint = false,
                destructive_hint = true,
                idempotent_hint = false,
                open_world_hint = true
            )
        )]
        async fn clipboard_write(
            &self,
            Parameters(ClipboardWrite { text }): Parameters<ClipboardWrite>,
        ) -> String {
            self.call(do_clipboard_write(&self.state, &text)).await
        }
    };
}
