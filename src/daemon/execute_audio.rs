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

        _ => anyhow::bail!("internal dispatch error: not an audio action"),
    })
}

// pactl localizes its output (LC_MESSAGES); our parsers match English keywords.
// Force the C locale on every pactl invocation so parsing never breaks under
// non-English desktop locales.
fn pactl(args: &[&str]) -> tokio::process::Command {
    let mut c = tokio::process::Command::new("pactl");
    c.args(args).env("LC_ALL", "C");
    c
}

/// Parse a `Volume:` line like `front-left: 65536 /  95% / -1.33 dB, ...`.
fn parse_volume_pct(v: &str) -> f64 {
    v.split('%')
        .next()
        .and_then(|s| s.rsplit('/').next())
        .and_then(|s| s.trim().parse::<u32>().ok())
        .map(|pct| pct as f64 / 100.0)
        .unwrap_or(0.0)
}

async fn list_sources() -> anyhow::Result<Vec<AudioSourceInfo>> {
    let output = pactl(&["list", "sources"]).output().await?;
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
            volume = parse_volume_pct(v);
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
    let output = pactl(&["list", what]).output().await?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    // id == 0 means "no explicit id" — resolve the default sink/source by name.
    // Sink ids are allocator-dependent (e.g. PipeWire starts at 64), so 0 is
    // never a valid fallback target.
    let needle = if id == 0 {
        let get_default = if target == "source" {
            "get-default-source"
        } else {
            "get-default-sink"
        };
        let out = pactl(&[get_default]).output().await?;
        if !out.status.success() {
            anyhow::bail!("no {target} id given and no default {target} found");
        }
        let default = String::from_utf8_lossy(&out.stdout).trim().to_string();
        format!("Name: {default}")
    } else {
        format!(
            "{} #{}",
            if target == "source" { "Source" } else { "Sink" },
            id
        )
    };

    let mut in_target = false;
    for line in stdout.lines() {
        let t = line.trim();
        if t.starts_with("Sink #") || t.starts_with("Source #") {
            if in_target {
                break;
            }
            if id != 0 && t.starts_with(&needle) {
                in_target = true;
            }
        } else if id == 0 && !in_target && t == needle {
            in_target = true;
        } else if in_target && t.starts_with("Volume: ") {
            return Ok(t
                .strip_prefix("Volume: ")
                .map(parse_volume_pct)
                .unwrap_or(0.0));
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
    pactl(&[cmd, &id.to_string(), &format!("{}%", pct)])
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
    pactl(&[cmd, &id.to_string(), state]).output().await?;
    Ok(())
}

async fn set_default(target: &str, name: &str) -> anyhow::Result<()> {
    let cmd = if target == "source" {
        "set-default-source"
    } else {
        "set-default-sink"
    };
    pactl(&[cmd, name]).output().await?;
    Ok(())
}
