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
        fn bluetooth_list(&self) -> Json<Value> {
            block(
                &self.rt,
                do_execute(&self.state, "bluetooth.list", json!({})),
            )
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
        fn bluetooth_scan(
            &self,
            Parameters(BluetoothScan { duration }): Parameters<BluetoothScan>,
        ) -> Json<Value> {
            let mut args = json!({});
            if let Some(d) = duration {
                args["duration"] = json!(d);
            }
            execute(self.state.clone(), &self.rt, "bluetooth.scan", args)
        }
    };
}
