use crate::DaemonState;
use std::collections::VecDeque;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use super::helpers::ensure_safe_pid;

use super::terminal::{MAX_BUFFER_BYTES, TerminalSession};

pub(crate) async fn terminal_session(
    state: &DaemonState,
    terminal_id: &str,
) -> anyhow::Result<TerminalSession> {
    if terminal_id.trim().is_empty() {
        anyhow::bail!("terminal_id must not be empty");
    }
    state
        .terminals
        .lock()
        .await
        .get(terminal_id)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("terminal not found: {terminal_id}"))
}

pub(crate) fn dup_fd(fd: i32, label: &str) -> anyhow::Result<i32> {
    let duped = unsafe { libc::dup(fd) };
    if duped < 0 {
        anyhow::bail!(
            "failed to duplicate pty fd for {label}: {}",
            std::io::Error::last_os_error()
        );
    }
    Ok(duped)
}

pub(crate) fn spawn_reader(
    _id: String,
    mut reader: File,
    buffer: Arc<std::sync::Mutex<VecDeque<u8>>>,
    closed: Arc<AtomicBool>,
) {
    std::thread::spawn(move || {
        let mut chunk = [0u8; 8192];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) => {
                    closed.store(true, Ordering::Relaxed);
                    break;
                }
                Ok(n) => {
                    let mut buffer = buffer.lock().unwrap_or_else(|e| e.into_inner());
                    buffer.extend(&chunk[..n]);
                    while buffer.len() > MAX_BUFFER_BYTES {
                        buffer.pop_front();
                    }
                }
                Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    closed.store(true, Ordering::Relaxed);
                    break;
                }
            }
        }
    });
}

pub(crate) fn signal_process_group_or_pid(pid: u32, sig: i32) -> anyhow::Result<()> {
    ensure_safe_pid(pid)?;
    let group_rc = unsafe { libc::kill(-(pid as i32), sig) };
    if group_rc == 0 {
        return Ok(());
    }
    let pid_rc = unsafe { libc::kill(pid as i32, sig) };
    if pid_rc != 0 {
        anyhow::bail!(
            "failed to signal terminal pid {pid}: {}",
            std::io::Error::last_os_error()
        );
    }
    Ok(())
}

pub(crate) fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
