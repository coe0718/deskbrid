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

/// Time-of-day runtime config. Mutable via `system.time_of_day.config` action.
#[derive(Debug, Clone, Copy)]
pub struct TimeOfDayConfig {
    /// Latitude for solar calculations (sunrise/sunset). Default: None.
    pub latitude: Option<f64>,
    /// Longitude for solar calculations. Default: None.
    pub longitude: Option<f64>,
    /// Display time in 24-hour format. Default: true.
    pub format_24h: bool,
}

impl Default for TimeOfDayConfig {
    fn default() -> Self {
        Self {
            latitude: None,
            longitude: None,
            format_24h: true,
        }
    }
}

impl TimeOfDayConfig {
    pub(crate) fn to_json(self) -> serde_json::Value {
        let mut obj = serde_json::json!({"format_24h": self.format_24h});
        if let Some(lat) = self.latitude {
            obj["latitude"] = serde_json::json!(lat);
        }
        if let Some(lon) = self.longitude {
            obj["longitude"] = serde_json::json!(lon);
        }
        obj
    }
}

/// Snapshot returned from `system.time_of_day`.
#[derive(Debug, Clone)]
pub struct TimeOfDaySnapshot {
    pub local_time: String,
    pub unix_timestamp: i64,
    pub timezone: String,
    pub timezone_offset: i32,
    pub dst_active: bool,
    pub uptime_seconds: u64,
    pub boot_time: i64,
    pub day_of_week: u8,
    pub hour_of_day: u8,
    pub is_business_hours: bool,
    pub sunrise: Option<String>,
    pub sunset: Option<String>,
}

