// TESTING_NEEDED: This feature requires manual testing on a live desktop environment
use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::Action;
use crate::protocol::AudioSourceInfo;
use serde_json::Value;

pub(crate) async fn execute_audio(
    action: Action,
    backend: &dyn DesktopBackend,
    _state: &DaemonState,
) -> anyhow::Result<Value> {
    use Action::*;
    Ok(match action {
        AudioListSinks => serde_json::json!(backend.audio_list_sinks().await?),
        AudioSetSinkVolume { sink_id, volume } => {
            backend.audio_set_sink_volume(sink_id, volume).await?;
            serde_json::json!({"sink": sink_id, "volume": volume})
        }

        // New audio actions — DE-agnostic, using pactl directly
        AudioListSources => serde_json::json!(list_sources().await?),
        AudioGetVolume { target, id } => {
            let vol = get_volume(&target, id).await?;
            serde_json::json!({"target": target, "id": id, "volume": vol})
        }
        AudioSetVolume { target, id, volume } => {
            set_volume(&target, id, volume).await?;
            serde_json::json!({"target": target, "id": id, "volume": volume})
        }
        AudioMute { target, id, mute } => {
            set_mute(&target, id, mute).await?;
            serde_json::json!({"target": target, "id": id, "muted": mute})
        }
        AudioSetDefault { target, name } => {
            set_default(&target, &name).await?;
            serde_json::json!({"target": target, "name": name, "default": true})
        }

        _ => unreachable!("not an audio action"),
    })
}

async fn list_sources() -> anyhow::Result<Vec<AudioSourceInfo>> {
    let output = tokio::process::Command::new("pactl")
        .args(["list", "sources"])
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut sources = Vec::new();
    let mut id = 0u32;
    let mut name = String::new();
    let mut desc = String::new();
    let mut volume = 0.0_f64;
    let mut muted = false;

    for line in stdout.lines() {
        let t = line.trim();
        if t.starts_with("Source #") {
            if id > 0 {
                sources.push(AudioSourceInfo {
                    id,
                    name: std::mem::take(&mut name),
                    description: std::mem::take(&mut desc),
                    volume,
                    muted,
                });
            }
            id = t
                .strip_prefix("Source #")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            name.clear();
            desc.clear();
            volume = 0.0;
            muted = false;
        } else if let Some(v) = t.strip_prefix("Description: ") {
            desc = v.to_string();
            name = v.to_string();
        } else if let Some(v) = t.strip_prefix("Volume: ") {
            volume = v
                .split('%')
                .next()
                .and_then(|s| s.trim().parse::<u32>().ok())
                .map(|pct| pct as f64 / 100.0)
                .unwrap_or(0.0);
        } else if let Some(v) = t.strip_prefix("Mute: ") {
            muted = v.trim() == "yes";
        }
    }
    if id > 0 {
        sources.push(AudioSourceInfo {
            id,
            name,
            description: desc,
            volume,
            muted,
        });
    }
    Ok(sources)
}

async fn get_volume(target: &str, id: u32) -> anyhow::Result<f64> {
    let what = if target == "source" {
        "sources"
    } else {
        "sinks"
    };
    let output = tokio::process::Command::new("pactl")
        .args(["list", what])
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let needle = format!(
        "{} #{}",
        if target == "source" { "Source" } else { "Sink" },
        id
    );

    let mut in_target = false;
    for line in stdout.lines() {
        let t = line.trim();
        if t.starts_with(&needle) {
            in_target = true;
        } else if in_target && t.starts_with("Volume: ") {
            return Ok(t
                .strip_prefix("Volume: ")
                .and_then(|v| v.split('%').next())
                .and_then(|s| s.trim().parse::<u32>().ok())
                .map(|pct| pct as f64 / 100.0)
                .unwrap_or(0.0));
        } else if in_target && (t.starts_with("Sink #") || t.starts_with("Source #")) {
            break;
        }
    }
    anyhow::bail!("{} #{} not found", target, id)
}

async fn set_volume(target: &str, id: u32, volume: f64) -> anyhow::Result<()> {
    let cmd = if target == "source" {
        "set-source-volume"
    } else {
        "set-sink-volume"
    };
    let pct = (volume * 100.0).round() as u32;
    tokio::process::Command::new("pactl")
        .args([cmd, &id.to_string(), &format!("{}%", pct)])
        .output()
        .await?;
    Ok(())
}

async fn set_mute(target: &str, id: u32, mute: bool) -> anyhow::Result<()> {
    let cmd = if target == "source" {
        "set-source-mute"
    } else {
        "set-sink-mute"
    };
    let state = if mute { "1" } else { "0" };
    tokio::process::Command::new("pactl")
        .args([cmd, &id.to_string(), state])
        .output()
        .await?;
    Ok(())
}

async fn set_default(target: &str, name: &str) -> anyhow::Result<()> {
    let cmd = if target == "source" {
        "set-default-source"
    } else {
        "set-default-sink"
    };
    tokio::process::Command::new("pactl")
        .args([cmd, name])
        .output()
        .await?;
    Ok(())
}
