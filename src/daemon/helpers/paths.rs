//! Path utilities: home directory resolution, path expansion with sandboxing, screenshot temp paths.

use std::path::PathBuf;

/// Resolve the user's home directory.
///
/// W13 (docs/CODE_REVIEW_VEX.md): prefers `dirs::home_dir()` (which reads
/// `/etc/passwd` + `HOME`), then `HOME`, then logs a warning and falls
/// back to `/tmp` instead of `/root` so a misconfigured systemd unit
/// doesn't silently write private data to root's home.
pub fn home_dir() -> PathBuf {
    if let Some(dir) = dirs::home_dir() {
        return dir;
    }
    if let Some(home) = std::env::var_os("HOME") {
        let path = PathBuf::from(home);
        if !path.as_os_str().is_empty() {
            return path;
        }
    }
    tracing::warn!(
        "could not resolve user home directory (HOME unset, no entry in /etc/passwd); \
         falling back to /tmp — deskbrid may not function correctly"
    );
    PathBuf::from("/tmp")
}

pub fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Expand ~ to $HOME and resolve relative paths to absolute.
/// Canonicalizes existent paths (resolves symlinks, `..`, `.`).
/// For non-existent paths, canonicalizes the parent directory.
/// Then verifies the result is within the allowed sandbox dirs.
pub fn expand_path(path: &str) -> anyhow::Result<PathBuf> {
    let expanded = if path.starts_with('~') {
        let home = home_dir();
        PathBuf::from(path.replacen('~', &home.to_string_lossy(), 1))
    } else {
        PathBuf::from(path)
    };

    // Canonicalize to resolve symlinks and `../` traversal
    let canonical = match std::fs::canonicalize(&expanded) {
        Ok(p) => p,
        Err(_) => {
            // Path doesn't exist yet — canonicalize parent instead
            if let Some(parent) = expanded.parent() {
                let canon_parent = std::fs::canonicalize(parent)
                    .map_err(|_| anyhow::anyhow!("invalid path: {path}"))?;
                let file_name = expanded
                    .file_name()
                    .ok_or_else(|| anyhow::anyhow!("invalid path: {path}"))?;
                canon_parent.join(file_name)
            } else {
                anyhow::bail!("invalid path: {path}");
            }
        }
    };

    // Sandbox check: verify path is within allowed directories
    let allowed_dirs = std::env::var("DESKBRID_ALLOWED_DIRS").unwrap_or_else(|_| {
        let home = home_dir().to_string_lossy().to_string();
        format!("{}:/tmp", home)
    });
    let allowed: Vec<PathBuf> = allowed_dirs
        .split(':')
        .map(|d| {
            let p = PathBuf::from(d);
            // Canonicalize each allowed dir for comparison
            std::fs::canonicalize(&p).unwrap_or(p)
        })
        .collect();

    let is_allowed = allowed.iter().any(|dir| canonical.starts_with(dir));
    if !is_allowed {
        anyhow::bail!(
            "access denied: path {} is outside allowed directories",
            canonical.display()
        );
    }

    Ok(canonical)
}

/// Generate a safe temporary path for screenshots.
/// Uses XDG_RUNTIME_DIR or ~/.cache/deskbrid with UUID filenames
/// instead of predictable /tmp paths with world-readable permissions.
pub fn screenshot_temp_path() -> String {
    let dir = std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = home_dir().to_string_lossy().to_string();
            PathBuf::from(home).join(".cache")
        })
        .join("deskbrid");
    let _ = std::fs::create_dir_all(&dir);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
    }
    dir.join(format!("screenshot_{}.png", uuid::Uuid::new_v4()))
        .to_string_lossy()
        .to_string()
}

