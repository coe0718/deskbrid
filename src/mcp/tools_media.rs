#[macro_export]
macro_rules! tools_media {
    () => {

    #[tool(
        name = "list_media_players",
        description = "List MPRIS media players on the D-Bus session bus.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn list_media_players(&self) -> String {
        self.call(do_execute(&self.state, "mpris.list", json!({}))).await
    }

    #[tool(
        name = "media_player_info",
        description = "Get detailed info about an MPRIS media player (track, artist, album, position, playback status).",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn media_player_info(
        &self,
        Parameters(MprisPlayer { player }): Parameters<MprisPlayer>,
    ) -> String {
        let mut args = json!({});
        if let Some(p) = player {
            args["player"] = json!(p);
        }
        self.exec("mpris.get", args).await
    }

    #[tool(
        name = "media_player_control",
        description = "Control an MPRIS media player (play, pause, next, previous, stop).",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn media_player_control(
        &self,
        Parameters(MprisControl { player, action }): Parameters<MprisControl>,
    ) -> String {
        let mut args = json!({"action": action});
        if let Some(p) = player {
            args["player"] = json!(p);
        }
        self.exec("mpris.control", args).await
    }
    };
}
