#[macro_export]
macro_rules! tools_system {
    () => {
    #[tool(
        name = "system_info",
        description = "System information — hostname, OS, kernel, uptime, memory, CPU.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn system_info(&self) -> Json<Value> {
        block(&self.rt, do_execute(&self.state, "system.info", json!({})))
    }

    #[tool(
        name = "battery_status",
        description = "Battery percentage and charging state.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    fn battery_status(&self) -> Json<Value> {
        block(
            &self.rt,
            do_execute(&self.state, "system.battery", json!({})),
        )
    }

    #[tool(
        name = "idle_seconds",
        description = "User idle time in seconds (time since last input).",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    fn idle_seconds(&self) -> Json<Value> {
        block(&self.rt, do_execute(&self.state, "system.idle", json!({})))
    }

    #[tool(
        name = "check_update",
        description = "Check the latest GitHub release without installing it.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    fn check_update(&self) -> Json<Value> {
        block(
            &self.rt,
            do_execute(&self.state, "system.update", json!({"check": true})),
        )
    }

    #[tool(
        name = "self_update",
        description = "Download the latest GitHub release, replace the deskbrid binary, and restart the user service if active. High-risk action: requires explicit system.update permission.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    fn self_update(&self) -> Json<Value> {
        block(
            &self.rt,
            do_execute(&self.state, "system.update", json!({"force": false})),
        )
    }


    #[tool(
        name = "list_processes",
        description = "List running processes with PID, name, CPU, and memory.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    fn list_processes(&self) -> Json<Value> {
        block(&self.rt, do_execute(&self.state, "process.list", json!({})))
    }

    #[tool(
        name = "start_process",
        description = "Start a new background process. Returns the PID.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    fn start_process(
        &self,
        Parameters(ProcessStart { command, workdir }): Parameters<ProcessStart>,
    ) -> Json<Value> {
        let mut args = json!({"command": command});
        if let Some(w) = workdir {
            args["workdir"] = json!(w);
        }
        execute(self.state.clone(), &self.rt, "process.start", args)
    }

    #[tool(
        name = "stop_process",
        description = "Stop a running process by PID.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    fn stop_process(
        &self,
        Parameters(ProcessSignal { pid, signal }): Parameters<ProcessSignal>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "process.stop",
            json!({"pid": pid, "signal": signal}),
        )
    }

    #[tool(
        name = "signal_process",
        description = "Send a signal to a running process.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    fn signal_process(
        &self,
        Parameters(ProcessSignal { pid, signal }): Parameters<ProcessSignal>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "process.signal",
            json!({"pid": pid, "signal": signal}),
        )
    }

    #[tool(
        name = "process_exists",
        description = "Check if a process with the given PID exists.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    fn process_exists(
        &self,
        Parameters(ProcessPid { pid }): Parameters<ProcessPid>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "process.exists",
            json!({"pid": pid}),
        )
    }

    #[tool(
        name = "wait_for_process",
        description = "Wait for a process to exit.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    fn wait_for_process(
        &self,
        Parameters(ProcessWait { pid, timeout_ms }): Parameters<ProcessWait>,
    ) -> Json<Value> {
        let mut args = json!({"pid": pid});
        if let Some(t) = timeout_ms {
            args["timeout_ms"] = json!(t);
        }
        execute(self.state.clone(), &self.rt, "process.wait", args)
    }
    };
}
