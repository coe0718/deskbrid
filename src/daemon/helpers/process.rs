//! Process utilities: find windows, spawn detached processes, validate PIDs, parse signals.

use std::collections::HashMap;
use std::process::Stdio;

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
    // If DESKBRID_ALLOWED_COMMANDS is unset, any command is allowed — set it to restrict.
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