impl TimeOfDaySnapshot {
    pub(crate) fn to_json(&self) -> serde_json::Value {
        let mut obj = serde_json::json!({
            "local_time": self.local_time,
            "unix_timestamp": self.unix_timestamp,
            "timezone": self.timezone,
            "timezone_offset": self.timezone_offset,
            "dst_active": self.dst_active,
            "uptime_seconds": self.uptime_seconds,
            "boot_time": self.boot_time,
            "day_of_week": self.day_of_week,
            "hour_of_day": self.hour_of_day,
            "is_business_hours": self.is_business_hours,
        });
        if let Some(sr) = &self.sunrise {
            obj["sunrise"] = serde_json::json!(sr);
        }
        if let Some(ss) = &self.sunset {
            obj["sunset"] = serde_json::json!(ss);
        }
        obj
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
    pub time_of_day_config: Mutex<TimeOfDayConfig>,
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

/// Read the current runtime time-of-day config.
#[allow(dead_code)] // exposed for future use when config is queried standalone
pub(crate) async fn current_time_of_day_config(state: &DaemonState) -> TimeOfDayConfig {
    *state.presence.time_of_day_config.lock().await
}

/// Apply a config update from the `system.time_of_day.config` action.
/// `None` fields preserve the existing value. Returns the new config.
pub(crate) async fn update_time_of_day_config(
    state: &DaemonState,
    latitude: Option<f64>,
    longitude: Option<f64>,
    format_24h: Option<bool>,
) -> TimeOfDayConfig {
    let mut cfg = state.presence.time_of_day_config.lock().await;
    if let Some(lat) = latitude {
        cfg.latitude = Some(lat);
    }
    if let Some(lon) = longitude {
        cfg.longitude = Some(lon);
    }
    if let Some(f24) = format_24h {
        cfg.format_24h = f24;
    }
    *cfg
}

/// Build a `TimeOfDaySnapshot` for the `system.time_of_day` action.
/// Uses the configured latitude/longitude for solar times.
pub(crate) async fn current_time_of_day_snapshot(state: &DaemonState) -> TimeOfDaySnapshot {
    let cfg = *state.presence.time_of_day_config.lock().await;
    build_time_of_day_snapshot(cfg)
}

/// Build the time-of-day snapshot from config. Separated for testability.
fn build_time_of_day_snapshot(cfg: TimeOfDayConfig) -> TimeOfDaySnapshot {
    use chrono::{Datelike, Local, Offset, Timelike, Utc};

    let now_local = Local::now();
    let now_utc = Utc::now();
    let unix_ts = now_utc.timestamp();
    let timezone = now_local.offset().to_string();
    let offset = now_local.offset().fix().local_minus_utc();
    // FixedOffset has no DST; it's always standard time
    let dst_active = false;

    // Uptime from /proc/uptime
    let uptime_seconds = std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| {
            s.split_whitespace()
                .next()
                .and_then(|p| p.parse::<f64>().ok())
        })
        .map(|s| s as u64)
        .unwrap_or(0);
    let boot_time = unix_ts - uptime_seconds as i64;

    // Day of week (0=Mon...6=Sun for chrono, but we output 0=Sun...6=Sat)
    let day_of_week = ((now_local.weekday().num_days_from_monday() + 1) % 7) as u8;
    let hour_of_day = now_local.hour() as u8;

    // Business hours: Mon-Fri 9-17 local
    let is_business_hours = (1..=5).contains(&day_of_week) && (9..17).contains(&hour_of_day);

    // Local time string
    let local_time = if cfg.format_24h {
        now_local.format("%Y-%m-%d %H:%M:%S").to_string()
    } else {
        now_local.format("%Y-%m-%d %I:%M:%S %p").to_string()
    };

    // Sunrise/sunset if lat/lon configured
    let (sunrise, sunset) = if let (Some(lat), Some(lon)) = (cfg.latitude, cfg.longitude) {
        let (sr, ss) = calculate_sun_times(lat, lon, now_local.date_naive());
        (
            Some(sr.format("%H:%M:%S").to_string()),
            Some(ss.format("%H:%M:%S").to_string()),
        )
    } else {
        (None, None)
    };

    TimeOfDaySnapshot {
        local_time,
        unix_timestamp: unix_ts,
        timezone,
        timezone_offset: offset,
        dst_active,
        uptime_seconds,
        boot_time,
        day_of_week,
        hour_of_day,
        is_business_hours,
        sunrise,
        sunset,
    }
}

/// Calculate sunrise/sunset for a given date and coordinates.
/// Uses the standard NOAA solar position algorithm (simplified).
fn calculate_sun_times(
    lat: f64,
    lon: f64,
    date: chrono::NaiveDate,
) -> (chrono::NaiveTime, chrono::NaiveTime) {
    use chrono::{Datelike, Local, NaiveTime, Offset};

    // Solar declination angle
    let day_of_year = date.ordinal() as f64;
    let decl: f64 = -23.44
        * (360.0 / 365.0 * (day_of_year + 10.0))
            .to_radians()
            .cos()
            .to_degrees();
    let decl_rad = decl.to_radians();
    let lat_rad = lat.to_radians();

    // Hour angle at sunrise/sunset
    let cos_h = (-lat_rad.tan() * decl_rad.tan()).clamp(-1.0, 1.0);
    let h = cos_h.acos().to_degrees(); // degrees

    // Solar noon at this longitude
    let solar_noon_utc = 12.0 - lon / 15.0; // hours

    // Sunrise/sunset in UTC hours
    let sunrise_utc = solar_noon_utc - h / 15.0;
    let sunset_utc = solar_noon_utc + h / 15.0;

    // Convert to local time
    let tz_offset = Local::now().offset().fix().local_minus_utc() as f64 / 3600.0;
    let sunrise_local = sunrise_utc + tz_offset;
    let sunset_local = sunset_utc + tz_offset;

    // Normalize to 0-24 and create NaiveTime
    let norm = |h: f64| -> NaiveTime {
        let h = (h % 24.0 + 24.0) % 24.0;
        let hour = h.floor() as u32;
        let minute = ((h - hour as f64) * 60.0).round() as u32;
        let second = (((h - hour as f64) * 60.0 - minute as f64) * 60.0).round() as u32;
        NaiveTime::from_hms_opt(hour, minute.min(59), second.min(59))
            .unwrap_or(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
    };

    (norm(sunrise_local), norm(sunset_local))
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
