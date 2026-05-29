use crate::protocol::Action;
use serde_json::Value;

pub(super) fn parse_audio(raw: &Value, _id: &str, type_str: &str) -> anyhow::Result<Action> {
    Ok(match type_str {
        "audio.list_sinks" => Action::AudioListSinks,
        "audio.set_sink_volume" => Action::AudioSetSinkVolume {
            sink_id: raw["sink_id"].as_u64().unwrap_or(0) as u32,
            volume: raw["volume"].as_f64().unwrap_or(1.0),
        },
        "audio.list_sources" => Action::AudioListSources,
        "audio.get_volume" => Action::AudioGetVolume {
            target: raw["target"].as_str().unwrap_or("sink").to_string(),
            id: raw["id"].as_u64().unwrap_or(0) as u32,
        },
        "audio.set_volume" => Action::AudioSetVolume {
            target: raw["target"].as_str().unwrap_or("sink").to_string(),
            id: raw["id"].as_u64().unwrap_or(0) as u32,
            volume: raw["volume"].as_f64().unwrap_or(1.0),
        },
        "audio.mute" => Action::AudioMute {
            target: raw["target"].as_str().unwrap_or("sink").to_string(),
            id: raw["id"].as_u64().unwrap_or(0) as u32,
            mute: raw["mute"].as_bool().unwrap_or(true),
        },
        "audio.set_default" => Action::AudioSetDefault {
            target: raw["target"].as_str().unwrap_or("sink").to_string(),
            name: raw["name"].as_str().unwrap_or("").to_string(),
        },
        _ => anyhow::bail!("unknown audio type: {type_str}"),
    })
}
