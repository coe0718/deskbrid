#[macro_export]
macro_rules! tools_monitors {
    () => {

    #[tool(
        name = "list_monitors",
        description = "List all connected monitors/displays with resolution, position, scale, and refresh rate.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn list_monitors(&self) -> String {
        self.call(do_execute(&self.state, "monitor.list", json!({}))).await
    }

    #[tool(
        name = "set_primary_monitor",
        description = "Set a monitor as the primary display.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn set_primary_monitor(
        &self,
        Parameters(MonitorOutput { output }): Parameters<MonitorOutput>,
    ) -> String {
        self.exec("monitor.set_primary", json!({"output": output}),).await
    }

    #[tool(
        name = "set_monitor_resolution",
        description = "Change a monitor's resolution and optionally refresh rate.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn set_monitor_resolution(
        &self,
        Parameters(SetResolution {
            output,
            width,
            height,
            refresh_rate,
        }): Parameters<SetResolution>,
    ) -> String {
        let mut args = json!({"output": output, "width": width, "height": height});
        if let Some(r) = refresh_rate {
            args["refresh_rate"] = json!(r);
        }
        self.exec("monitor.set_resolution", args).await
    }

    #[tool(
        name = "set_monitor_scale",
        description = "Set a monitor's display scale factor.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn set_monitor_scale(
        &self,
        Parameters(SetScale { output, scale }): Parameters<SetScale>,
    ) -> String {
        self.exec("monitor.set_scale", json!({"output": output, "scale": scale}),).await
    }

    #[tool(
        name = "set_monitor_rotation",
        description = "Rotate a monitor's display output.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn set_monitor_rotation(
        &self,
        Parameters(SetRotation { output, rotation }): Parameters<SetRotation>,
    ) -> String {
        self.exec("monitor.set_rotation", json!({"output": output, "rotation": rotation}),).await
    }

    #[tool(
        name = "enable_monitor",
        description = "Enable a previously disabled monitor.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn enable_monitor(
        &self,
        Parameters(MonitorOutput { output }): Parameters<MonitorOutput>,
    ) -> String {
        self.exec("monitor.enable", json!({"output": output}),).await
    }

    #[tool(
        name = "disable_monitor",
        description = "Disable a monitor output.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn disable_monitor(
        &self,
        Parameters(MonitorOutput { output }): Parameters<MonitorOutput>,
    ) -> String {
        self.exec("monitor.disable", json!({"output": output}),).await
    }
    };
}
