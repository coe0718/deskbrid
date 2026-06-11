use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::Action;
use serde_json::Value;

use super::{ensure_safe_pid, parse_signal, spawn_detached_process};

pub(crate) async fn execute_process(
    action: Action,
    _backend: &dyn DesktopBackend,
    _state: &DaemonState,
) -> anyhow::Result<Value> {
    use Action::*;
    Ok(match action {
        ProcessList => {
            let output = tokio::process::Command::new("ps")
                .args(["aux", "--no-headers"])
                .output()
                .await?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            let processes: Vec<serde_json::Value> = stdout
                .lines()
                .take(200)
                .filter_map(|line| {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() < 11 {
                        return None;
                    }
                    Some(serde_json::json!({
                        "user": parts[0],
                        "pid": parts[1].parse::<u32>().unwrap_or(0),
                        "cpu": parts[2],
                        "mem": parts[3],
                        "command": parts[10..].join(" ")
                    }))
                })
                .collect();
            serde_json::json!({"processes": processes})
        }
        ProcessStart {
            ref command,
            ref workdir,
            ref env,
        } => {
            let pid = spawn_detached_process(command, workdir.as_deref(), env.as_ref()).await?;
            serde_json::json!({"pid": pid, "command": command})
        }
        ProcessStop { pid, ref signal } => {
            ensure_safe_pid(pid)?;
            let sig = parse_signal(signal.as_deref().unwrap_or("TERM"))?;
            let rc = unsafe { libc::kill(pid as i32, sig) };
            if rc != 0 {
                let err = std::io::Error::last_os_error();
                anyhow::bail!("failed to stop pid {}: {}", pid, err);
            }
            serde_json::json!({"stopped": pid, "signal": sig})
        }
        ProcessSignal { pid, ref signal } => {
            ensure_safe_pid(pid)?;
            let sig = parse_signal(signal)?;
            let rc = unsafe { libc::kill(pid as i32, sig) };
            if rc != 0 {
                let err = std::io::Error::last_os_error();
                anyhow::bail!("failed to signal pid {}: {}", pid, err);
            }
            serde_json::json!({"signaled": pid, "signal": sig})
        }
        ProcessExists { pid } => {
            ensure_safe_pid(pid)?;
            let rc = unsafe { libc::kill(pid as i32, 0) };
            if rc == 0 {
                serde_json::json!({"pid": pid, "exists": true})
            } else {
                let errno = std::io::Error::last_os_error()
                    .raw_os_error()
                    .unwrap_or_default();
                if errno == libc::ESRCH {
                    serde_json::json!({"pid": pid, "exists": false})
                } else {
                    anyhow::bail!(
                        "failed to check pid {}: {}",
                        pid,
                        std::io::Error::last_os_error()
                    )
                }
            }
        }
        ProcessWait { pid, timeout_ms } => {
            ensure_safe_pid(pid)?;
            let timeout = std::time::Duration::from_millis(timeout_ms.unwrap_or(30_000));
            let started = std::time::Instant::now();
            let mut poll_interval = std::time::Duration::from_millis(50);
            const MAX_POLL: std::time::Duration = std::time::Duration::from_millis(500);
            loop {
                let rc = unsafe { libc::kill(pid as i32, 0) };
                if rc != 0 {
                    let errno = std::io::Error::last_os_error()
                        .raw_os_error()
                        .unwrap_or_default();
                    if errno == libc::ESRCH {
                        break;
                    }
                    anyhow::bail!(
                        "failed to wait on pid {}: {}",
                        pid,
                        std::io::Error::last_os_error()
                    );
                }
                if started.elapsed() >= timeout {
                    return Ok(
                        serde_json::json!({"pid": pid, "exited": false, "timeout_ms": timeout.as_millis()}),
                    );
                }
                tokio::time::sleep(poll_interval).await;
                poll_interval = (poll_interval * 2).min(MAX_POLL);
            }
            serde_json::json!({"pid": pid, "exited": true, "elapsed_ms": started.elapsed().as_millis()})
        }

        _ => anyhow::bail!("internal dispatch error: not a process action"),
    })
}
