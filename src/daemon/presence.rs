//! User presence monitor — polls idle seconds and emits `presence.*` events
//! on state transitions (active ↔ idle ↔ locked ↔ returned).
//!
//! Push events broadcast on transitions:
//!   - `presence.active`    — user just provided input (came back from idle)
//!   - `presence.idle`      — idle threshold exceeded
//!   - `presence.returned`  — user returned after being idle (carries `idle_duration_secs`)
//!   - `presence.locked`    — screen locked (logind `LockedHint` true)
//!   - `presence.unlocked`  — screen unlocked (logind `LockedHint` false)
//!   - `presence.sleep`     — reserved for logind `PrepareForSleep` signal
//!
//! Pull: `system.presence.get` → `PresenceSnapshot { state, idle_seconds, last_active, locked }`
//! Configure: `system.presence.config { idle_threshold_secs, away_threshold_secs }` →
//!   updates the runtime thresholds (returns the new config).

use crate::DaemonState;
use crate::protocol::DeskbridEvent;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tokio::time::interval;
use tracing::{debug, info, warn};

/// Default seconds of inactivity before transitioning from `active` to `idle`.
/// Matches GNOME's default screen-blank idle hint.
pub(crate) const DEFAULT_IDLE_THRESHOLD_SECS: u64 = 300; // 5 min

/// Default seconds of inactivity before transitioning from `idle` to `away`.
/// Long-leave sentinel for agents that distinguish "stepped away" from
/// "still here but reading". `Away` state surfaces only as `idle_seconds`
/// in the snapshot — there is no discrete `Away` variant (keeps the state
/// machine simple), but the threshold is exposed via `PresenceConfig`.
pub(crate) const DEFAULT_AWAY_THRESHOLD_SECS: u64 = 900; // 15 min

/// Poll interval. 5 s gives sub-minute detection latency for state changes
/// without burning CPU. Each poll is a cheap `xprintidle`/`loginctl` read
/// plus a brief RwLock acquisition that completes in <1 ms.
pub(crate) const POLL_INTERVAL_SECS: u64 = 5;

/// Runtime presence thresholds. Mutable via the `system.presence.config`
/// action — guarded by a `Mutex<PresenceConfig>` on `PresenceStore`.
#[derive(Debug, Clone, Copy)]
pub struct PresenceConfig {
    /// Seconds of inactivity → `idle` state. Default: 300.
    pub idle_threshold_secs: u64,
    /// Seconds of inactivity → `away` (long-leave) tier. Default: 900.
    /// Surfaced in config but not yet a discrete `PresenceState` variant.
    pub away_threshold_secs: u64,
}

impl Default for PresenceConfig {
    fn default() -> Self {
        Self {
            idle_threshold_secs: DEFAULT_IDLE_THRESHOLD_SECS,
            away_threshold_secs: DEFAULT_AWAY_THRESHOLD_SECS,
        }
    }
}

impl PresenceConfig {
    pub(crate) fn to_json(self) -> serde_json::Value {
        serde_json::json!({
            "idle_threshold_secs": self.idle_threshold_secs,
            "away_threshold_secs": self.away_threshold_secs,
        })
    }
}

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
#[derive(Debug, Clone, Copy)]
pub struct PresenceSnapshot {
    pub state: PresenceState,
    pub idle_seconds: u64,
    /// Approximate epoch-seconds timestamp of last user input
    /// (`now() - idle_seconds` at poll time).
    pub last_active: u64,
    /// True if logind reports the session as locked.
    pub locked: bool,
}

impl PresenceSnapshot {
    pub(crate) fn to_json(self) -> serde_json::Value {
        serde_json::json!({
            "state": self.state.as_str(),
            "idle_seconds": self.idle_seconds,
            "last_active": self.last_active,
            "locked": self.locked,
        })
    }

    fn empty_active() -> Self {
        Self {
            state: PresenceState::Active,
            idle_seconds: 0,
            last_active: now_secs(),
            locked: false,
        }
    }
}

/// Shared presence state. Holds the latest snapshot so `system.presence.get`
/// can answer without touching the backend, and the runtime config that
/// `system.presence.config` updates.
#[derive(Debug, Default)]
pub struct PresenceStore {
    pub snapshot: RwLock<Option<PresenceSnapshot>>,
    pub config: Mutex<PresenceConfig>,
}

/// Read the current presence snapshot without taking the backend.
/// Held only for the duration of a `Option::clone()`.
pub(crate) async fn current_snapshot(state: &DaemonState) -> PresenceSnapshot {
    state
        .presence
        .snapshot
        .read()
        .await
        .unwrap_or_else(PresenceSnapshot::empty_active)
}

