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
    async fn terminal_create(
        &self,
        Parameters(TerminalCreate {
            shell,
            cwd,
            rows,
            cols,
        }): Parameters<TerminalCreate>,
    ) -> String {
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
        self.exec("terminal.create", args).await
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
    async fn terminal_write(
        &self,
        Parameters(TerminalWrite { terminal_id, input }): Parameters<TerminalWrite>,
    ) -> String {
        self.exec("terminal.write", json!({"terminal_id": terminal_id, "input": input}),).await
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
    async fn terminal_read(
        &self,
        Parameters(TerminalRead {
            terminal_id,
            max_bytes,
            flush,
        }): Parameters<TerminalRead>,
    ) -> String {
        let mut args = json!({"terminal_id": terminal_id});
        if let Some(m) = max_bytes {
            args["max_bytes"] = json!(m);
        }
        if flush {
            args["flush"] = json!(true);
        }
        self.exec("terminal.read", args).await
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
    async fn terminal_resize(
        &self,
        Parameters(TerminalResize {
            terminal_id,
            rows,
            cols,
        }): Parameters<TerminalResize>,
    ) -> String {
        self.exec("terminal.resize", json!({"terminal_id": terminal_id, "rows": rows, "cols": cols}),).await
    }
    };
}
