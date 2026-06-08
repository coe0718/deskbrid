//! Cron-style scheduled actions — the daemon runs configured actions on a timer.
//!
//! Schedule is stored at `~/.config/deskbrid/schedule.json` as a JSON array
//! of entries. Each entry has a name, an interval in seconds, and a raw JSON
//! action string matching the Deskbrid protocol format.
//!
//! TESTING_NEEDED: Verify schedule persistence across daemon restarts and
//! that long-running actions don't block the schedule loop.

use super::helpers::home_dir;
use crate::DaemonState;
use crate::protocol::Action;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

const CHECK_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleEntry {
    pub name: String,
    /// Interval in seconds between runs
    pub interval_secs: u64,
    /// The action type string (e.g. "screenshot", "system.info")
    pub action_type: String,
    /// Optional action parameters as a JSON object
    #[serde(default)]
    pub action_params: serde_json::Value,
    /// Last run timestamp (Unix epoch seconds). Managed by the engine.
    #[serde(default)]
    pub last_run: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Schedule {
    pub entries: Vec<ScheduleEntry>,
}

impl Schedule {
    fn path() -> PathBuf {
        let home = home_dir().to_string_lossy().to_string();
        PathBuf::from(home)
            .join(".config")
            .join("deskbrid")
            .join("schedule.json")
    }

    pub async fn load() -> Self {
        let path = Self::path();
        match tokio::fs::read_to_string(&path).await {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|e| {
                warn!("Failed to parse schedule file: {e}. Starting with empty schedule.");
                Schedule::default()
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                debug!("No schedule file found, starting empty.");
                Schedule::default()
            }
            Err(e) => {
                warn!("Failed to read schedule file: {e}. Starting with empty schedule.");
                Schedule::default()
            }
        }
    }

    pub async fn save(&self) -> anyhow::Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let json = serde_json::to_string_pretty(self)?;
        tokio::fs::write(&path, json).await?;
        Ok(())
    }

    /// Return which entries are due to run (last_run + interval <= now).
    pub fn due_entries(&self, now_secs: u64) -> Vec<&ScheduleEntry> {
        self.entries
            .iter()
            .filter(|e| now_secs.saturating_sub(e.last_run) >= e.interval_secs)
            .collect()
    }
}

/// Shared schedule state protected by a Mutex.
pub struct ScheduleState {
    pub schedule: Mutex<Schedule>,
}

impl ScheduleState {
    pub fn new() -> Self {
        Self {
            schedule: Mutex::new(Schedule::default()),
        }
    }
}

/// Spawn the schedule engine background task.
pub fn spawn_schedule_engine(schedule_state: Arc<ScheduleState>, daemon_state: Arc<DaemonState>) {
    tokio::spawn(async move {
        info!(
            "Schedule engine started (check interval: {:?})",
            CHECK_INTERVAL
        );

        loop {
            // Reload schedule from disk on every tick so changes take effect
            // without requiring a daemon restart.
            {
                let mut sched = schedule_state.schedule.lock().await;
                *sched = Schedule::load().await;
            }

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let due: Vec<ScheduleEntry> = {
                let sched = schedule_state.schedule.lock().await;
                sched.due_entries(now).into_iter().cloned().collect()
            };

            for mut entry in due {
                info!(
                    "Running scheduled action '{}' ({})",
                    entry.name, entry.action_type
                );

                // Build the protocol action JSON string
                let mut action_json = serde_json::json!({
                    "type": entry.action_type,
                    "id": format!("scheduled-{}", entry.name),
                });
                if let serde_json::Value::Object(ref params) = entry.action_params {
                    for (k, v) in params {
                        action_json[k] = v.clone();
                    }
                }

                let action_str = serde_json::to_string(&action_json).unwrap_or_default();

                match Action::from_json(&action_str) {
                    Ok((request_id, action)) => {
                        let seq = crate::daemon::helpers::unix_timestamp();
                        let result = crate::daemon::dispatch::dispatch_action(
                            &request_id,
                            action,
                            &daemon_state,
                            0, // peer_uid: scheduled actions run as daemon
                            seq,
                        )
                        .await;
                        debug!(
                            "Scheduled action '{}' completed: {:?}",
                            entry.name,
                            result.get("status")
                        );
                    }
                    Err(e) => {
                        error!("Failed to parse scheduled action '{}': {e}", entry.name);
                    }
                }

                // Update last_run
                entry.last_run = now;
                let mut sched = schedule_state.schedule.lock().await;
                if let Some(existing) = sched.entries.iter_mut().find(|e| e.name == entry.name) {
                    existing.last_run = now;
                }
                if let Err(e) = sched.save().await {
                    error!("Failed to save schedule: {e}");
                }
            }

            tokio::time::sleep(CHECK_INTERVAL).await;
        }
    });
}
