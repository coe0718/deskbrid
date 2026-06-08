//! Path utilities: home directory resolution, path expansion with sandboxing, screenshot temp paths.

use std::path::PathBuf;

pub fn home_dir() -> PathBuf {
    dirs::home_dir()
        .or_else(|| std::env::var("HOME").ok().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("/root"))
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

#[cfg(test)]
mod tests {
    use super::*;
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
        let home = home_dir().to_string_lossy().to_string();
        let traversal = format!("{}/../../../etc/passwd", home);
        let result = expand_path(&traversal);
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
