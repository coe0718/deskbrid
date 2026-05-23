#[macro_export]
macro_rules! tools_audio {
    () => {

    #[tool(
        name = "list_audio_sinks",
        description = "List audio output devices with volume and mute state.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    fn list_audio_sinks(&self) -> Json<Value> {
        block(
            &self.rt,
            do_execute(&self.state, "audio.list_sinks", json!({})),
        )
    }

    #[tool(
        name = "set_volume",
        description = "Set audio sink volume.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    fn set_volume(
        &self,
        Parameters(SetVolume { sink_id, volume }): Parameters<SetVolume>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "audio.set_sink_volume",
            json!({"sink_id": sink_id, "volume": volume}),
        )
    }
    };
}
