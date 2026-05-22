use super::*;

pub(super) async fn files_watch(
    backend: &HyprBackend,
    path: &str,
    recursive: bool,
    _patterns: Option<&[String]>,
) -> anyhow::Result<()> {
    use notify::*;
    let event_tx = backend.event_tx.clone();
    let watch_path = path.to_string();
    let recursive_mode = if recursive {
        RecursiveMode::Recursive
    } else {
        RecursiveMode::NonRecursive
    };
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res {
            let path_str = event.paths.first().map(|p| p.to_string_lossy().to_string());
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            if event.kind.is_create() {
                let _ = event_tx.send(DeskbridEvent::FileCreated {
                    path: path_str.unwrap_or_default(),
                    timestamp: ts,
                });
            } else if event.kind.is_modify() {
                let _ = event_tx.send(DeskbridEvent::FileModified {
                    path: path_str.unwrap_or_default(),
                    timestamp: ts,
                });
            } else if event.kind.is_remove() {
                let _ = event_tx.send(DeskbridEvent::FileDeleted {
                    path: path_str.unwrap_or_default(),
                    timestamp: ts,
                });
            }
        }
    })?;
    watcher.watch(std::path::Path::new(&watch_path), recursive_mode)?;
    let mut watchers = backend.watchers.lock().unwrap();
    watchers.insert(watch_path, watcher);
    Ok(())
}

pub(super) async fn files_unwatch(backend: &HyprBackend, path: &str) -> anyhow::Result<()> {
    let mut watchers = backend.watchers.lock().unwrap();
    watchers.remove(path);
    Ok(())
}

pub(super) async fn files_search(
    backend: &HyprBackend,
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
