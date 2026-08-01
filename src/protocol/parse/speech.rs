use crate::protocol::Action;
use serde_json::Value;

pub(super) fn parse_speech(raw: &Value, _id: &str, type_str: &str) -> anyhow::Result<Action> {
    Ok(match type_str {
        // Speech / Text-to-Speech
        "speech.speak" => Action::SpeechSpeak {
            text: raw["text"].as_str().unwrap_or("").into(),
            voice: raw["voice"].as_str().map(String::from),
            rate: raw["rate"].as_i64().map(|v| v as i32),
            pitch: raw["pitch"].as_i64().map(|v| v as i32),
            engine: raw["engine"].as_str().map(String::from),
            wait: raw["wait"].as_bool().unwrap_or(false),
        },
        "speech.stop" => Action::SpeechStop,
        "speech.voices" => Action::SpeechListVoices,
        _ => anyhow::bail!("unknown speech type: {type_str}"),
    })
}
