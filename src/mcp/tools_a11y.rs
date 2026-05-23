#[macro_export]
macro_rules! tools_a11y {
    () => {
    #[tool(
        name = "list_apps",
        description = "List AT-SPI application roots running on the desktop.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    fn list_apps(&self) -> Json<Value> {
        block(&self.rt, do_list_apps(&self.state))
    }

    #[tool(
        name = "get_accessibility_tree",
        description = "Full AT-SPI tree for an app or window with bounds, roles, states, actions, and text.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    fn get_accessibility_tree(
        &self,
        Parameters(A11yTree {
            app_name,
            pid,
            max_nodes,
            max_depth,
        }): Parameters<A11yTree>,
    ) -> Json<Value> {
        block(
            &self.rt,
            do_get_accessibility_tree(&self.state, app_name.as_deref(), pid, max_nodes, max_depth),
        )
    }


    #[tool(
        name = "perform_action",
        description = "Perform an AT-SPI action on an accessibility element (click, activate, toggle).",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    fn perform_action(
        &self,
        Parameters(A11yAction {
            object_ref,
            action_name,
        }): Parameters<A11yAction>,
    ) -> Json<Value> {
        block(
            &self.rt,
            do_perform_action(&self.state, &object_ref, action_name.as_deref()),
        )
    }

    #[tool(
        name = "set_element_value",
        description = "Set the text value of an AT-SPI editable element.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    fn set_element_value(
        &self,
        Parameters(SetValue { object_ref, value }): Parameters<SetValue>,
    ) -> Json<Value> {
        block(
            &self.rt,
            do_set_element_value(&self.state, &object_ref, &value),
        )
    }

    #[tool(
        name = "get_element_text",
        description = "Get the text content from an AT-SPI element.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    fn get_element_text(
        &self,
        Parameters(GetText {
            object_ref,
            max_chars,
        }): Parameters<GetText>,
    ) -> Json<Value> {
        block(
            &self.rt,
            do_get_element_text(&self.state, &object_ref, max_chars),
        )
    }

    #[tool(
        name = "click_element",
        description = "Click an AT-SPI element using its bounds, falling back to coordinate click.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    fn click_element(
        &self,
        Parameters(ClickElement { object_ref }): Parameters<ClickElement>,
    ) -> Json<Value> {
        block(&self.rt, do_click_element(&self.state, &object_ref))
    }
    };
}
