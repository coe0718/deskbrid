#[macro_export]
macro_rules! tools_input {
    () => {

    #[tool(
        name = "type_text",
        description = "Type a string via keyboard input.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn type_text(&self, Parameters(TypeText { text }): Parameters<TypeText>) -> String {
        self.call(do_type_text(&self.state, &text)).await
    }

    #[tool(
        name = "press_key",
        description = "Press a single key (e.g. 'Return', 'Escape', 'Tab').",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn press_key(&self, Parameters(PressKey { key }): Parameters<PressKey>) -> String {
        self.exec("input.keyboard", json!({"key": key}),).await
    }

    #[tool(
        name = "press_keys",
        description = "Press a key combination (e.g. ['Control_L', 'c'] for Ctrl+C).",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn press_keys(&self, Parameters(PressKeys { keys }): Parameters<PressKeys>) -> String {
        self.call(do_press_keys(&self.state, &keys)).await
    }

    #[tool(
        name = "mouse_move",
        description = "Move the mouse cursor to absolute coordinates.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn mouse_move(&self, Parameters(MouseMove { x, y }): Parameters<MouseMove>) -> String {
        self.call(do_mouse_move(&self.state, x, y)).await
    }

    #[tool(
        name = "mouse_click",
        description = "Click a mouse button at the current position.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn mouse_click(
        &self,
        Parameters(MouseClick { button }): Parameters<MouseClick>,
    ) -> String {
        self.call(do_mouse_click(&self.state, &button)).await
    }

    #[tool(
        name = "mouse_scroll",
        description = "Scroll the mouse wheel. Negative dy scrolls down.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn mouse_scroll(
        &self,
        Parameters(MouseScroll { dx, dy }): Parameters<MouseScroll>,
    ) -> String {
        self.exec("input.mouse", json!({"action": "scroll", "dx": dx, "dy": dy}),).await
    }

    #[tool(
        name = "click_coordinate",
        description = "Move to pixel coordinates and click.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn click_coordinate(
        &self,
        Parameters(ClickCoord { x, y, button }): Parameters<ClickCoord>,
    ) -> String {
        self.call(do_click_coordinate(&self.state, x, y, &button)).await
    }

    #[tool(
        name = "drag",
        description = "Click-and-drag between two pixel coordinates.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn drag(
        &self,
        Parameters(Drag {
            from_x,
            from_y,
            to_x,
            to_y,
            button,
        }): Parameters<Drag>,
    ) -> String {
        self.call(do_drag(&self.state, from_x, from_y, to_x, to_y, &button)).await
    }
    };
}
