use crate::protocol::{MacroSummary, RecordedAction};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{debug, info, warn};

use crate::DaemonState;

// ─── Recording State ──────────────────────────────────

/// An in-progress macro recording — stored in DaemonState while active.
#[derive(Debug)]
pub struct ActiveRecording {
    pub name: String,
    pub description: Option<String>,
    pub started_at: u64,
    pub actions: Vec<RecordedAction>,
    /// Tracks elapsed time from start for each recorded action
    last_recorded_at: u64,
}

impl ActiveRecording {
    pub fn new(name: String, description: Option<String>) -> Self {
        let now = unix_ms();
        Self {
            name,
            description,
            started_at: now,
            actions: Vec::new(),
            last_recorded_at: now,
        }
    }

    /// Record one action. Called from the dispatch layer.
    pub fn push(&mut self, action_type: &str, params: serde_json::Value) {
        let now = unix_ms();
        let seq = self.actions.len() as u64;
        self.actions.push(RecordedAction {
            seq,
            timestamp: now,
            elapsed_ms: now - self.last_recorded_at,
            action_type: action_type.to_string(),
            params,
        });
        self.last_recorded_at = now;
    }
}

// ─── Macro File (Disk Format) ─────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct MacroFile {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_at: u64,
    pub actions: Vec<RecordedAction>,
}

fn macro_dir() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
            PathBuf::from(home).join(".local").join("share")
        });
    base.join("deskbrid").join("macros")
}

fn macro_path(name: &str) -> PathBuf {
    macro_dir().join(format!("{}.json", sanitize_name(name)))
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

// ─── Recording API ────────────────────────────────────

/// Start recording a new macro. Returns the recording state.
pub fn start_recording(
    state: &DaemonState,
    name: String,
    description: Option<String>,
) -> anyhow::Result<()> {
    let mut guard = state
        .recording
        .try_lock()
        .map_err(|_| anyhow::anyhow!("recording lock contended — try again"))?;
    if guard.is_some() {
        anyhow::bail!("already recording — stop current recording first");
    }
    info!("Started recording macro '{}'", name);
    *guard = Some(ActiveRecording::new(name, description));
    Ok(())
}

/// Stop recording and save the macro to disk. Returns the summary.
pub fn stop_recording(state: &DaemonState) -> anyhow::Result<MacroSummary> {
    let mut guard = state
        .recording
        .try_lock()
        .map_err(|_| anyhow::anyhow!("recording lock contended — try again"))?;
    let recording = guard
        .take()
        .ok_or_else(|| anyhow::anyhow!("no recording in progress"))?;

    let total_duration_ms: u64 = recording.actions.iter().map(|a| a.elapsed_ms).sum();
    let action_count = recording.actions.len();

    let macro_file = MacroFile {
        name: recording.name.clone(),
        description: recording.description.clone(),
        created_at: recording.started_at,
        actions: recording.actions,
    };

    save_macro(&macro_file)?;
    info!(
        "Stopped recording '{}': {} actions, {}ms total",
        recording.name, action_count, total_duration_ms
    );

    Ok(MacroSummary {
        name: recording.name,
        description: recording.description,
        action_count,
        total_duration_ms,
        created_at: recording.started_at,
    })
}

/// Push an action into the active recording (if any).
pub fn record_action(state: &DaemonState, action_type: &str, params: serde_json::Value) {
    if let Ok(mut guard) = state.recording.try_lock()
        && let Some(ref mut rec) = *guard
    {
        rec.push(action_type, params);
    }
}

// ─── Replay ───────────────────────────────────────────

/// Replay a saved macro. Executes actions through the normal dispatch pipeline.
pub async fn replay_macro(
    state: &DaemonState,
    name: &str,
    mode: &str,
    loop_count: u32,
    stop_on_error: bool,
    peer_uid: u32,
) -> anyhow::Result<Vec<serde_json::Value>> {
    let macro_file = load_macro(name)?;
    let mut results = Vec::new();

    for _ in 0..loop_count {
        for action in &macro_file.actions {
            debug!(
                "Replaying macro action seq={} type={}",
                action.seq, action.action_type
            );

            // Wait for timed mode
            if mode == "timed" && action.elapsed_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(
                    action.elapsed_ms.min(30_000), // cap at 30s
                ))
                .await;
            }

            // Parse and dispatch the action
            let line = serde_json::to_string(&action.params)?;
            match crate::protocol::Action::from_json_with_options(&line) {
                Ok((_id, replay_action, _opts)) => {
                    let result = Box::pin(crate::daemon::dispatch::dispatch_action_with_options(
                        "",
                        replay_action,
                        state,
                        peer_uid,
                        action.seq,
                        crate::protocol::RequestOptions::default(),
                        "default",
                    ))
                    .await;
                    results.push(result.clone());
                    if stop_on_error
                        && let Some(status) = result.get("status").and_then(|s| s.as_str())
                        && status != "ok"
                    {
                        warn!(
                            "Macro replay stopped on error at seq={}: {:?}",
                            action.seq, result
                        );
                        return Ok(results);
                    }
                }
                Err(e) => {
                    warn!("Failed to parse recorded action: {}", e);
                    if stop_on_error {
                        return Ok(results);
                    }
                }
            }
        }
    }

    Ok(results)
}

