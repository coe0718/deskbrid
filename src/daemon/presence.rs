//! User presence monitor — polls idle seconds and emits `presence.*` events
//! on state transitions (active ↔ idle).
//!
//! This is the push-event companion to the existing `system.idle` polling
//! action. Agents subscribe via the `subscribe` protocol and receive
//! `presence.active` / `presence.idle` events when the user state changes.
//!
//! Three states are tracked:
//!   - `active` — user recently provided input (`idle_seconds` < threshold)
//!   - `idle`   — no user input for `IDLE_THRESHOLD_SECS` seconds
//!   - `sleep`  — reserved for future logind signal handling
//!
//! Push: broadcasts `DeskbridEvent::PresenceActive` / `PresenceIdle` via the
//! shared `state.event_tx` subscribe channel — every subscribed client with
//! the matching event filter receives it.

use crate::DaemonState;
use crate::protocol::DeskbridEvent;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, info, warn};

/// Seconds of inactivity before we transition from `active` to `idle`.
/// Matches GNOME's default screen-blank idle hint and works as a reasonable
/// "user stepped away" signal for most agents.
pub(crate) const IDLE_THRESHOLD_SECS: u64 = 300; // 5 minutes

/// Poll interval. 5 s gives sub-minute detection latency for state changes
/// without burning CPU. Each poll is a cheap `xprintidle`/`loginctl` read
/// plus a brief RwLock acquisition that completes in <1 ms.
pub(crate) const POLL_INTERVAL_SECS: u64 = 5;

/// Discrete presence state for a given moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceState {
    Active,
    Idle,
    #[allow(dead_code)] // reserved: logind PrepareForSleep signal hookup (roadmap #138)
    Sleep,
}

impl PresenceState {
    fn as_str(self) -> &'static str {
        match self {
            PresenceState::Active => "active",
            PresenceState::Idle => "idle",
            PresenceState::Sleep => "sleep",
        }
    }
}

/// Snapshot returned from `system.presence.get`.
#[derive(Debug, Clone)]
pub struct PresenceSnapshot {
    pub state: PresenceState,
    pub idle_seconds: u64,
}

impl PresenceSnapshot {
    pub(crate) fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "state": self.state.as_str(),
            "idle_seconds": self.idle_seconds,
        })
    }
}

/// Shared presence state. Holds the latest snapshot so `system.presence.get`
/// can answer without touching the backend.
#[derive(Debug, Default)]
pub struct PresenceStore {
    pub snapshot: RwLock<Option<PresenceSnapshot>>,
}

/// Read the current presence snapshot without taking the backend.
///
/// Called from the action handler in `execute_action()`. Held only for the
/// duration of a `Option::clone()`, so contention is essentially zero.
pub(crate) async fn current_snapshot(state: &DaemonState) -> PresenceSnapshot {
    state
        .presence
        .snapshot
        .read()
        .await
        .clone()
        .unwrap_or(PresenceSnapshot {
            state: PresenceState::Active,
            idle_seconds: 0,
        })
}

/// Spawn the background presence monitor. Returns immediately; the monitor
/// runs for the lifetime of the daemon.
pub(crate) fn spawn_presence_monitor(state: Arc<DaemonState>) {
    tokio::spawn(async move {
        if let Err(e) = run_presence_monitor(state).await {
            warn!("presence monitor exited: {e:#}");
        }
    });
    info!(
        "Presence monitor spawned (poll_interval={}s, idle_threshold={}s)",
        POLL_INTERVAL_SECS, IDLE_THRESHOLD_SECS
    );
}

