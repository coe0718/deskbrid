#[macro_export]
macro_rules! tools_services {
    () => {
    #[tool(
        name = "service_status",
        description = "Check a systemd service's status.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn service_status(
        &self,
        Parameters(ServiceName { name }): Parameters<ServiceName>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "service.status",
            json!({"name": name}),
        )
    }

    #[tool(
        name = "service_start",
        description = "Start a systemd service.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn service_start(
        &self,
        Parameters(ServiceName { name }): Parameters<ServiceName>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "service.start",
            json!({"name": name}),
        )
    }

    #[tool(
        name = "service_stop",
        description = "Stop a systemd service.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn service_stop(
        &self,
        Parameters(ServiceName { name }): Parameters<ServiceName>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "service.stop",
            json!({"name": name}),
        )
    }

    #[tool(
        name = "journal_query",
        description = "Query the systemd journal.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    fn journal_query(
        &self,
        Parameters(JournalQuery {
            since,
            until,
            unit,
            priority,
            tail,
        }): Parameters<JournalQuery>,
    ) -> Json<Value> {
        let mut args = json!({});
        if let Some(s) = since {
            args["since"] = json!(s);
        }
        if let Some(u) = until {
            args["until"] = json!(u);
        }
        if let Some(n) = unit {
            args["unit"] = json!(n);
        }
        if let Some(p) = priority {
            args["priority"] = json!(p);
        }
        if let Some(t) = tail {
            args["tail"] = json!(t);
        }
        execute(self.state.clone(), &self.rt, "journal.query", args)
    }
    };
}
