#[macro_export]
macro_rules! tools_browser {
    () => {

    #[tool(
        name = "list_browser_tabs",
        description = "List open browser tabs via Chrome DevTools Protocol. Requires Chrome/Chromium with remote debugging enabled.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn list_browser_tabs(&self) -> String {
        self.call(do_execute(&self.state, "browser.list_tabs", json!({})),).await
    }

    #[tool(
        name = "browser_navigate",
        description = "Navigate a browser tab to a URL.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn browser_navigate(
        &self,
        Parameters(BrowserNavigate { tab_index, url }): Parameters<BrowserNavigate>,
    ) -> String {
        let mut args = json!({"url": url});
        if let Some(t) = tab_index {
            args["tab_index"] = json!(t);
        }
        self.exec("browser.navigate", args).await
    }

    #[tool(
        name = "browser_evaluate",
        description = "Evaluate JavaScript in a browser tab and return the result.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn browser_evaluate(
        &self,
        Parameters(BrowserEvaluate {
            tab_index,
            expression,
            await_promise,
        }): Parameters<BrowserEvaluate>,
    ) -> String {
        let mut args = json!({"expression": expression, "await_promise": await_promise});
        if let Some(t) = tab_index {
            args["tab_index"] = json!(t);
        }
        self.exec("browser.evaluate", args).await
    }

    #[tool(
        name = "browser_screenshot",
        description = "Take a screenshot of a browser tab.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn browser_screenshot(
        &self,
        Parameters(TabIndex { tab_index }): Parameters<TabIndex>,
    ) -> String {
        let mut args = json!({});
        if let Some(t) = tab_index {
            args["tab_index"] = json!(t);
        }
        self.exec("browser.screenshot_tab", args).await
    }

    #[tool(
        name = "browser_click",
        description = "Click an element in a browser tab by CSS selector.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn browser_click(
        &self,
        Parameters(BrowserClick {
            tab_index,
            selector,
        }): Parameters<BrowserClick>,
    ) -> String {
        let mut args = json!({"selector": selector});
        if let Some(t) = tab_index {
            args["tab_index"] = json!(t);
        }
        self.exec("browser.click", args).await
    }
    };
}
