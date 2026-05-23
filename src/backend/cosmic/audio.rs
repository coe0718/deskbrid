use super::*;
use crate::protocol;

pub(super) async fn audio_list_sinks(
    backend: &CosmicBackend,
) -> anyhow::Result<Vec<protocol::AudioSinkInfo>> {
    let output = backend.sh("pactl", &["list", "sinks"]).await?;
    let mut sinks = Vec::new();
    let mut id = 0u32;
    let mut name = String::new();
    let mut description = String::new();
    let mut volume = 0.0;
    let mut muted = false;

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Sink #") {
            if id > 0 {
                sinks.push(protocol::AudioSinkInfo {
                    id,
                    name: std::mem::take(&mut name),
                    description: std::mem::take(&mut description),
                    volume,
                    muted,
                });
            }
            id = trimmed
                .strip_prefix("Sink #")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            volume = 0.0;
            muted = false;
        } else if let Some(value) = trimmed.strip_prefix("Name: ") {
            name = value.to_string();
        } else if let Some(value) = trimmed.strip_prefix("Description: ") {
            description = value.to_string();
        } else if trimmed.starts_with("Volume:") {
            if let Some(percent) = trimmed.split('/').nth(1) {
                volume = percent
                    .trim()
                    .strip_suffix('%')
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0)
                    / 100.0;
            }
        } else if let Some(value) = trimmed.strip_prefix("Mute: ") {
            muted = value.trim() == "yes";
        }
    }

    if id > 0 {
        sinks.push(protocol::AudioSinkInfo {
            id,
            name,
            description,
            volume,
            muted,
        });
    }

    Ok(sinks)
}

pub(super) async fn audio_set_sink_volume(
    backend: &CosmicBackend,
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

// ═══════════════════════════════════════════════════════
// MONITOR (via cosmic-randr)
// ═══════════════════════════════════════════════════════