// ─── CRUD ─────────────────────────────────────────────

pub fn list_macros() -> anyhow::Result<Vec<MacroSummary>> {
    let dir = macro_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut summaries = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        match load_macro_file(&path) {
            Ok(mf) => {
                let total_duration_ms: u64 = mf.actions.iter().map(|a| a.elapsed_ms).sum();
                summaries.push(MacroSummary {
                    name: mf.name.clone(),
                    description: mf.description.clone(),
                    action_count: mf.actions.len(),
                    total_duration_ms,
                    created_at: mf.created_at,
                });
            }
            Err(e) => {
                warn!("Failed to load macro file {}: {}", path.display(), e);
            }
        }
    }
    summaries.sort_by_key(|s| s.name.clone());
    Ok(summaries)
}

pub fn get_macro(name: &str) -> anyhow::Result<MacroFile> {
    load_macro(name)
}

pub fn delete_macro(name: &str) -> anyhow::Result<()> {
    let path = macro_path(name);
    if path.exists() {
        std::fs::remove_file(&path)?;
        info!("Deleted macro '{}'", name);
    }
    Ok(())
}

pub fn export_macro(name: &str) -> anyhow::Result<String> {
    let mf = load_macro(name)?;
    serde_json::to_string_pretty(&mf).map_err(Into::into)
}

pub fn import_macro(name: &str, data: &str) -> anyhow::Result<MacroSummary> {
    let mf: MacroFile = serde_json::from_str(data)?;
    let total_duration_ms: u64 = mf.actions.iter().map(|a| a.elapsed_ms).sum();
    let summary = MacroSummary {
        name: mf.name.clone(),
        description: mf.description.clone(),
        action_count: mf.actions.len(),
        total_duration_ms,
        created_at: mf.created_at,
    };
    // Save under the requested name, not the original
    let renamed = MacroFile {
        name: name.to_string(),
        ..mf
    };
    save_macro(&renamed)?;
    Ok(summary)
}

// ─── File I/O ─────────────────────────────────────────

fn load_macro(name: &str) -> anyhow::Result<MacroFile> {
    let path = macro_path(name);
    load_macro_file(&path)
}

fn load_macro_file(path: &std::path::Path) -> anyhow::Result<MacroFile> {
    let data = std::fs::read_to_string(path)?;
    let mf: MacroFile = serde_json::from_str(&data)?;
    Ok(mf)
}

fn save_macro(mf: &MacroFile) -> anyhow::Result<()> {
    let dir = macro_dir();
    std::fs::create_dir_all(&dir)?;
    let path = macro_path(&mf.name);
    let json = serde_json::to_string_pretty(mf)?;
    std::fs::write(&path, json)?;
    debug!("Saved macro '{}' to {}", mf.name, path.display());
    Ok(())
}

// ─── Helpers ──────────────────────────────────────────

fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
