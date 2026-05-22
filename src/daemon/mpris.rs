use std::collections::HashMap;

use crate::DaemonState;
use crate::protocol::{Action, MprisPlayerInfo};
use zbus::zvariant;

use super::mpris_convert::*;

const MPRIS_PREFIX: &str = "org.mpris.MediaPlayer2.";
const MPRIS_PATH: &str = "/org/mpris/MediaPlayer2";

pub(crate) fn is_mpris_action(action: &Action) -> bool {
    matches!(
        action,
        Action::MprisList | Action::MprisGet { .. } | Action::MprisControl { .. }
    )
}

pub(crate) async fn execute_mpris_action(
    action: Action,
    _state: &DaemonState,
) -> anyhow::Result<serde_json::Value> {
    match action {
        Action::MprisList => {
            let players = list_players().await?;
            Ok(serde_json::json!({"players": players, "count": players.len()}))
        }
        Action::MprisGet { player } => {
            let bus_name = resolve_player(player.as_deref()).await?;
            Ok(serde_json::json!(player_info(&bus_name).await?))
        }
        Action::MprisControl { player, action } => {
            let bus_name = resolve_player(player.as_deref()).await?;
            let method = mpris_method(&action)?;
            let conn = zbus::Connection::session().await?;
            conn.call_method(
                Some(bus_name.as_str()),
                MPRIS_PATH,
                Some("org.mpris.MediaPlayer2.Player"),
                method,
                &(),
            )
            .await?;
            Ok(serde_json::json!({"player": bus_name, "action": action}))
        }
        _ => anyhow::bail!("not an mpris action"),
    }
}

async fn list_players() -> anyhow::Result<Vec<MprisPlayerInfo>> {
    let names = mpris_bus_names().await?;
    let mut players = Vec::new();
    for name in names {
        if let Ok(info) = player_info(&name).await {
            players.push(info);
        }
    }
    Ok(players)
}

async fn resolve_player(player: Option<&str>) -> anyhow::Result<String> {
    let names = mpris_bus_names().await?;
    if names.is_empty() {
        anyhow::bail!("no MPRIS players found");
    }
    let Some(player) = player else {
        return Ok(names[0].clone());
    };
    let player_l = player.to_lowercase();
    names
        .into_iter()
        .find(|name| {
            name.eq_ignore_ascii_case(player)
                || name
                    .strip_prefix(MPRIS_PREFIX)
                    .is_some_and(|id| id.eq_ignore_ascii_case(player))
                || name.to_lowercase().contains(&player_l)
        })
        .ok_or_else(|| anyhow::anyhow!("MPRIS player not found: {}", player))
}

async fn mpris_bus_names() -> anyhow::Result<Vec<String>> {
    let conn = zbus::Connection::session().await?;
    let reply = conn
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "ListNames",
            &(),
        )
        .await?;
    let (names,): (Vec<String>,) = reply.body().deserialize()?;
    Ok(names
        .into_iter()
        .filter(|name| name.starts_with(MPRIS_PREFIX))
        .collect())
}

async fn player_info(bus_name: &str) -> anyhow::Result<MprisPlayerInfo> {
    let conn = zbus::Connection::session().await?;
    let root_props = get_all(&conn, bus_name, "org.mpris.MediaPlayer2").await?;
    let player_props = get_all(&conn, bus_name, "org.mpris.MediaPlayer2.Player").await?;

    Ok(MprisPlayerInfo {
        bus_name: bus_name.to_string(),
        player_id: bus_name
            .strip_prefix(MPRIS_PREFIX)
            .unwrap_or(bus_name)
            .to_string(),
        identity: get_string(&root_props, "Identity"),
        playback_status: get_string(&player_props, "PlaybackStatus"),
        metadata: player_props
            .get("Metadata")
            .map(owned_value_to_json)
            .unwrap_or_else(|| serde_json::json!({})),
        volume: player_props
            .get("Volume")
            .and_then(|value| value.downcast_ref::<f64>().ok()),
        can_play: get_bool(&player_props, "CanPlay"),
        can_pause: get_bool(&player_props, "CanPause"),
        can_go_next: get_bool(&player_props, "CanGoNext"),
        can_go_previous: get_bool(&player_props, "CanGoPrevious"),
    })
}

async fn get_all(
    conn: &zbus::Connection,
    bus_name: &str,
    interface: &str,
) -> anyhow::Result<HashMap<String, zvariant::OwnedValue>> {
    let reply = conn
        .call_method(
            Some(bus_name),
            MPRIS_PATH,
            Some("org.freedesktop.DBus.Properties"),
            "GetAll",
            &(interface,),
        )
        .await?;
    Ok(reply.body().deserialize()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_mpris_actions_to_methods() {
        assert_eq!(mpris_method("play_pause").unwrap(), "PlayPause");
        assert_eq!(mpris_method("next").unwrap(), "Next");
        assert!(mpris_method("shuffle").is_err());
    }
}
