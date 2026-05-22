use super::*;
use crate::protocol::DeskbridEvent;

pub(super) async fn files_watch(
    backend: &NiriBackend,
    path: &str,
    recursive: bool,
    _patterns: Option<&[String]>,
) -> anyhow::Result<()> {
    use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
    let watched_path = path.to_string();
    let tx = backend.event_tx.clone();
    let mut watcher = RecommendedWatcher::new(
        move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let path_str = event
                    .paths
                    .first()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                match event.kind {
                    EventKind::Create(_) => {
                        let _ = tx.send(DeskbridEvent::FileCreated {
                            path: path_str,
                            timestamp: ts,
                        });
                    }
                    EventKind::Modify(_) => {
                        let _ = tx.send(DeskbridEvent::FileModified {
                            path: path_str,
                            timestamp: ts,
                        });
                    }
                    EventKind::Remove(_) => {
                        let _ = tx.send(DeskbridEvent::FileDeleted {
                            path: path_str,
                            timestamp: ts,
                        });
                    }
                    _ => {}
                }
            }
        },
        Config::default(),
    )?;
    let mode = if recursive {
        RecursiveMode::Recursive
    } else {
        RecursiveMode::NonRecursive
    };
    watcher.watch(std::path::Path::new(&watched_path), mode)?;
    backend
        .watchers
        .lock()
        .map_err(|e| anyhow::anyhow!("mutex poisoned: {}", e))?
        .insert(watched_path, watcher);
    Ok(())
}

pub(super) async fn files_unwatch(backend: &NiriBackend, path: &str) -> anyhow::Result<()> {
    backend
        .watchers
        .lock()
        .map_err(|e| anyhow::anyhow!("mutex poisoned: {}", e))?
        .remove(path);
    Ok(())
}

pub(super) async fn files_search(
    _backend: &NiriBackend,
    pattern: &str,
    root: Option<&str>,
    max_results: u32,
) -> anyhow::Result<Vec<String>> {
    let search_root = root.unwrap_or(".");
    let out = Command::new("find")
        .args([
            search_root,
            "-maxdepth",
            "5",
            "-iname",
            pattern,
            "-not",
            "-path",
            "*/.*",
        ])
        .output()
        .await?;
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .take(max_results as usize)
        .map(|s| s.to_string())
        .collect())
}
