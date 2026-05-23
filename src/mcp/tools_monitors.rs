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
    fn list_monitors(&self) -> Json<Value> {
        block(&self.rt, do_execute(&self.state, "monitor.list", json!({})))
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
    fn set_primary_monitor(
        &self,
        Parameters(MonitorOutput { output }): Parameters<MonitorOutput>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "monitor.set_primary",
            json!({"output": output}),
        )
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
    fn set_monitor_resolution(
        &self,
        Parameters(SetResolution {
            output,
            width,
            height,
            refresh_rate,
        }): Parameters<SetResolution>,
    ) -> Json<Value> {
        let mut args = json!({"output": output, "width": width, "height": height});
        if let Some(r) = refresh_rate {
            args["refresh_rate"] = json!(r);
        }
        execute(self.state.clone(), &self.rt, "monitor.set_resolution", args)
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
    fn set_monitor_scale(
        &self,
        Parameters(SetScale { output, scale }): Parameters<SetScale>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "monitor.set_scale",
            json!({"output": output, "scale": scale}),
        )
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
    fn set_monitor_rotation(
        &self,
        Parameters(SetRotation { output, rotation }): Parameters<SetRotation>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "monitor.set_rotation",
            json!({"output": output, "rotation": rotation}),
        )
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
    fn enable_monitor(
        &self,
        Parameters(MonitorOutput { output }): Parameters<MonitorOutput>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "monitor.enable",
            json!({"output": output}),
        )
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
    fn disable_monitor(
        &self,
        Parameters(MonitorOutput { output }): Parameters<MonitorOutput>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "monitor.disable",
            json!({"output": output}),
        )
    }
    };
}