/// W17 (Vex review): delete screenshot temp files older than
/// `max_age_secs`. Default 1 hour — gives clients plenty of time to
/// download, while keeping the runtime directory bounded on long-running
/// daemons. Returns the number of files removed.
///
/// Called by `spawn_screenshot_cleaner` periodically.
pub fn cleanup_stale_screenshots(max_age_secs: u64) -> usize {
    let dir = std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = home_dir().to_string_lossy().to_string();
            PathBuf::from(home).join(".cache")
        })
        .join("deskbrid");

    if !dir.is_dir() {
        return 0;
    }

    let now = std::time::SystemTime::now();
    let max_age = std::time::Duration::from_secs(max_age_secs);
    let mut removed = 0usize;

    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return 0,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        // Only clean up files we created (prefix "screenshot_" or "diff_").
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let is_ours = name.starts_with("screenshot_") || name.starts_with("diff_");
        if !is_ours {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let Ok(mtime) = meta.modified() else { continue };
        if now.duration_since(mtime).unwrap_or_default() > max_age
            && std::fs::remove_file(&path).is_ok()
        {
            removed += 1;
        }
    }
    removed
}

/// W17: spawn a background task that calls `cleanup_stale_screenshots`
/// every `interval_secs` (default 5 min). The handle is stored in
/// `DaemonState.background_tasks` so graceful shutdown aborts it.
pub fn spawn_screenshot_cleaner(state: std::sync::Arc<crate::DaemonState>) {
    tokio::spawn(async move {
        let interval_secs: u64 = std::env::var("DESKBRID_SCREENSHOT_CLEANUP_INTERVAL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(300);
        let max_age_secs: u64 = std::env::var("DESKBRID_SCREENSHOT_TTL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3600);
        loop {
            if state.shutdown.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            let removed = cleanup_stale_screenshots(max_age_secs);
            if removed > 0 {
                tracing::debug!("screenshot cleaner: removed {removed} stale file(s)");
            }
            tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_path_allows_home_dir() {
        let home = home_dir().to_string_lossy().to_string();
        let result = expand_path(&format!("{}/.bashrc", home));
        assert!(
            result.is_ok(),
            "path in HOME should be allowed: {:?}",
            result.err()
        );
    }

    #[test]
    fn expand_path_allows_tmp() {
        let result = expand_path("/tmp/test-file");
        assert!(result.is_ok(), "/tmp should be allowed: {:?}", result.err());
    }

    #[test]
    fn expand_path_blocks_etc_passwd() {
        let result = expand_path("/etc/passwd");
        assert!(result.is_err(), "/etc/passwd should be blocked");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("outside allowed directories"),
            "error should mention sandbox, got: {msg}"
        );
    }

    #[test]
    fn expand_path_blocks_traversal_into_etc() {
        let traversal = "/tmp/../../../etc/passwd";
        let result = expand_path(traversal);
        assert!(
            result.is_err(),
            "../../../etc/passwd traversal should be blocked"
        );
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("outside allowed directories"),
            "traversal should hit sandbox, got: {msg}"
        );
    }

    #[test]
    fn expand_path_blocks_traversal_into_root() {
        let traversal = "/tmp/../../../etc/shadow";
        let result = expand_path(traversal);
        assert!(
            result.is_err(),
            "/etc/shadow via traversal should be blocked"
        );
    }

    #[test]
    fn expand_path_tilde_expands_to_home() {
        let result = expand_path("~/.bashrc");
        assert!(
            result.is_ok(),
            "~ expansion should work: {:?}",
            result.err()
        );
        let home = home_dir().to_string_lossy().to_string();
        assert!(result.unwrap().starts_with(&home));
    }

    #[test]
    fn expand_path_tilde_traversal_blocked() {
        let result = expand_path("~/../../../etc/passwd");
        assert!(result.is_err(), "~/../../../etc/passwd should be blocked");
    }

    #[test]
    fn expand_path_allows_existing_files_in_home() {
        let home = home_dir().to_string_lossy().to_string();
        // .bashrc typically exists
        let path = format!("{}/.bashrc", home);
        if std::path::Path::new(&path).exists() {
            let result = expand_path(&path);
            assert!(
                result.is_ok(),
                "existing .bashrc should be allowed: {:?}",
                result.err()
            );
        }
    }
}
