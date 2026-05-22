use super::*;
use crate::protocol;

pub(super) async fn audio_list_sinks(
    backend: &SwayBackend,
) -> anyhow::Result<Vec<protocol::AudioSinkInfo>> {
    let out = backend.sh("pactl", &["list", "sinks"]).await?;
    let mut sinks = Vec::new();
    let mut current_id = 0u32;
    let mut current_name = String::new();
    let mut current_desc = String::new();
    let mut current_volume: f64 = 0.0;
    let mut current_muted = false;

    for line in out.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Sink #") {
            if current_id > 0 {
                sinks.push(protocol::AudioSinkInfo {
                    id: current_id,
                    name: std::mem::take(&mut current_name),
                    description: std::mem::take(&mut current_desc),
                    volume: current_volume,
                    muted: current_muted,
                });
            }
            current_id = trimmed
                .strip_prefix("Sink #")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            current_name.clear();
            current_desc.clear();
            current_volume = 0.0;
            current_muted = false;
        } else if trimmed.starts_with("Description: ") {
            current_desc = trimmed
                .strip_prefix("Description: ")
                .unwrap_or("")
                .to_string();
            current_name = current_desc.clone();
        } else if trimmed.starts_with("Volume: ") {
            if let Some(vol_str) = trimmed.strip_prefix("Volume: ") {
                current_volume = vol_str
                    .split('%')
                    .next()
                    .and_then(|s| s.trim().parse::<u32>().ok())
                    .map(|v| v as f64 / 100.0)
                    .unwrap_or(0.0);
            }
        } else if trimmed.starts_with("Mute: ") {
            current_muted = trimmed
                .strip_prefix("Mute: ")
                .map(|s| s.trim() == "yes")
                .unwrap_or(false);
        }
    }
    if current_id > 0 {
        sinks.push(protocol::AudioSinkInfo {
            id: current_id,
            name: current_name,
            description: current_desc,
            volume: current_volume,
            muted: current_muted,
        });
    }
    Ok(sinks)
}

pub(super) async fn audio_set_sink_volume(
    backend: &SwayBackend,
    sink_id: u32,
    volume: f64,
) -> anyhow::Result<()> {
    backend
        .sh(
            "pactl",
            &[
                "set-sink-volume",
                &sink_id.to_string(),
                &format!("{}%", (volume * 100.0) as u32),
            ],
        )
        .await
        .map(|_| ())
}
