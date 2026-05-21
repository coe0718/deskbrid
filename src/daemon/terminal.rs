use crate::DaemonState;
use crate::protocol::Action;
use anyhow::Context;
use serde_json::json;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use super::helpers::{ensure_safe_pid, expand_path, parse_signal};

const DEFAULT_ROWS: u16 = 24;
const DEFAULT_COLS: u16 = 80;
const MAX_BUFFER_BYTES: usize = 1024 * 1024;
const DEFAULT_READ_BYTES: usize = 64 * 1024;

#[derive(Clone)]
pub struct TerminalSession {
    id: String,
    pid: u32,
    shell: String,
    cwd: Option<String>,
    rows: Arc<std::sync::Mutex<u16>>,
    cols: Arc<std::sync::Mutex<u16>>,
    created_at: u64,
    buffer: Arc<std::sync::Mutex<VecDeque<u8>>>,
    writer: Arc<std::sync::Mutex<File>>,
    closed: Arc<AtomicBool>,
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
        _ => unreachable!("non-terminal action passed to terminal dispatcher"),
    }
}

async fn create_terminal(
    state: &DaemonState,
    shell: Option<String>,
    cwd: Option<String>,
    env: Option<std::collections::HashMap<String, String>>,
    rows: Option<u16>,
    cols: Option<u16>,
) -> anyhow::Result<serde_json::Value> {
    let rows = rows.unwrap_or(DEFAULT_ROWS).max(1);
    let cols = cols.unwrap_or(DEFAULT_COLS).max(1);
    let shell = shell
        .filter(|s| !s.trim().is_empty())
        .or_else(|| std::env::var("SHELL").ok())
        .unwrap_or_else(|| "/bin/bash".to_string());
    let cwd_path = cwd.as_deref().map(expand_path).transpose()?;

    let mut master_fd = -1;
    let mut slave_fd = -1;
    let winsize = libc::winsize {
        ws_row: rows,
        ws_col: cols,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let openpty_rc = unsafe {
        libc::openpty(
            &mut master_fd,
            &mut slave_fd,
            std::ptr::null_mut(),
            std::ptr::null(),
            &winsize,
        )
    };
    if openpty_rc != 0 {
        anyhow::bail!(
            "failed to allocate pty: {}",
            std::io::Error::last_os_error()
        );
    }

    let master = unsafe { File::from_raw_fd(master_fd) };
    let reader = master.try_clone().context("failed to clone pty master")?;
    let writer = master;

    let stdin_fd = dup_fd(slave_fd, "stdin")?;
    let stdout_fd = dup_fd(slave_fd, "stdout")?;
    let stderr_fd = dup_fd(slave_fd, "stderr")?;

    let mut command = Command::new(&shell);
    command
        .stdin(unsafe { Stdio::from_raw_fd(stdin_fd) })
        .stdout(unsafe { Stdio::from_raw_fd(stdout_fd) })
        .stderr(unsafe { Stdio::from_raw_fd(stderr_fd) })
        .env("TERM", "xterm-256color");
    if let Some(cwd_path) = &cwd_path {
        command.current_dir(cwd_path);
    }
    if let Some(env) = &env {
        command.envs(env);
    }

    unsafe {
        command.pre_exec(move || {
            if libc::setsid() < 0 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::ioctl(slave_fd, libc::TIOCSCTTY, 0) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }

    let mut child = command
        .spawn()
        .with_context(|| format!("failed to spawn terminal shell '{}'", shell))?;
    unsafe {
        libc::close(slave_fd);
    }

    let pid = child.id();
    let id = state.next_terminal_id();
    let buffer = Arc::new(std::sync::Mutex::new(VecDeque::new()));
    let closed = Arc::new(AtomicBool::new(false));
    let session = TerminalSession {
        id: id.clone(),
        pid,
        shell: shell.clone(),
        cwd: cwd_path.map(|p| p.to_string_lossy().to_string()),
        rows: Arc::new(std::sync::Mutex::new(rows)),
        cols: Arc::new(std::sync::Mutex::new(cols)),
        created_at: unix_now(),
        buffer: Arc::clone(&buffer),
        writer: Arc::new(std::sync::Mutex::new(writer)),
        closed: Arc::clone(&closed),
    };

    spawn_reader(id.clone(), reader, Arc::clone(&buffer), Arc::clone(&closed));
    std::thread::spawn({
        let closed = Arc::clone(&closed);
        move || {
            let _ = child.wait();
            closed.store(true, Ordering::Relaxed);
        }
    });

    state
        .terminals
        .lock()
        .await
        .insert(id.clone(), session.clone());

    Ok(json!({
        "terminal_id": id,
        "pid": pid,
        "shell": shell,
        "rows": rows,
        "cols": cols,
        "created_at": session.created_at
    }))
}

async fn write_terminal(
    state: &DaemonState,
    terminal_id: &str,
    input: String,
) -> anyhow::Result<serde_json::Value> {
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

async fn terminal_session(
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

fn dup_fd(fd: i32, label: &str) -> anyhow::Result<i32> {
    let duped = unsafe { libc::dup(fd) };
    if duped < 0 {
        anyhow::bail!(
            "failed to duplicate pty fd for {label}: {}",
            std::io::Error::last_os_error()
        );
    }
    Ok(duped)
}

fn spawn_reader(
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

fn signal_process_group_or_pid(pid: u32, sig: i32) -> anyhow::Result<()> {
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

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