/// Read the current runtime presence config.
#[allow(dead_code)] // exposed for future use when config is queried standalone
pub(crate) async fn current_config(state: &DaemonState) -> PresenceConfig {
    *state.presence.config.lock().await
}

/// Apply a config update from the `system.presence.config` action.
/// `None` fields preserve the existing value. Returns the new config.
pub(crate) async fn update_config(
    state: &DaemonState,
    idle_threshold_secs: Option<u64>,
    away_threshold_secs: Option<u64>,
) -> PresenceConfig {
    let mut cfg = state.presence.config.lock().await;
    if let Some(idle) = idle_threshold_secs {
        cfg.idle_threshold_secs = idle;
    }
    if let Some(away) = away_threshold_secs {
        cfg.away_threshold_secs = away;
    }
    *cfg
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
        "Presence monitor spawned (poll_interval={}s, idle_threshold={}s, away_threshold={}s)",
        POLL_INTERVAL_SECS, DEFAULT_IDLE_THRESHOLD_SECS, DEFAULT_AWAY_THRESHOLD_SECS
    );
}

/// Internal per-tick observation. Holds everything the poll loop needs to
/// decide on a transition and write a fresh snapshot.
struct Observation {
    state: PresenceState,
    idle_seconds: u64,
    locked: bool,
}

/// Main loop — polls idle + locked, persists snapshot, and broadcasts
/// events on state transitions (active/idle/locked/unlocked/returned).
async fn run_presence_monitor(state: Arc<DaemonState>) -> anyhow::Result<()> {
    let mut ticker = interval(Duration::from_secs(POLL_INTERVAL_SECS));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let mut last_state: Option<PresenceState> = None;
    let mut last_locked: Option<bool> = None;
    // Wall-clock second at which we entered the idle state. Used to compute
    // `idle_duration_secs` for `PresenceReturned` when the user comes back.
    let mut idle_since: Option<u64> = None;

    loop {
        ticker.tick().await;

        let cfg = *state.presence.config.lock().await;
        let idle_seconds = read_idle_seconds(&state).await.ok().unwrap_or(0);
        let locked = read_locked_hint_zbus().await.unwrap_or(false);

        let class = if idle_seconds >= cfg.idle_threshold_secs {
            PresenceState::Idle
        } else {
            PresenceState::Active
        };

        let observation = Observation {
            state: class,
            idle_seconds,
            locked,
        };

        let now = now_secs();
        let snapshot = PresenceSnapshot {
            state: observation.state,
            idle_seconds: observation.idle_seconds,
            last_active: now.saturating_sub(observation.idle_seconds),
            locked: observation.locked,
        };
        *state.presence.snapshot.write().await = Some(snapshot);

        // State transition (active ↔ idle)
        if Some(observation.state) != last_state {
            debug!(
                "presence transition: {:?} → {:?} (idle={}s, locked={})",
                last_state, observation.state, observation.idle_seconds, observation.locked
            );

            match (last_state, observation.state) {
                // idle → active means the user returned. Emit `presence.returned`
                // carrying the duration of the idle window, then `presence.active`.
                (Some(PresenceState::Idle), PresenceState::Active) => {
                    let idle_duration = idle_since.map(|s| now.saturating_sub(s)).unwrap_or(0);
                    broadcast_returned(&state, idle_duration, now);
                    broadcast_basic(&state, PresenceState::Active, observation.idle_seconds, now);
                    idle_since = None;
                }
                // active → idle: mark the entry time for the next return event.
                (Some(PresenceState::Active), PresenceState::Idle) => {
                    idle_since = Some(now);
                    broadcast_basic(&state, PresenceState::Idle, observation.idle_seconds, now);
                }
                // First-ever tick or any other transition: just emit the basic event.
                _ => {
                    broadcast_basic(&state, observation.state, observation.idle_seconds, now);
                }
            }
            last_state = Some(observation.state);
        }

        // Locked transition (independent of active/idle — you can lock while idle)
        if Some(observation.locked) != last_locked {
            if observation.locked {
                broadcast_lock_change(&state, true, now);
            } else {
                broadcast_lock_change(&state, false, now);
            }
            last_locked = Some(observation.locked);
        }
    }
}

async fn read_idle_seconds(state: &DaemonState) -> anyhow::Result<u64> {
    let guard = state.backend.read().await;
    let backend = guard
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no backend loaded — cannot read idle seconds"))?;
    backend.idle_seconds().await
}

