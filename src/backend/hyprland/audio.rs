use super::*;
use crate::protocol;

pub(super) async fn audio_list_sinks(
    backend: &HyprBackend,
) -> anyhow::Result<Vec<protocol::AudioSinkInfo>> {
    let output = backend
        .sh("pactl", &["list", "short", "sinks"])
        .await
        .unwrap_or_default();
    let mut sinks = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 2 {
            sinks.push(protocol::AudioSinkInfo {
                id: parts[0].parse().unwrap_or(0),
                name: parts[1].to_string(),
                description: String::new(),
                volume: 1.0,
                muted: false,
            });
        }
    }
    Ok(sinks)
}

pub(super) async fn audio_set_sink_volume(
    backend: &HyprBackend,
    sink_id: u32,
    volume: f64,
) -> anyhow::Result<()> {
    let vol_pct = (volume * 100.0) as u32;
    backend
        .sh(
            "pactl",
            &[
                "set-sink-volume",
                &sink_id.to_string(),
                &format!("{}%", vol_pct),
            ],
        )
        .await?;
    Ok(())
}
