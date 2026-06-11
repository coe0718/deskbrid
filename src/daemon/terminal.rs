use crate::DaemonState;
use crate::protocol::Action;
use serde_json::json;
use std::collections::VecDeque;
use std::fs::File;

use super::terminal_create::*;
use super::terminal_helpers::*;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::helpers::{ensure_safe_pid, parse_signal};

pub(crate) const DEFAULT_ROWS: u16 = 24;
pub(crate) const DEFAULT_COLS: u16 = 80;
pub(crate) const MAX_BUFFER_BYTES: usize = 1024 * 1024;
pub(crate) const DEFAULT_READ_BYTES: usize = 64 * 1024;

#[derive(Clone)]
pub struct TerminalSession {
    pub(crate) id: String,
    pub(crate) pid: u32,
    pub(crate) shell: String,
    pub(crate) cwd: Option<String>,
    pub(crate) rows: Arc<std::sync::Mutex<u16>>,
    pub(crate) cols: Arc<std::sync::Mutex<u16>>,
    pub(crate) created_at: u64,
    pub(crate) buffer: Arc<std::sync::Mutex<VecDeque<u8>>>,
    pub(crate) writer: Arc<std::sync::Mutex<File>>,
    pub(crate) closed: Arc<AtomicBool>,
}

impl TerminalSession {
    fn summary(&self) -> serde_json::Value {
        json!({
            "terminal_id": self.id,
            "pid": self.pid,
            "shell": self.shell,
            "cwd": self.cwd,
            "rows": *self.rows.lock().unwrap_or_else(|e| e.into_inner()),
            "cols": *self.cols.lock().unwrap_or_else(|e| e.into_inner()),
            "created_at": self.created_at,
            "closed": self.closed.load(Ordering::Relaxed),
            "buffered_bytes": self.buffer.lock().unwrap_or_else(|e| e.into_inner()).len(),
        })
    }
}

pub fn is_terminal_action(action: &Action) -> bool {
    matches!(
        action,
        Action::TerminalCreate { .. }
            | Action::TerminalWrite { .. }
            | Action::TerminalRead { .. }
            | Action::TerminalResize { .. }
            | Action::TerminalList
            | Action::TerminalKill { .. }
    )
}

pub async fn execute_terminal_action(
    action: Action,
    state: &DaemonState,
) -> anyhow::Result<serde_json::Value> {
    match action {
        Action::TerminalCreate {
            shell,
            cwd,
            env,
            rows,
            cols,
        } => create_terminal(state, shell, cwd, env, rows, cols).await,
        Action::TerminalWrite { terminal_id, input } => {
            write_terminal(state, &terminal_id, input).await
        }
        Action::TerminalRead {
            terminal_id,
            max_bytes,
            flush,
        } => read_terminal(state, &terminal_id, max_bytes, flush).await,
        Action::TerminalResize {
            terminal_id,
            rows,
            cols,
        } => resize_terminal(state, &terminal_id, rows, cols).await,
        Action::TerminalList => list_terminals(state).await,
        Action::TerminalKill {
            terminal_id,
            signal,
        } => kill_terminal(state, &terminal_id, signal.as_deref()).await,
        _ => anyhow::bail!(
            "internal dispatch error: non-terminal action passed to terminal dispatcher"
        ),
    }
}

async fn write_terminal(
    state: &DaemonState,
    terminal_id: &str,
    input: String,
) -> anyhow::Result<serde_json::Value> {
    const MAX_WRITE_BYTES: usize = 1_000_000; // 1MB cap per write
    if input.len() > MAX_WRITE_BYTES {
        anyhow::bail!(
            "terminal write input too large: {} bytes (max {})",
            input.len(),
            MAX_WRITE_BYTES
        );
    }
    let session = terminal_session(state, terminal_id).await?;
    if session.closed.load(Ordering::Relaxed) {
        anyhow::bail!("terminal is closed: {terminal_id}");
    }
    let bytes = input.into_bytes();
    let len = bytes.len();
    let writer = Arc::clone(&session.writer);
    tokio::task::spawn_blocking(move || {
        let mut writer = writer.lock().unwrap_or_else(|e| e.into_inner());
        writer.write_all(&bytes)?;
        writer.flush()
    })
    .await??;
    Ok(json!({"terminal_id": terminal_id, "bytes_written": len}))
}

async fn read_terminal(
    state: &DaemonState,
    terminal_id: &str,
    max_bytes: Option<u64>,
    flush: bool,
) -> anyhow::Result<serde_json::Value> {
    let session = terminal_session(state, terminal_id).await?;
    let max_bytes = max_bytes
        .unwrap_or(DEFAULT_READ_BYTES as u64)
        .min(MAX_BUFFER_BYTES as u64) as usize;
    let mut buffer = session.buffer.lock().unwrap_or_else(|e| e.into_inner());
    let take = buffer.len().min(max_bytes);
    let bytes: Vec<u8> = if flush {
        buffer.drain(..take).collect()
    } else {
        buffer.iter().take(take).copied().collect()
    };
    let remaining_bytes = buffer.len();
    drop(buffer);
    let output = String::from_utf8_lossy(&bytes).to_string();
    Ok(json!({
        "terminal_id": terminal_id,
        "output": output,
        "bytes": bytes.len(),
        "remaining_bytes": remaining_bytes,
        "closed": session.closed.load(Ordering::Relaxed),
    }))
}

async fn resize_terminal(
    state: &DaemonState,
    terminal_id: &str,
    rows: u16,
    cols: u16,
) -> anyhow::Result<serde_json::Value> {
    if rows == 0 || cols == 0 {
        anyhow::bail!("rows and cols must be positive");
    }
    let session = terminal_session(state, terminal_id).await?;
    let writer = session.writer.lock().unwrap_or_else(|e| e.into_inner());
    let mut winsize = libc::winsize {
        ws_row: rows,
        ws_col: cols,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let rc = unsafe { libc::ioctl(writer.as_raw_fd(), libc::TIOCSWINSZ, &mut winsize) };
    drop(writer);
    if rc != 0 {
        anyhow::bail!("failed to resize pty: {}", std::io::Error::last_os_error());
    }
    *session.rows.lock().unwrap_or_else(|e| e.into_inner()) = rows;
    *session.cols.lock().unwrap_or_else(|e| e.into_inner()) = cols;
    signal_process_group_or_pid(session.pid, libc::SIGWINCH)?;
    Ok(json!({"terminal_id": terminal_id, "rows": rows, "cols": cols}))
}

async fn list_terminals(state: &DaemonState) -> anyhow::Result<serde_json::Value> {
    let sessions: Vec<_> = state
        .terminals
        .lock()
        .await
        .values()
        .map(TerminalSession::summary)
        .collect();
    Ok(json!({"terminals": sessions}))
}

async fn kill_terminal(
    state: &DaemonState,
    terminal_id: &str,
    signal: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    let session = {
        let mut terminals = state.terminals.lock().await;
        terminals
            .remove(terminal_id)
            .ok_or_else(|| anyhow::anyhow!("terminal not found: {terminal_id}"))?
    };
    ensure_safe_pid(session.pid)?;
    let sig = parse_signal(signal.unwrap_or("HUP"))?;
    signal_process_group_or_pid(session.pid, sig)?;
    session.closed.store(true, Ordering::Relaxed);
    Ok(json!({"terminal_id": terminal_id, "killed": true, "signal": sig}))
}
