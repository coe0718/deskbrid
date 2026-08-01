use super::Action;
use serde_json::json;

pub(super) fn serialize_speech(action: &Action, id: &str) -> serde_json::Value {
    match action {
        Action::SpeechSpeak {
            text,
            voice,
            rate,
            pitch,
            engine,
            wait,
        } => {
            let mut obj = json!({
                "type": "speech.speak",
                "id": id,
                "text": text,
                "wait": wait
            });
            if let Some(voice) = voice {
                obj["voice"] = json!(voice);
            }
            if let Some(rate) = rate {
                obj["rate"] = json!(rate);
            }
            if let Some(pitch) = pitch {
                obj["pitch"] = json!(pitch);
            }
            if let Some(engine) = engine {
                obj["engine"] = json!(engine);
            }
            obj
        }
        Action::SpeechStop => json!({"type": "speech.stop", "id": id}),
        Action::SpeechListVoices => json!({"type": "speech.voices", "id": id}),
        _ => json!({"error": "not a speech action"}),
    }
}
