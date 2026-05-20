use super::GnomeBackend;
use crate::protocol::DeskbridEvent;
use notify::Watcher;
use tracing::debug;

impl GnomeBackend {
    pub(super) async fn files_watch_inner(
        &self,
        path: &str,
        recursive: bool,
        _patterns: Option<&[String]>,
    ) -> anyhow::Result<()> {
        let meta = tokio::fs::metadata(path).await?;
        if !meta.is_dir() && !meta.is_file() {
            anyhow::bail!("path does not exist: {}", path);
        }

        let path_owned = path.to_string();
        let event_tx = self.event_tx.clone();
        let mode = if recursive {
            notify::RecursiveMode::Recursive
        } else {
            notify::RecursiveMode::NonRecursive
        };

        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                if let Ok(event) = event {
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    for path in &event.paths {
                        let path_str = path.to_string_lossy().to_string();
                        let evt = match event.kind {
                            notify::EventKind::Create(_) => DeskbridEvent::FileCreated {
                                path: path_str,
                                timestamp: ts,
                            },
                            notify::EventKind::Modify(_) => DeskbridEvent::FileModified {
                                path: path_str,
                                timestamp: ts,
                            },
                            notify::EventKind::Remove(_) => DeskbridEvent::FileDeleted {
                                path: path_str,
                                timestamp: ts,
                            },
                            _ => continue,
                        };
                        let _ = event_tx.send(evt);
                    }
                }
            })?;

        watcher.watch(std::path::Path::new(&path_owned), mode)?;
        let mut guard = self.watchers.lock().unwrap();
        guard.insert(path_owned.clone(), watcher);
        debug!(
            "File watch active on {} (recursive={})",
            path_owned, recursive
        );
        Ok(())
    }

    pub(super) async fn files_unwatch_inner(&self, path: &str) -> anyhow::Result<()> {
        let mut guard = self.watchers.lock().unwrap();
        guard.remove(path);
        debug!("File watch removed on {}", path);
        Ok(())
    }

    pub(super) async fn files_search_inner(
        &self,
        pattern: &str,
        root: Option<&str>,
        max_results: u32,
    ) -> anyhow::Result<Vec<String>> {
        let base = root.unwrap_or(".");
        if self.sh_ok("which", &["fd"]).await {
            let out = self
                .sh(
                    "fd",
                    &[
                        "--max-results",
                        &max_results.to_string(),
                        "--search-path",
                        base,
                        pattern,
                    ],
                )
                .await?;
            Ok(out.lines().map(|l| l.to_string()).collect())
        } else {
            let out = self
                .sh("find", &[base, "-name", pattern, "-maxdepth", "10"])
                .await?;
            Ok(out
                .lines()
                .take(max_results as usize)
                .map(|l| l.to_string())
                .collect())
        }
    }
}
