use super::*;
use crate::protocol;

pub(super) async fn audio_list_sinks(
    backend: &LabwcBackend,
) -> anyhow::Result<Vec<protocol::AudioSinkInfo>> {
    let o = backend.sh("pactl", &["list", "sinks"]).await?;
    let mut sinks = Vec::new();
    let mut id = 0u32;
    let mut name = String::new();
    let mut desc = String::new();
    let mut vol: f64 = 0.0;
    let mut muted = false;
    for l in o.lines() {
        let t = l.trim();
        if t.starts_with("Sink #") {
            if id > 0 {
                sinks.push(protocol::AudioSinkInfo {
                    id,
                    name: std::mem::take(&mut name),
                    description: std::mem::take(&mut desc),
                    volume: vol,
                    muted,
                });
            }
            id = t
                .strip_prefix("Sink #")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            vol = 0.0;
            muted = false;
        } else if t.starts_with("Name: ") {
            name = t.strip_prefix("Name: ").unwrap_or("").to_string();
        } else if t.starts_with("Description: ") {
            desc = t.strip_prefix("Description: ").unwrap_or("").to_string();
        } else if t.starts_with("Volume:") {
            if let Some(pct) = t.split('/').nth(1) {
                vol = pct
                    .trim()
                    .strip_suffix('%')
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0)
                    / 100.0;
            }
        } else if t.starts_with("Mute: ") {
            muted = t.strip_prefix("Mute: ").unwrap_or("").trim() == "yes";
        }
    }
    if id > 0 {
        sinks.push(protocol::AudioSinkInfo {
            id,
            name,
            description: desc,
            volume: vol,
            muted,
        });
    }
    Ok(sinks)
}

pub(super) async fn audio_set_sink_volume(
    backend: &LabwcBackend,
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
