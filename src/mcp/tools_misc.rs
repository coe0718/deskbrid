#[macro_export]
macro_rules! tools_misc {
    () => {

    #[tool(
        name = "doctor",
        description = "Check AT-SPI accessibility readiness. Returns dependency status and fixes needed.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn doctor(&self) -> String {
        self.call(do_doctor(&self.state)).await
    }

    #[tool(
        name = "setup_accessibility",
        description = "Enable AT-SPI accessibility via gsettings. Requires user session.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn setup_accessibility(&self) -> String {
        self.call(do_setup_accessibility(&self.state)).await
    }

    #[tool(
        name = "capabilities",
        description = "List all available Deskbrid capabilities and tool types.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn capabilities(&self) -> String {
        self.call(do_capabilities(&self.state)).await
    }


    #[tool(
        name = "layout_list",
        description = "List saved window layout profiles.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn layout_list(&self) -> String {
        self.call(do_execute(&self.state, "layout_profiles.list", json!({})),).await
    }

    #[tool(
        name = "layout_save",
        description = "Save current window layout as a named profile.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn layout_save(
        &self,
        Parameters(LayoutSave { name, overwrite }): Parameters<LayoutSave>,
    ) -> String {
        self.exec("layout_profiles.save", json!({"name": name, "overwrite": overwrite}),).await
    }

    #[tool(
        name = "layout_restore",
        description = "Restore a saved window layout profile.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn layout_restore(
        &self,
        Parameters(LayoutName { name }): Parameters<LayoutName>,
    ) -> String {
        self.exec("layout_profiles.restore", json!({"name": name}),).await
    }

    #[tool(
        name = "layout_delete",
        description = "Delete a saved layout profile.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn layout_delete(
        &self,
        Parameters(LayoutName { name }): Parameters<LayoutName>,
    ) -> String {
        self.exec("layout_profiles.delete", json!({"name": name}),).await
    }


    #[tool(
        name = "register_hotkey",
        description = "Register a global hotkey combination.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn register_hotkey(
        &self,
        Parameters(HotkeyRegister { hotkey_id, keys }): Parameters<HotkeyRegister>,
    ) -> String {
        self.exec("hotkeys.register", json!({"hotkey_id": hotkey_id, "keys": keys}),).await
    }

    #[tool(
        name = "unregister_hotkey",
        description = "Unregister a previously registered hotkey.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn unregister_hotkey(
        &self,
        Parameters(HotkeyUnregister { hotkey_id }): Parameters<HotkeyUnregister>,
    ) -> String {
        self.exec("hotkeys.unregister", json!({"hotkey_id": hotkey_id}),).await
    }
    };
}
