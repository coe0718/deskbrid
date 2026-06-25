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
    async fn list_audio_sinks(&self) -> String {
        self.call(do_execute(&self.state, "audio.list_sinks", json!({})),).await
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
    async fn list_audio_sources(&self) -> String {
        self.call(do_execute(&self.state, "audio.list_sources", json!({})),).await
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
    async fn get_audio_volume(
        &self,
        Parameters(AudioTargetParams { target, id }): Parameters<AudioTargetParams>,
    ) -> String {
        self.call(do_execute(&self.state, "audio.get_volume", json!({"target": target, "id": id})),).await
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
    async fn set_audio_volume(
        &self,
        Parameters(SetVolume { sink_id, volume }): Parameters<SetVolume>,
    ) -> String {
        self.exec("audio.set_sink_volume", json!({"sink_id": sink_id, "volume": volume}),).await
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
    async fn set_audio_node_volume(
        &self,
        Parameters(AudioVolumeParams { target, id, volume }): Parameters<AudioVolumeParams>,
    ) -> String {
        self.exec("audio.set_volume", json!({"target": target, "id": id, "volume": volume}),).await
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
    async fn mute_audio(
        &self,
        Parameters(AudioMuteParams { target, id, mute }): Parameters<AudioMuteParams>,
    ) -> String {
        self.exec("audio.mute", json!({"target": target, "id": id, "mute": mute}),).await
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
    async fn set_default_audio(
        &self,
        Parameters(AudioDefaultParams { target, name }): Parameters<AudioDefaultParams>,
    ) -> String {
        self.exec("audio.set_default", json!({"target": target, "name": name}),).await
    }
    };
}
