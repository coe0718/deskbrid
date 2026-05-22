use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::Action;
use anyhow::Context;
use serde_json::Value;

use super::expand_path;

use tokio::io::AsyncWriteExt;

pub(crate) async fn execute_files(
    action: Action,
    backend: &dyn DesktopBackend,
    _state: &DaemonState,
) -> anyhow::Result<Value> {
    use Action::*;
    Ok(match action {
        FilesWatch {
            ref path,
            recursive,
            ref patterns,
        } => {
            backend
                .files_watch(path, recursive, patterns.as_deref())
                .await?;
            serde_json::json!({"watching": path})
        }
        FilesUnwatch { ref path } => {
            backend.files_unwatch(path).await?;
            serde_json::json!({"unwatched": path})
        }
        FilesSearch {
            ref pattern,
            ref root,
            max_results,
        } => {
            serde_json::json!({"matches": backend.files_search(pattern, root.as_deref(), max_results).await?})
        }

        FilesRead {
            ref path,
            offset,
            limit,
        } => {
            use tokio::io::{AsyncReadExt, AsyncSeekExt};
            let path = expand_path(path)?;
            let mut file = tokio::fs::File::open(&path)
                .await
                .with_context(|| format!("failed to open {}", path.display()))?;
            let metadata = file.metadata().await?;
            // Cap reads at 10 MB to avoid OOM on large files
            let max_read = 10 * 1024 * 1024u64;
            let limit = limit.unwrap_or(max_read).min(max_read);
            if let Some(off) = offset {
                file.seek(std::io::SeekFrom::Start(off)).await?;
            }
            let mut buf = vec![0u8; limit as usize];
            let n = file.read(&mut buf).await?;
            buf.truncate(n);
            // Try UTF-8; fall back to base64 for binary files
            match String::from_utf8(buf) {
                Ok(text) => serde_json::json!({
                    "path": path.to_string_lossy(),
                    "content": text,
                    "bytes": n,
                    "size": metadata.len(),
                    "encoding": "utf-8",
                }),
                Err(e) => {
                    let bytes = e.into_bytes();
                    serde_json::json!({
                        "path": path.to_string_lossy(),
                        "content": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes),
                        "bytes": n,
                        "size": metadata.len(),
                        "encoding": "base64",
                    })
                }
            }
        }
        FilesWrite {
            ref path,
            ref content,
            append,
        } => {
            let path = expand_path(path)?;
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            let mut file = if append {
                tokio::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .await?
            } else {
                tokio::fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(&path)
                    .await?
            };
            file.write_all(content.as_bytes()).await?;
            file.flush().await?;
            serde_json::json!({"path": path.to_string_lossy(), "bytes_written": content.len()})
        }
        FilesCopy {
            ref source,
            ref destination,
        } => {
            let src = expand_path(source)?;
            let dst = expand_path(destination)?;
            if src.is_dir() {
                anyhow::bail!("directory copy not supported — use process.start with cp -r");
            }
            if let Some(parent) = dst.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::copy(&src, &dst).await.with_context(|| {
                format!("failed to copy {} -> {}", src.display(), dst.display())
            })?;
            serde_json::json!({"source": src.to_string_lossy(), "destination": dst.to_string_lossy()})
        }
        FilesMove {
            ref source,
            ref destination,
        } => {
            let src = expand_path(source)?;
            let dst = expand_path(destination)?;
            if let Some(parent) = dst.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::rename(&src, &dst).await.with_context(|| {
                format!("failed to move {} -> {}", src.display(), dst.display())
            })?;
            serde_json::json!({"source": src.to_string_lossy(), "destination": dst.to_string_lossy()})
        }
        FilesDelete {
            ref path,
            recursive,
        } => {
            let path = expand_path(path)?;
            if path.is_dir() {
                if recursive {
                    tokio::fs::remove_dir_all(&path).await?;
                } else {
                    tokio::fs::remove_dir(&path).await?;
                }
            } else {
                tokio::fs::remove_file(&path).await?;
            }
            serde_json::json!({"deleted": path.to_string_lossy()})
        }
        FilesMkdir { ref path, parents } => {
            let path = expand_path(path)?;
            if parents {
                tokio::fs::create_dir_all(&path).await?;
            } else {
                tokio::fs::create_dir(&path).await?;
            }
            serde_json::json!({"created": path.to_string_lossy()})
        }
        FilesList { ref path } => {
            let path = expand_path(path)?;
            let mut entries = Vec::new();
            let mut dir = tokio::fs::read_dir(&path)
                .await
                .with_context(|| format!("failed to list {}", path.display()))?;
            while let Some(entry) = dir.next_entry().await? {
                let metadata = entry.metadata().await?;
                entries.push(serde_json::json!({
                    "name": entry.file_name().to_string_lossy(),
                    "is_dir": metadata.is_dir(),
                    "size": metadata.len(),
                    "modified": metadata.modified().ok().map(|t| {
                        t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
                    }),
                }));
            }
            entries.sort_by(|a, b| {
                let a_dir = a.get("is_dir").and_then(|v| v.as_bool()).unwrap_or(false);
                let b_dir = b.get("is_dir").and_then(|v| v.as_bool()).unwrap_or(false);
                b_dir.cmp(&a_dir).then_with(|| {
                    a.get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .cmp(b.get("name").and_then(|v| v.as_str()).unwrap_or(""))
                })
            });
            serde_json::json!({"path": path.to_string_lossy(), "entries": entries})
        }

        // Browser (Chrome DevTools Protocol)
        _ => unreachable!("not a files action"),
    })
}
