use super::*;
use crate::protocol::DeskbridEvent;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

pub(super) async fn files_watch(
    backend: &X11Backend,
    path: &str,
    recursive: bool,
    _patterns: Option<&[String]>,
) -> anyhow::Result<()> {
    let watched_path = path.to_string();
    let tx = backend.event_tx.clone();

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let path = event
                    .paths
                    .first()
                    .cloned()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                match event.kind {
                    EventKind::Create(_) => {
                        let _ = tx.send(DeskbridEvent::FileCreated {
                            path,
                            timestamp: ts,
                        });
                    }
                    EventKind::Modify(_) => {
                        let _ = tx.send(DeskbridEvent::FileModified {
                            path,
                            timestamp: ts,
                        });
                    }
                    EventKind::Remove(_) => {
                        let _ = tx.send(DeskbridEvent::FileDeleted {
                            path,
                            timestamp: ts,
                        });
                    }
                    _ => {}
                }
            }
        },
        Config::default(),
    )?;

    watcher.watch(
        std::path::Path::new(&watched_path),
        if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        },
    )?;

    backend
        .watchers
        .lock()
        .map_err(|e| anyhow::anyhow!("mutex poisoned: {}", e))?
        .insert(watched_path, watcher);
    Ok(())
}

pub(super) async fn files_unwatch(backend: &X11Backend, path: &str) -> anyhow::Result<()> {
    backend
        .watchers
        .lock()
        .map_err(|e| anyhow::anyhow!("mutex poisoned: {}", e))?
        .remove(path);
    Ok(())
}

pub(super) async fn files_search(
    backend: &X11Backend,
    pattern: &str,
    root: Option<&str>,
    max_results: u32,
) -> anyhow::Result<Vec<String>> {
    let search_root = root.unwrap_or(".");
    let glob = if pattern.contains('*') {
        pattern.to_string()
    } else {
        format!("*{}*", pattern)
    };
    let output = backend
        .sh(
            "find",
            &[
                search_root,
                "-maxdepth",
                "5",
                "-iname",
                &glob,
                "-not",
                "-path",
                "*/.*",
            ],
        )
        .await?;
    Ok(output
        .lines()
        .take(max_results as usize)
        .map(String::from)
        .collect())
}
