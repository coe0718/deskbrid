use super::GnomeBackend;
use crate::protocol;

impl GnomeBackend {
    pub(super) async fn audio_list_sinks_inner(
        &self,
    ) -> anyhow::Result<Vec<protocol::AudioSinkInfo>> {
        let out = self.sh("pactl", &["list", "sinks"]).await?;
        parse_pactl_sinks(&out)
    }

    pub(super) async fn audio_set_sink_volume_inner(
        &self,
        sink_id: u32,
        volume: f64,
    ) -> anyhow::Result<()> {
        let vol_pct = (volume * 100.0) as u32;
        self.sh(
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
}

fn parse_pactl_sinks(raw: &str) -> anyhow::Result<Vec<protocol::AudioSinkInfo>> {
    let mut sinks = Vec::new();
    let mut current_id = 0u32;
    let mut current_name = String::new();
    let mut current_desc = String::new();
    let mut current_volume = 0.0f64;
    let mut current_muted = false;
    let mut in_sink = false;

    for line in raw.lines() {
        if line.starts_with("Sink #") {
            if in_sink {
                sinks.push(protocol::AudioSinkInfo {
                    id: current_id,
                    name: current_name.clone(),
                    description: current_desc.clone(),
                    volume: current_volume,
                    muted: current_muted,
                });
            }
            in_sink = true;
            current_name.clear();
            current_desc.clear();
            current_volume = 0.0;
            current_muted = false;
            current_id = line
                .strip_prefix("Sink #")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        } else if in_sink {
            let trimmed = line.trim();
            if let Some(v) = trimmed.strip_prefix("Name: ") {
                current_name = v.to_string();
            } else if let Some(v) = trimmed.strip_prefix("Description: ") {
                current_desc = v.to_string();
            } else if trimmed.starts_with("Mute: ") {
                current_muted = trimmed.contains("yes");
            } else if trimmed.starts_with("Volume:") {
                if let Some(pct) = trimmed.split('/').nth(1) {
                    current_volume = pct
                        .trim()
                        .trim_end_matches('%')
                        .parse::<f64>()
                        .unwrap_or(0.0)
                        / 100.0;
                }
            }
        }
    }
    if in_sink {
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
