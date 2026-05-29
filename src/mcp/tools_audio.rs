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
        name = "list_audio_sources",
        description = "List audio input devices (microphones) with volume and mute state.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    fn list_audio_sources(&self) -> Json<Value> {
        block(
            &self.rt,
            do_execute(&self.state, "audio.list_sources", json!({})),
        )
    }

    #[tool(
        name = "get_audio_volume",
        description = "Get volume level for a sink or source.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    fn get_audio_volume(
        &self,
        Parameters(AudioTargetParams { target, id }): Parameters<AudioTargetParams>,
    ) -> Json<Value> {
        block(
            &self.rt,
            do_execute(&self.state, "audio.get_volume", json!({"target": target, "id": id})),
        )
    }

    #[tool(
        name = "set_audio_volume",
        description = "Set volume for a sink or source. Volume is 0.0-1.0.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    fn set_audio_volume(
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

    #[tool(
        name = "set_audio_node_volume",
        description = "Set volume for any audio node (sink or source) by ID. Volume is 0.0-1.0.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    fn set_audio_node_volume(
        &self,
        Parameters(AudioVolumeParams { target, id, volume }): Parameters<AudioVolumeParams>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "audio.set_volume",
            json!({"target": target, "id": id, "volume": volume}),
        )
    }

    #[tool(
        name = "mute_audio",
        description = "Mute or unmute a sink or source.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    fn mute_audio(
        &self,
        Parameters(AudioMuteParams { target, id, mute }): Parameters<AudioMuteParams>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "audio.mute",
            json!({"target": target, "id": id, "mute": mute}),
        )
    }

    #[tool(
        name = "set_default_audio",
        description = "Set the default sink or source by name.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    fn set_default_audio(
        &self,
        Parameters(AudioDefaultParams { target, name }): Parameters<AudioDefaultParams>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "audio.set_default",
            json!({"target": target, "name": name}),
        )
    }
    };
}
