use super::*;
use crate::protocol;

pub(super) async fn audio_list_sinks(
    backend: &WayfireBackend,
) -> anyhow::Result<Vec<protocol::AudioSinkInfo>> {
    let out = backend.sh("pactl", &["list", "sinks"]).await?;
    let mut sinks = Vec::new();
    let mut cur_id = 0u32;
    let mut cur_name = String::new();
    let mut cur_desc = String::new();
    let mut cur_vol: f64 = 0.0;
    let mut cur_muted = false;
    for line in out.lines() {
        let t = line.trim();
        if t.starts_with("Sink #") {
            if cur_id > 0 {
                sinks.push(protocol::AudioSinkInfo {
                    id: cur_id,
                    name: std::mem::take(&mut cur_name),
                    description: std::mem::take(&mut cur_desc),
                    volume: cur_vol,
                    muted: cur_muted,
                });
            }
            cur_id = t
                .strip_prefix("Sink #")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            cur_name.clear();
            cur_desc.clear();
            cur_vol = 0.0;
            cur_muted = false;
        } else if t.starts_with("Description: ") {
            cur_desc = t.strip_prefix("Description: ").unwrap_or("").to_string();
            cur_name = cur_desc.clone();
        } else if t.starts_with("Volume: ") {
            cur_vol = t
                .strip_prefix("Volume: ")
                .and_then(|v| {
                    v.split('%')
                        .next()
                        .and_then(|s| s.trim().parse::<u32>().ok())
                })
                .map(|v| v as f64 / 100.0)
                .unwrap_or(0.0);
        } else if t.starts_with("Mute: ") {
            cur_muted = t
                .strip_prefix("Mute: ")
                .map(|s| s.trim() == "yes")
                .unwrap_or(false);
        }
    }
    if cur_id > 0 {
        sinks.push(protocol::AudioSinkInfo {
            id: cur_id,
            name: cur_name,
            description: cur_desc,
            volume: cur_vol,
            muted: cur_muted,
        });
    }
    Ok(sinks)
}

pub(super) async fn audio_set_sink_volume(
    backend: &WayfireBackend,
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

// Monitor controls — wf-ipc doesn't support these yet
