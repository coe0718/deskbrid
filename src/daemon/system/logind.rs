use crate::DaemonState;
use anyhow::Context;
use serde_json::json;
use std::process::Stdio;
use tokio::process::Command;

use super::command::{ensure_arg, run};

pub async fn inhibit(
    state: &DaemonState,
    what: &str,
    who: &str,
    why: Option<&str>,
    mode: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    ensure_arg(what, "what")?;
    ensure_arg(who, "who")?;
    let why = why.unwrap_or("Deskbrid task in progress");
    let mode = mode.unwrap_or("block");
    ensure_arg(why, "why")?;
    ensure_arg(mode, "mode")?;

    let mut child = Command::new("systemd-inhibit")
        .args([
            format!("--what={what}"),
            format!("--who={who}"),
            format!("--why={why}"),
            format!("--mode={mode}"),
            "sleep".to_string(),
            "infinity".to_string(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .context("failed to start systemd-inhibit")?;

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    if let Some(status) = child.try_wait()? {
        anyhow::bail!("systemd-inhibit exited immediately with {status}");
    }

    let inhibitor_id = state.next_inhibitor_id();
    state.inhibitors.insert(inhibitor_id, child);
    Ok(json!({ "inhibitor_id": inhibitor_id, "what": what, "who": who, "why": why, "mode": mode }))
}

pub async fn release_inhibit(
    state: &DaemonState,
    inhibitor_id: u32,
) -> anyhow::Result<serde_json::Value> {
    let Some((_, mut child)) = state.inhibitors.remove(&inhibitor_id) else {
        anyhow::bail!("inhibitor not found: {inhibitor_id}");
    };
    child.start_kill()?;
    let _ = child.wait().await;
    Ok(json!({ "released": inhibitor_id }))
}

pub async fn list_sessions() -> anyhow::Result<serde_json::Value> {
    let out = run("loginctl", &["list-sessions", "--no-legend", "--no-pager"]).await?;
    let sessions: Vec<_> = out
        .lines()
        .filter_map(|line| {
            let cols: Vec<&str> = line.split_whitespace().collect();
            Some(json!({
                "id": *cols.first()?,
                "uid": cols.get(1).copied().unwrap_or(""),
                "user": cols.get(2).copied().unwrap_or(""),
                "seat": cols.get(3).copied().unwrap_or(""),
                "tty": cols.get(4).copied().unwrap_or(""),
                "raw": line
            }))
        })
        .collect();
    Ok(json!({ "sessions": sessions }))
}

pub async fn lock_session(session_id: Option<&str>) -> anyhow::Result<serde_json::Value> {
    let mut args = vec!["lock-session"];
    if let Some(id) = session_id {
        ensure_arg(id, "session_id")?;
        args.push(id);
    }
    run("loginctl", &args).await?;
    Ok(json!({ "locked": session_id }))
}

pub async fn switch_user(username: &str) -> anyhow::Result<serde_json::Value> {
    ensure_arg(username, "username")?;
    run("dm-tool", &["switch-to-user", username])
        .await
        .context(
            "failed to switch user; install dm-tool or use a display manager that supports it",
        )?;
    Ok(json!({ "switched_user": username }))
}
