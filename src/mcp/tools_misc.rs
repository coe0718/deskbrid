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
    fn doctor(&self) -> Json<Value> {
        block(&self.rt, do_doctor(&self.state))
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
    fn setup_accessibility(&self) -> Json<Value> {
        block(&self.rt, do_setup_accessibility(&self.state))
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
    fn capabilities(&self) -> Json<Value> {
        block(&self.rt, do_capabilities(&self.state))
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
    fn layout_list(&self) -> Json<Value> {
        block(
            &self.rt,
            do_execute(&self.state, "layout_profiles.list", json!({})),
        )
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
    fn layout_save(
        &self,
        Parameters(LayoutSave { name, overwrite }): Parameters<LayoutSave>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "layout_profiles.save",
            json!({"name": name, "overwrite": overwrite}),
        )
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
    fn layout_restore(
        &self,
        Parameters(LayoutName { name }): Parameters<LayoutName>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "layout_profiles.restore",
            json!({"name": name}),
        )
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
    fn layout_delete(
        &self,
        Parameters(LayoutName { name }): Parameters<LayoutName>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "layout_profiles.delete",
            json!({"name": name}),
        )
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
    fn register_hotkey(
        &self,
        Parameters(HotkeyRegister { hotkey_id, keys }): Parameters<HotkeyRegister>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "hotkeys.register",
            json!({"hotkey_id": hotkey_id, "keys": keys}),
        )
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
    fn unregister_hotkey(
        &self,
        Parameters(HotkeyUnregister { hotkey_id }): Parameters<HotkeyUnregister>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "hotkeys.unregister",
            json!({"hotkey_id": hotkey_id}),
        )
    }
    };
}
