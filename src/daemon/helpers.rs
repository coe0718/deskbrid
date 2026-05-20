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
pub fn expand_path(path: &str) -> anyhow::Result<PathBuf> {
    let expanded = if path.starts_with('~') {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
        PathBuf::from(path.replacen('~', &home, 1))
    } else {
        PathBuf::from(path)
    };
    Ok(expanded)
}

pub fn ensure_safe_pid(pid: u32) -> anyhow::Result<()> {
    if pid <= 1 {
        anyhow::bail!("refusing to target reserved pid {}", pid);
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

pub fn not_supported_response(msg: &str, seq: u64) -> serde_json::Value {
    serde_json::json!({
        "type": "response", "id": "?", "seq": seq, "status": "error",
        "error": { "code": "NOT_SUPPORTED", "message": msg }
    })
}

pub fn permission_denied_response(seq: u64) -> serde_json::Value {
    serde_json::json!({
        "type": "response", "id": "?", "seq": seq, "status": "error",
        "error": { "code": "PERMISSION_DENIED", "message": "action not permitted" }
    })
}