/// Read `LockedHint` from `org.freedesktop.login1.Session` at path
/// `/org/freedesktop/login1/session/self`. Returns `Ok(false)` if the
/// session bus or logind is unavailable (containers, headless, absence of
/// session cookie) — graceful fallback so the monitor never aborts.
async fn read_locked_hint_zbus() -> anyhow::Result<bool> {
    use zbus::Connection;

    let conn = match Connection::system().await {
        Ok(c) => c,
        Err(_) => return Ok(false),
    };

    // `org.freedesktop.login1.Session` exposes `LockedHint` (b) on
    // `/org/freedesktop/login1/session/self` (resolves to the caller's seat).
    let reply = match conn
        .call_method(
            Some("org.freedesktop.login1"),
            "/org/freedesktop/login1/session/self",
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &("org.freedesktop.login1.Session", "LockedHint"),
        )
        .await
    {
        Ok(r) => r,
        // Session object doesn't exist — no logind seat for this process.
        Err(_) => return Ok(false),
    };

    let body = reply.body();
    let val: zbus::zvariant::Value = match body.deserialize() {
        Ok(v) => v,
        Err(_) => return Ok(false),
    };

    // `Get` returns a `Value<'v'>` wrapping the property as a variant.
    // Unwrap `Value::Bool(b)` from the inner `Value`.
    Ok(match val {
        zbus::zvariant::Value::Bool(b) => b,
        zbus::zvariant::Value::Value(inner) => {
            matches!(*inner, zbus::zvariant::Value::Bool(true))
        }
        _ => false,
    })
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn broadcast_basic(state: &DaemonState, new_state: PresenceState, idle_seconds: u64, now: u64) {
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
    send(&state.event_tx, event);
}

fn broadcast_returned(state: &DaemonState, idle_duration_secs: u64, now: u64) {
    send(
        &state.event_tx,
        DeskbridEvent::PresenceReturned {
            idle_duration_secs,
            timestamp: now,
        },
    );
}

fn broadcast_lock_change(state: &DaemonState, locked: bool, now: u64) {
    let event = if locked {
        DeskbridEvent::PresenceLocked { timestamp: now }
    } else {
        DeskbridEvent::PresenceUnlocked { timestamp: now }
    };
    send(&state.event_tx, event);
}

fn send(tx: &tokio::sync::broadcast::Sender<DeskbridEvent>, event: DeskbridEvent) {
    // `event_tx.send` only fails when there are no subscribers — that's
    // expected when no client is connected, so log at debug level only.
    if tx.send(event).is_err() {
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
    fn snapshot_to_json_shape_includes_locked_and_last_active() {
        let snap = PresenceSnapshot {
            state: PresenceState::Idle,
            idle_seconds: 312,
            last_active: 1747000000,
            locked: true,
        };
        let v = snap.to_json();
        assert_eq!(v["state"], "idle");
        assert_eq!(v["idle_seconds"], 312);
        assert_eq!(v["last_active"], 1747000000);
        assert_eq!(v["locked"], true);
    }

    #[test]
    fn snapshot_empty_active_has_sensible_defaults() {
        let snap = PresenceSnapshot::empty_active();
        assert_eq!(snap.state, PresenceState::Active);
        assert_eq!(snap.idle_seconds, 0);
        assert_eq!(snap.locked, false);
        assert!(snap.last_active > 0); // epoch seconds, non-zero
    }

    #[test]
    fn config_default_thresholds_match_roadmap() {
        let cfg = PresenceConfig::default();
        assert_eq!(cfg.idle_threshold_secs, 300);
        assert_eq!(cfg.away_threshold_secs, 900);
    }

    #[test]
    fn config_to_json_shape() {
        let cfg = PresenceConfig {
            idle_threshold_secs: 120,
            away_threshold_secs: 600,
        };
        let v = cfg.to_json();
        assert_eq!(v["idle_threshold_secs"], 120);
        assert_eq!(v["away_threshold_secs"], 600);
    }

    #[test]
    fn empty_active_now_secs_is_recent() {
        // Within a few seconds of "now" — guards against regression where
        // last_active would be 0 because we forgot to compute it.
        let before = now_secs();
        let snap = PresenceSnapshot::empty_active();
        let after = now_secs();
        assert!(snap.last_active >= before && snap.last_active <= after);
    }

    #[test]
    fn default_idle_threshold_is_five_minutes() {
        assert_eq!(DEFAULT_IDLE_THRESHOLD_SECS, 300);
    }

    #[test]
    fn default_away_threshold_is_fifteen_minutes() {
        assert_eq!(DEFAULT_AWAY_THRESHOLD_SECS, 900);
    }
}
