#[macro_export]
macro_rules! tools_terminal {
    () => {

    #[tool(
        name = "terminal_create",
        description = "Create a PTY terminal. Returns a terminal_id for subsequent operations.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn terminal_create(
        &self,
        Parameters(TerminalCreate {
            shell,
            cwd,
            rows,
            cols,
        }): Parameters<TerminalCreate>,
    ) -> Json<Value> {
        let mut args = json!({});
        if let Some(s) = shell {
            args["shell"] = json!(s);
        }
        if let Some(c) = cwd {
            args["cwd"] = json!(c);
        }
        if let Some(r) = rows {
            args["rows"] = json!(r);
        }
        if let Some(c) = cols {
            args["cols"] = json!(c);
        }
        execute(self.state.clone(), &self.rt, "terminal.create", args)
    }

    #[tool(
        name = "terminal_write",
        description = "Send input to a terminal (supports ANSI escape sequences).",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn terminal_write(
        &self,
        Parameters(TerminalWrite { terminal_id, input }): Parameters<TerminalWrite>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "terminal.write",
            json!({"terminal_id": terminal_id, "input": input}),
        )
    }

    #[tool(
        name = "terminal_read",
        description = "Read output from a terminal.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn terminal_read(
        &self,
        Parameters(TerminalRead {
            terminal_id,
            max_bytes,
            flush,
        }): Parameters<TerminalRead>,
    ) -> Json<Value> {
        let mut args = json!({"terminal_id": terminal_id});
        if let Some(m) = max_bytes {
            args["max_bytes"] = json!(m);
        }
        if flush {
            args["flush"] = json!(true);
        }
        execute(self.state.clone(), &self.rt, "terminal.read", args)
    }

    #[tool(
        name = "terminal_resize",
        description = "Resize a terminal's rows and columns.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn terminal_resize(
        &self,
        Parameters(TerminalResize {
            terminal_id,
            rows,
            cols,
        }): Parameters<TerminalResize>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "terminal.resize",
            json!({"terminal_id": terminal_id, "rows": rows, "cols": cols}),
        )
    }
    };
}
