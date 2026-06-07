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
        name = "dbus_call",
        description = "Raw D-Bus method call. Escape hatch for direct D-Bus access. Requires explicit dbus.call permission.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    fn dbus_call(
        &self,
        Parameters(DbusCallArgs {
            bus,
            service,
            path,
            interface,
            method,
            args,
        }): Parameters<DbusCallArgs>,
    ) -> Json<Value> {
        let mut req = json!({
            "service": service,
            "path": path,
            "interface": interface,
            "method": method,
        });
        if let Some(b) = bus {
            req["bus"] = json!(b);
        }
        if let Some(a) = args {
            req["args"] = a;
        }
        execute(self.state.clone(), &self.rt, "dbus.call", req)
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

    #[tool(
        name = "backlight_list",
        description = "List all backlight devices with max and current brightness.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    fn backlight_list(&self) -> Json<Value> {
        block(&self.rt, do_execute(&self.state, "system.backlight_list", json!({})))
    }

    #[tool(
        name = "backlight_get",
        description = "Get current brightness of a backlight device (or default).",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    fn backlight_get(
        &self,
        Parameters(BacklightDevice { device }): Parameters<BacklightDevice>,
    ) -> Json<Value> {
        let mut args = json!({});
        if let Some(d) = device {
            args["device"] = json!(d);
        }
        execute(self.state.clone(), &self.rt, "system.backlight_get", args)
    }

    #[tool(
        name = "backlight_set",
        description = "Set backlight brightness by percentage ('50%') or raw value ('469').",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    fn backlight_set(
        &self,
        Parameters(BacklightSetArgs { device, value }): Parameters<BacklightSetArgs>,
    ) -> Json<Value> {
        let mut args = json!({"value": value});
        if let Some(d) = device {
            args["device"] = json!(d);
        }
        execute(self.state.clone(), &self.rt, "system.backlight_set", args)
    }

    #[tool(
        name = "print_list",
        description = "List all configured printers with name, status, and default.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    fn print_list(&self) -> Json<Value> {
        block(&self.rt, do_execute(&self.state, "system.print_list", json!({})))
    }

    #[tool(
        name = "print_default",
        description = "Get or set the default printer. Omit printer to read; provide printer to set.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    fn print_default(
        &self,
        Parameters(PrintDefaultArgs { printer }): Parameters<PrintDefaultArgs>,
    ) -> Json<Value> {
        let mut args = json!({});
        if let Some(p) = printer {
            args["printer"] = json!(p);
        }
        execute(self.state.clone(), &self.rt, "system.print_default", args)
    }

    #[tool(
        name = "print_file",
        description = "Send a file to a printer. printer: printer name, path: absolute path to file.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    fn print_file(
        &self,
        Parameters(PrintFileArgs { printer, path }): Parameters<PrintFileArgs>,
    ) -> Json<Value> {
        let args = json!({"printer": printer, "path": path});
        execute(self.state.clone(), &self.rt, "system.print_file", args)
    }

    #[tool(
        name = "print_jobs",
        description = "List active print jobs in the queue.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    fn print_jobs(&self) -> Json<Value> {
        block(&self.rt, do_execute(&self.state, "system.print_jobs", json!({})))
    }

    #[tool(
        name = "print_job_cancel",
        description = "Cancel a print job by ID.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    fn print_job_cancel(
        &self,
        Parameters(PrintJobAction { job_id }): Parameters<PrintJobAction>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "system.print_job_cancel",
            json!({"job_id": job_id}),
        )
    }

    #[tool(
        name = "print_job_pause",
        description = "Pause a print job by ID.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    fn print_job_pause(
        &self,
        Parameters(PrintJobAction { job_id }): Parameters<PrintJobAction>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "system.print_job_pause",
            json!({"job_id": job_id}),
        )
    }

    #[tool(
        name = "print_job_resume",
        description = "Resume a paused print job by ID.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    fn print_job_resume(
        &self,
        Parameters(PrintJobAction { job_id }): Parameters<PrintJobAction>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "system.print_job_resume",
            json!({"job_id": job_id}),
        )
    }

    #[tool(
        name = "pressure",
        description = "Read Linux Pressure Stall Information (PSI) — CPU, memory, and IO pressure stats. Agents use this to decide whether to proceed, back off, or retry.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    fn pressure(&self) -> Json<Value> {
        block(&self.rt, do_execute(&self.state, "system.pressure", json!({})))
    }
    };
}
