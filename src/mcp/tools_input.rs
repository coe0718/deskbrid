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
    fn type_text(&self, Parameters(TypeText { text }): Parameters<TypeText>) -> Json<Value> {
        block(&self.rt, do_type_text(&self.state, &text))
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
    fn press_key(&self, Parameters(PressKey { key }): Parameters<PressKey>) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "input.keyboard",
            json!({"key": key}),
        )
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
    fn press_keys(&self, Parameters(PressKeys { keys }): Parameters<PressKeys>) -> Json<Value> {
        block(&self.rt, do_press_keys(&self.state, &keys))
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
    fn mouse_move(&self, Parameters(MouseMove { x, y }): Parameters<MouseMove>) -> Json<Value> {
        block(&self.rt, do_mouse_move(&self.state, x, y))
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
    fn mouse_click(
        &self,
        Parameters(MouseClick { button }): Parameters<MouseClick>,
    ) -> Json<Value> {
        block(&self.rt, do_mouse_click(&self.state, &button))
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
    fn mouse_scroll(
        &self,
        Parameters(MouseScroll { dx, dy }): Parameters<MouseScroll>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "input.mouse",
            json!({"action": "scroll", "dx": dx, "dy": dy}),
        )
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
    fn click_coordinate(
        &self,
        Parameters(ClickCoord { x, y, button }): Parameters<ClickCoord>,
    ) -> Json<Value> {
        block(&self.rt, do_click_coordinate(x, y, &button))
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
    fn drag(
        &self,
        Parameters(Drag {
            from_x,
            from_y,
            to_x,
            to_y,
            button,
        }): Parameters<Drag>,
    ) -> Json<Value> {
        block(&self.rt, do_drag(from_x, from_y, to_x, to_y, &button))
    }
    };
}
