#[macro_export]
macro_rules! tools_agent {
    () => {
    #[tool(
        name = "send_message",
        description = "Send a message to another agent session's mailbox. Messages persist until retrieved via check_mailbox.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn send_message(
        &self,
        Parameters(args): Parameters<SendMessageArgs>,
    ) -> String {
        let to_session = args.to_session.clone();
        let subject = args.subject.clone();
        let body = args.body.clone();
        let ttl_ms = args.ttl_ms;
        let reply_to = args.reply_to.clone();
        self.call_state( move |state| {
            Box::pin(async move {
                let action = $crate::protocol::Action::AgentMessage {
                    to_session,
                    subject,
                    body: serde_json::to_value(&body).unwrap_or(serde_json::Value::Null),
                    ttl_ms,
                    reply_to,
                };
                $crate::daemon::execute_agent::execute_agent(action, &state, "mcp").await
            })
        }).await
    }

    #[tool(
        name = "broadcast",
        description = "Broadcast a message to all connected agent sessions.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn broadcast(
        &self,
        Parameters(args): Parameters<BroadcastArgs>,
    ) -> String {
        let subject = args.subject.clone();
        let body = args.body.clone();
        let exclude_self = args.exclude_self;
        self.call_state( move |state| {
            Box::pin(async move {
                let action = $crate::protocol::Action::AgentBroadcast {
                    subject,
                    body: serde_json::to_value(&body).unwrap_or(serde_json::Value::Null),
                    exclude_self,
                };
                $crate::daemon::execute_agent::execute_agent(action, &state, "mcp").await
            })
        }).await
    }

    #[tool(
        name = "check_mailbox",
        description = "Check the agent mailbox for received messages.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn check_mailbox(&self) -> String {
        self.call_state( |state| {
            Box::pin(async move {
                let action = $crate::protocol::Action::AgentMailbox;
                $crate::daemon::execute_agent::execute_agent(action, &state, "mcp").await
            })
        }).await
    }
    };
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct SendMessageArgs {
    pub to_session: String,
    pub subject: String,
    pub body: String,
    #[serde(default)]
    pub ttl_ms: Option<u64>,
    #[serde(default)]
    pub reply_to: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct BroadcastArgs {
    pub subject: String,
    pub body: String,
    #[serde(default)]
    pub exclude_self: Option<bool>,
}
