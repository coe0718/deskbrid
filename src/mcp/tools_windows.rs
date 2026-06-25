#[macro_export]
macro_rules! tools_windows {
    () => {
    #[tool(
        name = "list_windows",
        description = "List all open windows with IDs, titles, classes, workspace, and geometry.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn list_windows(&self) -> String {
        self.call(do_execute(&self.state, "windows.list", json!({}))).await
    }

    #[tool(
        name = "focused_window",
        description = "Get the currently focused/active window.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn focused_window(&self) -> String {
        self.call(do_focused_window(&self.state)).await
    }

    #[tool(
        name = "list_workspaces",
        description = "List all virtual desktops/workspaces with current state.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn list_workspaces(&self) -> String {
        self.call(do_execute(&self.state, "workspaces.list", json!({})),).await
    }


    #[tool(
        name = "focus_window",
        description = "Focus (activate) a window by its ID.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn focus_window(
        &self,
        Parameters(WindowId { window_id }): Parameters<WindowId>,
    ) -> String {
        self.call(do_focus_window(&self.state, &window_id)).await
    }

    #[tool(
        name = "close_window",
        description = "Close a window by its ID.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn close_window(
        &self,
        Parameters(WindowId { window_id }): Parameters<WindowId>,
    ) -> String {
        self.exec("windows.close", json!({"window_id": window_id}),).await
    }

    #[tool(
        name = "minimize_window",
        description = "Minimize a window by its ID.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn minimize_window(
        &self,
        Parameters(WindowId { window_id }): Parameters<WindowId>,
    ) -> String {
        self.exec("windows.minimize", json!({"window_id": window_id}),).await
    }

    #[tool(
        name = "maximize_window",
        description = "Maximize a window by its ID.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn maximize_window(
        &self,
        Parameters(WindowId { window_id }): Parameters<WindowId>,
    ) -> String {
        self.exec("windows.maximize", json!({"window_id": window_id}),).await
    }

    #[tool(
        name = "move_resize_window",
        description = "Move and/or resize a window.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn move_resize_window(
        &self,
        Parameters(MoveResize {
            window_id,
            x,
            y,
            width,
            height,
        }): Parameters<MoveResize>,
    ) -> String {
        self.exec("windows.move_resize", json!({"window_id": window_id, "x": x, "y": y, "width": width, "height": height}),).await
    }

    #[tool(
        name = "tile_window",
        description = "Tile a window to a preset position (left, right, maximize, fullscreen).",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn tile_window(
        &self,
        Parameters(TileWindow {
            window_id,
            preset,
            monitor,
            padding,
        }): Parameters<TileWindow>,
    ) -> String {
        let mut args = json!({"window_id": window_id, "preset": preset});
        if let Some(m) = monitor {
            args["monitor"] = json!(m);
        }
        if let Some(p) = padding {
            args["padding"] = json!(p);
        }
        self.exec("windows.tile", args).await
    }

    #[tool(
        name = "activate_or_launch",
        description = "Focus an existing app window or launch it if not running.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn activate_or_launch(
        &self,
        Parameters(ActivateOrLaunch {
            app_id,
            command,
            workdir,
        }): Parameters<ActivateOrLaunch>,
    ) -> String {
        let mut args = json!({"app_id": app_id, "command": command});
        if let Some(wd) = workdir {
            args["workdir"] = json!(wd);
        }
        self.exec("windows.activate_or_launch", args,).await
    }


    #[tool(
        name = "switch_workspace",
        description = "Switch to a specific workspace by index.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn switch_workspace(
        &self,
        Parameters(SwitchWorkspace { workspace_id }): Parameters<SwitchWorkspace>,
    ) -> String {
        self.exec("workspaces.switch", json!({"workspace_id": workspace_id}),).await
    }

    #[tool(
        name = "move_window_to_workspace",
        description = "Move a window to another workspace, optionally following it.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn move_window_to_workspace(
        &self,
        Parameters(MoveWindowToWorkspace {
            window_id,
            workspace_id,
            follow,
        }): Parameters<MoveWindowToWorkspace>,
    ) -> String {
        self.exec("workspaces.move_window", json!({"window_id": window_id, "workspace_id": workspace_id, "follow": follow}),).await
    }
    };
}
