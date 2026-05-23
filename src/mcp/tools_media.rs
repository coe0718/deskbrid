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
    fn list_media_players(&self) -> Json<Value> {
        block(&self.rt, do_execute(&self.state, "mpris.list", json!({})))
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
    fn media_player_info(
        &self,
        Parameters(MprisPlayer { player }): Parameters<MprisPlayer>,
    ) -> Json<Value> {
        let mut args = json!({});
        if let Some(p) = player {
            args["player"] = json!(p);
        }
        execute(self.state.clone(), &self.rt, "mpris.get", args)
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
    fn media_player_control(
        &self,
        Parameters(MprisControl { player, action }): Parameters<MprisControl>,
    ) -> Json<Value> {
        let mut args = json!({"action": action});
        if let Some(p) = player {
            args["player"] = json!(p);
        }
        execute(self.state.clone(), &self.rt, "mpris.control", args)
    }
    };
}
