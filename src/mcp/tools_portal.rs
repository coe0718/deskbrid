// TESTING_NEEDED: This feature requires manual testing on a live desktop environment

#[macro_export]
macro_rules! tools_portal {
    () => {
    #[tool(
        name = "portal_screenshot",
        description = "Take a screenshot via the XDG Desktop Portal (cross-Wayland compatible).",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn portal_screenshot(
        &self,
        Parameters(PortalScreenshotParams { interactive }): Parameters<PortalScreenshotParams>,
    ) -> String {
        self.exec("portal.screenshot", json!({"interactive": interactive}),).await
    }

    #[tool(
        name = "portal_screencast_start",
        description = "Start a screencast session via XDG Desktop Portal (requires PipeWire).",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn portal_screencast_start(
        &self,
        Parameters(ScreencastStartParams { output_path }): Parameters<ScreencastStartParams>,
    ) -> String {
        self.exec("portal.screencast_start", json!({"output_path": output_path}),).await
    }

    #[tool(
        name = "portal_screencast_stop",
        description = "Stop the running portal screencast session.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn portal_screencast_stop(&self) -> String {
        self.exec("portal.screencast_stop", json!({}),).await
    }
    };
}
