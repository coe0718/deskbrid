#[macro_export]
macro_rules! tools_bluetooth {
    () => {
        #[tool(
            name = "bluetooth_list",
            description = "List paired Bluetooth devices.",
            annotations(
                read_only_hint = true,
                destructive_hint = false,
                idempotent_hint = true,
                open_world_hint = true
            )
        )]
        async fn bluetooth_list(&self) -> String {
            self.call(do_execute(&self.state, "bluetooth.list", json!({})))
                .await
        }

        #[tool(
            name = "bluetooth_scan",
            description = "Scan for nearby Bluetooth devices.",
            annotations(
                read_only_hint = true,
                destructive_hint = false,
                idempotent_hint = false,
                open_world_hint = true
            )
        )]
        async fn bluetooth_scan(
            &self,
            Parameters(BluetoothScan { duration }): Parameters<BluetoothScan>,
        ) -> String {
            let mut args = json!({});
            if let Some(d) = duration {
                args["duration"] = json!(d);
            }
            self.exec("bluetooth.scan", args).await
        }
    };
}
