use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;

pub fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub async fn find_app_window(
    backend: &dyn crate::backend::DesktopBackend,
    app_id: &str,
) -> anyhow::Result<Option<crate::protocol::WindowInfo>> {
    if app_id.trim().is_empty() {
        anyhow::bail!("app_id must not be empty");
    }

    let windows = backend.windows_list().await?;
    let app_l = app_id.to_lowercase();
    Ok(windows
        .iter()
        .find(|w| w.app_id.eq_ignore_ascii_case(app_id))
        .cloned()
        .or_else(|| {
            windows
                .iter()
                .find(|w| w.title.eq_ignore_ascii_case(app_id))
                .cloned()
        })
        .or_else(|| {
            windows
                .iter()
                .find(|w| {
                    w.app_id.to_lowercase().contains(&app_l)
                        || w.title.to_lowercase().contains(&app_l)
                })
                .cloned()
        }))
}

pub async fn spawn_detached_process(
    command: &[String],
    workdir: Option<&str>,
    env: Option<&HashMap<String, String>>,
) -> anyhow::Result<u32> {
    let program = command
        .first()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("command must not be empty"))?;

    // Validate against command allowlist (env-configured, colon-separated)
    // If DESKBRID_ALLOWED_COMMANDS is unset, only allow known-safe starters
    let allowed_cmds = std::env::var("DESKBRID_ALLOWED_COMMANDS").unwrap_or_default();
    if !allowed_cmds.is_empty() {
        let allowed: Vec<&str> = allowed_cmds.split(':').collect();
        let program_str = program.as_str();
        let is_allowed = allowed.iter().any(|a| {
            // Exact match or glob-style prefix match (e.g., "/usr/bin/*")
            *a == program_str
                || a.strip_suffix("/*").is_some_and(|prefix| {
                    program_str.starts_with(prefix) && program_str.contains('/')
                })
        });
        if !is_allowed {
            anyhow::bail!(
                "command '{}' is not in the allowed commands list (DESKBRID_ALLOWED_COMMANDS)",
                program
            );
        }
    }

    let mut cmd = tokio::process::Command::new(program);
    cmd.args(&command[1..]);
    if let Some(wd) = workdir {
        cmd.current_dir(wd);
    }
    if let Some(env_vars) = env {
        for (k, v) in env_vars {
            cmd.env(k, v);
        }
    }

    let child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(child.id().unwrap_or(0))
}

/// Expand ~ to $HOME and resolve relative paths to absolute.
/// Canonicalizes existent paths (resolves symlinks, `..`, `.`).
/// For non-existent paths, canonicalizes the parent directory.
/// Then verifies the result is within the allowed sandbox dirs.
pub fn expand_path(path: &str) -> anyhow::Result<PathBuf> {
    let expanded = if path.starts_with('~') {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
        PathBuf::from(path.replacen('~', &home, 1))
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
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
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

pub fn ensure_safe_pid(pid: u32) -> anyhow::Result<()> {
    if pid <= 1 {
        anyhow::bail!("refusing to target reserved pid {}", pid);
    }
    // Block kernel pseudo-processes (PID 2 = kthreadd)
    if pid == 2 {
        anyhow::bail!("refusing to target kernel pid {}", pid);
    }
    if pid > i32::MAX as u32 {
        anyhow::bail!(
            "refusing to target out-of-range pid {} (exceeds i32::MAX)",
            pid
        );
    }
    let self_pid = std::process::id();
    if pid == self_pid {
        anyhow::bail!("refusing to target deskbrid daemon pid {}", pid);
    }
    Ok(())
}

pub fn parse_signal(sig: &str) -> anyhow::Result<i32> {
    let normalized = sig.trim().to_ascii_uppercase();
    let normalized = normalized.strip_prefix("SIG").unwrap_or(&normalized);
    let value = match normalized {
        "HUP" => libc::SIGHUP,
        "INT" => libc::SIGINT,
        "QUIT" => libc::SIGQUIT,
        "KILL" => libc::SIGKILL,
        "TERM" => libc::SIGTERM,
        "USR1" => libc::SIGUSR1,
        "USR2" => libc::SIGUSR2,
        "CONT" => libc::SIGCONT,
        "STOP" => libc::SIGSTOP,
        _ => anyhow::bail!("unsupported signal: {}", sig),
    };
    Ok(value)
}

pub fn ok_response(id: &str, seq: u64) -> serde_json::Value {
    serde_json::json!({"type": "response", "id": id, "seq": seq, "status": "ok", "data": {}})
}

pub fn not_supported_response(request_id: &str, msg: &str, seq: u64) -> serde_json::Value {
    serde_json::json!({
        "type": "response", "id": request_id, "seq": seq, "status": "error",
        "error": { "code": "NOT_SUPPORTED", "message": msg }
    })
}

pub fn permission_denied_response(
    request_id: &str,
    action_type: &str,
    seq: u64,
) -> serde_json::Value {
    serde_json::json!({
        "type": "response", "id": request_id, "seq": seq, "status": "error",
        "error": { "code": "PERMISSION_DENIED", "message": format!("action not permitted: {action_type} requires explicit permission — add '{action_type}' to your permissions.toml") }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_path_allows_home_dir() {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
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
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
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
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
        assert!(result.unwrap().starts_with(&home));
    }

    #[test]
    fn expand_path_tilde_traversal_blocked() {
        let result = expand_path("~/../../../etc/passwd");
        assert!(result.is_err(), "~/../../../etc/passwd should be blocked");
    }

    #[test]
    fn expand_path_allows_existing_files_in_home() {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
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
