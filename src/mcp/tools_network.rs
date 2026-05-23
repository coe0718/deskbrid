#[macro_export]
macro_rules! tools_network {
    () => {
        #[tool(
            name = "network_status",
            description = "Network interfaces, IP addresses, and connectivity state.",
            annotations(
                read_only_hint = true,
                destructive_hint = false,
                idempotent_hint = true,
                open_world_hint = true
            )
        )]
        fn network_status(&self) -> Json<Value> {
            block(
                &self.rt,
                do_execute(&self.state, "network.status", json!({})),
            )
        }
    };
}
