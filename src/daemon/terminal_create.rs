use crate::DaemonState;
use anyhow::Context;
use serde_json::json;
use std::collections::VecDeque;
use std::fs::File;
use std::os::fd::FromRawFd;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::helpers::expand_path;
use super::terminal::{DEFAULT_COLS, DEFAULT_ROWS, TerminalSession};
use super::terminal_helpers::*;

pub(crate) async fn create_terminal(
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
