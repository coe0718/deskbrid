#[macro_export]
macro_rules! tools_screenshot {
    () => {
    #[tool(
        name = "screenshot",
        description = "Take a screenshot. Returns base64-encoded PNG.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn screenshot(&self) -> String {
        self.call(do_execute(&self.state, "screenshot", json!({}))).await
    }

    #[tool(
        name = "screenshot_region",
        description = "Capture a region of the screen or a specific window.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn screenshot_region(
        &self,
        Parameters(ScreenshotOptions {
            monitor,
            window_id,
            region_x,
            region_y,
            region_w,
            region_h,
        }): Parameters<ScreenshotOptions>,
    ) -> String {
        let mut args = json!({});
        if let Some(m) = monitor {
            args["monitor"] = json!(m);
        }
        if let Some(w) = window_id {
            args["window_id"] = json!(w);
        }
        if let (Some(x), Some(y), Some(w), Some(h)) = (region_x, region_y, region_w, region_h) {
            args["region"] = json!({"x": x, "y": y, "width": w, "height": h});
        }
        self.exec("screenshot", args).await
    }

    #[tool(
        name = "screenshot_diff",
        description = "Pixel diff between two screenshots. Useful for detecting UI changes.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn screenshot_diff(
        &self,
        Parameters(ScreenshotDiff {
            before_path,
            after_path,
            tolerance,
            diff_path,
            monitor,
        }): Parameters<ScreenshotDiff>,
    ) -> String {
        let mut args = json!({"before_path": before_path});
        if let Some(a) = after_path {
            args["after_path"] = json!(a);
        }
        if let Some(t) = tolerance {
            args["tolerance"] = json!(t);
        }
        if let Some(d) = diff_path {
            args["diff_path"] = json!(d);
            args["save_diff"] = json!(true);
        }
        if let Some(m) = monitor {
            args["monitor"] = json!(m);
        }
        self.exec("screenshot.diff", args).await
    }
    };
}