/// Main loop — polls idle, persists latest snapshot, and broadcasts
/// `Presence{Active,Idle}` events on transitions.
async fn run_presence_monitor(state: Arc<DaemonState>) -> anyhow::Result<()> {
    let mut ticker = interval(Duration::from_secs(POLL_INTERVAL_SECS));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let mut last_state: Option<PresenceState> = None;

    loop {
        ticker.tick().await;

        let (new_state, idle_seconds) = classify(read_idle_seconds(&state).await);
        let snapshot = PresenceSnapshot {
            state: new_state,
            idle_seconds,
        };

        // Persist latest snapshot for `system.presence.get`.
        *state.presence.snapshot.write().await = Some(snapshot);

        // Emit only on transitions, not on every tick.
        if Some(new_state) != last_state {
            debug!(
                "presence transition: {:?} → {:?} (idle={}s)",
                last_state, new_state, idle_seconds
            );
            broadcast_presence(&state, new_state, idle_seconds);
            last_state = Some(new_state);
        }
    }
}

/// Map an idle-seconds reading to a discrete state.
fn classify(idle_seconds_result: anyhow::Result<u64>) -> (PresenceState, u64) {
    match idle_seconds_result {
        Ok(secs) if secs >= IDLE_THRESHOLD_SECS => (PresenceState::Idle, secs),
        Ok(secs) => (PresenceState::Active, secs),
        Err(_) => (PresenceState::Active, 0), // backend read failed; assume active
    }
}

async fn read_idle_seconds(state: &DaemonState) -> anyhow::Result<u64> {
    let guard = state.backend.read().await;
    let backend = guard
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no backend loaded — cannot read idle seconds"))?;
    backend.idle_seconds().await
}

fn broadcast_presence(state: &DaemonState, new_state: PresenceState, idle_seconds: u64) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let event = match new_state {
        PresenceState::Active => DeskbridEvent::PresenceActive {
            state: "active".to_string(),
            idle_seconds,
            timestamp: now,
        },
        PresenceState::Idle => DeskbridEvent::PresenceIdle {
            state: "idle".to_string(),
            idle_seconds,
            timestamp: now,
        },
        PresenceState::Sleep => DeskbridEvent::PresenceSleep {
            state: "sleep".to_string(),
            idle_seconds,
            timestamp: now,
        },
    };

    // `event_tx.send` only fails if there are no subscribers (i.e. no
    // client is connected) — that's fine, we still updated the snapshot.
    if state.event_tx.send(event).is_err() {
        debug!("no subscribers for presence event");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_as_str_distinct() {
        assert_eq!(PresenceState::Active.as_str(), "active");
        assert_eq!(PresenceState::Idle.as_str(), "idle");
        assert_eq!(PresenceState::Sleep.as_str(), "sleep");
    }

    #[test]
    fn snapshot_to_json_shape() {
        let snap = PresenceSnapshot {
            state: PresenceState::Idle,
            idle_seconds: 312,
        };
        let v = snap.to_json();
        assert_eq!(v["state"], "idle");
        assert_eq!(v["idle_seconds"], 312);
    }

    #[test]
    fn classify_under_threshold_is_active() {
        let (s, secs) = classify(Ok(120));
        assert_eq!(s, PresenceState::Active);
        assert_eq!(secs, 120);
    }

    #[test]
    fn classify_at_threshold_is_idle() {
        let (s, secs) = classify(Ok(IDLE_THRESHOLD_SECS));
        assert_eq!(s, PresenceState::Idle);
        assert_eq!(secs, IDLE_THRESHOLD_SECS);
    }

    #[test]
    fn classify_above_threshold_is_idle() {
        let (s, secs) = classify(Ok(IDLE_THRESHOLD_SECS + 100));
        assert_eq!(s, PresenceState::Idle);
        assert_eq!(secs, IDLE_THRESHOLD_SECS + 100);
    }

    #[test]
    fn classify_backend_error_falls_back_active() {
        let (s, secs) = classify(Err(anyhow::anyhow!("xprintidle missing")));
        assert_eq!(s, PresenceState::Active);
        assert_eq!(secs, 0);
    }

    #[test]
    fn idle_threshold_is_five_minutes() {
        assert_eq!(IDLE_THRESHOLD_SECS, 300);
    }
}
