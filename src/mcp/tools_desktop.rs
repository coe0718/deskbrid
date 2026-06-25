#[macro_export]
macro_rules! tools_desktop {
    () => {
    #[tool(
        name = "list_schemas",
        description = "List all available gsettings schemas on the system.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn list_schemas(&self) -> String {
        self.call(do_execute(&self.state, "desktop.list_schemas", json!({}))).await
    }

    #[tool(
        name = "get_setting",
        description = "Read a desktop setting value by schema and key (e.g. 'org.gnome.desktop.interface', 'gtk-theme').",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_setting(
        &self,
        Parameters(DesktopSettingKey { schema, key }): Parameters<DesktopSettingKey>,
    ) -> String {
        self.exec("desktop.get_setting", json!({"schema": schema, "key": key}),).await
    }

    #[tool(
        name = "set_setting",
        description = "Write a desktop setting value by schema, key, and value.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn set_setting(
        &self,
        Parameters(DesktopSettingValue { schema, key, value }): Parameters<DesktopSettingValue>,
    ) -> String {
        self.exec("desktop.set_setting", json!({"schema": schema, "key": key, "value": value}),).await
    }
    };
}
