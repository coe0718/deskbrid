#[macro_export]
macro_rules! tools_confirmation {
    () => {
    #[tool(
        name = "confirm_action",
        description = "Confirm a pending destructive action. Requires the confirmation ID from the pending list.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn confirm_action(
        &self,
        Parameters(ConfirmActionArgs { id }): Parameters<ConfirmActionArgs>,
    ) -> Json<Value> {
        let id = id.clone();
        block_state(&self.rt, &self.state, move |state| {
            Box::pin(async move {
                let action = $crate::protocol::Action::ConfirmAction { id };
                $crate::daemon::execute_confirmation::execute_confirmation(action, &state).await
            })
        })
    }

    #[tool(
        name = "deny_action",
        description = "Deny/reject a pending destructive action. Requires the confirmation ID.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn deny_action(
        &self,
        Parameters(ConfirmActionArgs { id }): Parameters<ConfirmActionArgs>,
    ) -> Json<Value> {
        let id = id.clone();
        block_state(&self.rt, &self.state, move |state| {
            Box::pin(async move {
                let action = $crate::protocol::Action::DenyAction { id };
                $crate::daemon::execute_confirmation::execute_confirmation(action, &state).await
            })
        })
    }

    #[tool(
        name = "list_confirmations",
        description = "List all pending action confirmations waiting for approval.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn list_confirmations(&self) -> Json<Value> {
        block_state(&self.rt, &self.state, |state| {
            Box::pin(async move {
                let action = $crate::protocol::Action::ConfirmationList;
                $crate::daemon::execute_confirmation::execute_confirmation(action, &state).await
            })
        })
    }
    };
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct ConfirmActionArgs {
    pub id: String,
}
