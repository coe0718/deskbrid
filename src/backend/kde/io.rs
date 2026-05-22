use super::*;
use crate::protocol;
use crate::protocol::DeskbridEvent;

pub(super) async fn files_watch(
    backend: &KdeBackend,
    path: &str,
    recursive: bool,
    patterns: Option<&[String]>,
) -> anyhow::Result<()> {
    use notify::{Event, EventKind, RecursiveMode, Watcher};
    let mut watchers = backend.watchers.lock().unwrap();
    if watchers.contains_key(path) {
        anyhow::bail!("already watching: {path}");
    }
    let event_tx = backend.event_tx.clone();
    let path_owned = path.to_string();
    let patterns_owned = patterns.map(|p| p.to_vec());

    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        if let Ok(event) = res {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let mut ev = match event.kind {
                EventKind::Create(_) => event.paths.first().map(|p| DeskbridEvent::FileCreated {
                    path: p.to_string_lossy().to_string(),
                    timestamp: now,
                }),
                EventKind::Modify(_) => event.paths.first().map(|p| DeskbridEvent::FileModified {
                    path: p.to_string_lossy().to_string(),
                    timestamp: now,
                }),
                EventKind::Remove(_) => event.paths.first().map(|p| DeskbridEvent::FileDeleted {
                    path: p.to_string_lossy().to_string(),
                    timestamp: now,
                }),
                _ => None,
            };
            if let Some(ref found) = ev {
                if let Some(ref pats) = patterns_owned {
                    let path_str = match found {
                        DeskbridEvent::FileCreated { path, .. } => path,
                        DeskbridEvent::FileModified { path, .. } => path,
                        DeskbridEvent::FileDeleted { path, .. } => path,
                        _ => return,
                    };
                    if !pats
                        .iter()
                        .any(|pat| path_str.ends_with(pat.trim_start_matches('*')))
                    {
                        return;
                    }
                }
                let _ = event_tx.send(ev.take().unwrap());
            }
        }
    })?;

    let mode = if recursive {
        RecursiveMode::Recursive
    } else {
        RecursiveMode::NonRecursive
    };
    watcher.watch(std::path::Path::new(path), mode)?;
    watchers.insert(path_owned, watcher);
    Ok(())
}

pub(super) async fn files_unwatch(backend: &KdeBackend, path: &str) -> anyhow::Result<()> {
    backend.watchers.lock().unwrap().remove(path);
    Ok(())
}

pub(super) async fn files_search(
    backend: &KdeBackend,
    pattern: &str,
    root: Option<&str>,
    max_results: u32,
) -> anyhow::Result<Vec<String>> {
    let root_path = root.unwrap_or(".");
    let output = backend
        .sh(
            "find",
            &[root_path, "-type", "f", "-name", pattern, "-maxdepth", "5"],
        )
        .await
        .unwrap_or_default();
    Ok(output
        .lines()
        .take(max_results as usize)
        .map(|l| l.to_string())
        .collect())
}

pub(super) async fn audio_list_sinks(
    backend: &KdeBackend,
) -> anyhow::Result<Vec<protocol::AudioSinkInfo>> {
    let out = backend
        .sh("pactl", &["list", "sinks"])
        .await
        .unwrap_or_default();
    let mut sinks = Vec::new();
    let mut current: Option<protocol::AudioSinkInfo> = None;
    for line in out.lines() {
        if line.starts_with("Sink #") {
            if let Some(sink) = current.take() {
                sinks.push(sink);
            }
            let id_str = line.trim_start_matches("Sink #");
            current = Some(protocol::AudioSinkInfo {
                id: id_str.parse().unwrap_or(0),
                name: String::new(),
                description: String::new(),
                volume: 0.0,
                muted: false,
            });
        } else if let Some(ref mut sink) = current {
            let trimmed = line.trim();
            if let Some(name) = trimmed.strip_prefix("Name: ") {
                sink.name = name.to_string();
            } else if let Some(desc) = trimmed.strip_prefix("Description: ") {
                sink.description = desc.to_string();
            } else if trimmed.starts_with("Volume:") {
                if let Some(pct_str) = trimmed.split('/').nth(1) {
                    let pct: f64 = pct_str.trim().trim_end_matches('%').parse().unwrap_or(0.0);
                    sink.volume = (pct / 100.0).clamp(0.0, 1.0);
                }
            } else if let Some(mute) = trimmed.strip_prefix("Mute: ") {
                sink.muted = mute.trim() == "yes";
            }
        }
    }
    if let Some(sink) = current.take() {
        sinks.push(sink);
    }
    Ok(sinks)
}

pub(super) async fn audio_set_sink_volume(
    backend: &KdeBackend,
    sink_id: u32,
    volume: f64,
) -> anyhow::Result<()> {
    let pct = (volume * 100.0) as u32;
    backend
        .sh(
            "pactl",
            &[
                "set-sink-volume",
                &sink_id.to_string(),
                &format!("{}%", pct),
            ],
        )
        .await?;
    Ok(())
}

pub(super) async fn monitor_set_primary(backend: &KdeBackend, output: &str) -> anyhow::Result<()> {
    backend
        .sh_owned("kscreen-doctor", vec![format!("output.{}.primary", output)])
        .await?;
    Ok(())
}

pub(super) async fn monitor_set_resolution(
    backend: &KdeBackend,
    output: &str,
    width: u32,
    height: u32,
    refresh_rate: Option<f64>,
) -> anyhow::Result<()> {
    let mode = if let Some(refresh) = refresh_rate {
        format!("{}x{}@{}", width, height, format_monitor_float(refresh))
    } else {
        backend
            .kscreen_mode_for(output, width, height)
            .await
            .unwrap_or_else(|_| format!("{}x{}", width, height))
    };
    backend
        .sh_owned(
            "kscreen-doctor",
            vec![format!("output.{}.mode.{}", output, mode)],
        )
        .await?;
    Ok(())
}

pub(super) async fn monitor_set_scale(
    backend: &KdeBackend,
    output: &str,
    scale: f64,
) -> anyhow::Result<()> {
    backend
        .sh_owned(
            "kscreen-doctor",
            vec![format!(
                "output.{}.scale.{}",
                output,
                format_monitor_float(scale)
            )],
        )
        .await?;
    Ok(())
}

pub(super) async fn monitor_set_rotation(
    backend: &KdeBackend,
    output: &str,
    rotation: &str,
) -> anyhow::Result<()> {
    backend
        .sh_owned(
            "kscreen-doctor",
            vec![format!(
                "output.{}.rotation.{}",
                output,
                kde_rotation(rotation)?
            )],
        )
        .await?;
    Ok(())
}

pub(super) async fn monitor_set_enabled(
    backend: &KdeBackend,
    output: &str,
    enabled: bool,
) -> anyhow::Result<()> {
    backend
        .sh_owned(
            "kscreen-doctor",
            vec![format!(
                "output.{}.{}",
                output,
                if enabled { "enable" } else { "disable" }
            )],
        )
        .await?;
    Ok(())
}
