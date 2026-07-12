//! Locale and timezone change monitor.
//!
//! Subscribes to `org.freedesktop.locale1` (locale vars) and
//! `org.freedesktop.timedate1` (timezone) D-Bus interfaces on the
//! system bus. When either interface fires its `PropertiesChanged`
//! signal, this monitor queries the current value via GetAll and
//! emits a `DeskbridEvent` if the value has changed since last seen.
//!
//! Implementation note: parsing the `PropertiesChanged` signal body
//! directly is brittle (the variant unwrap trap from #57). It's
//! cleaner to just call GetAll on the same interface whenever the
//! signal fires — the daemon gives us the current state, we diff it
//! against our last seen state, and emit on change. This pattern also
//! handles the "cold start" case naturally: if the daemon starts
//! after the locale was already set, we won't miss it (the next
//! signal will catch it; and we don't fire spuriously because we
//! compare against the initial GetAll read).
//!
//! Each monitor uses its own `zbus::Connection` because a single
//! locked Connection can't be safely shared across two concurrent
//! `for_match_rule` streams.

use crate::daemon::DaemonState;
use crate::protocol::DeskbridEvent;
use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use zbus::Connection;

const LOCALE1_PATH: &str = "/org/freedesktop/locale1";
const LOCALE1_SERVICE: &str = "org.freedesktop.locale1";
const TIMEDATE1_PATH: &str = "/org/freedesktop/timedate1";
const TIMEDATE1_SERVICE: &str = "org.freedesktop.timedate1";

/// Spawn both the locale and timezone monitors. They run until
/// the daemon shuts down (or the D-Bus connection drops, in which
/// case they exit silently and log a warning).
pub fn spawn_locale_timezone_monitors(state: Arc<DaemonState>) {
    let locale_state = Arc::clone(&state);
    tokio::spawn(async move {
        // W6 (Vex review): reconnect loop with exponential backoff.
        // If the D-Bus connection drops, the monitor task exits silently
        // — but now we respawn it. Backoff: 1s, 2s, 4s, 8s, 16s, 30s cap.
        let mut backoff_ms = 1000u64;
        loop {
            if locale_state
                .shutdown
                .load(std::sync::atomic::Ordering::Relaxed)
            {
                break;
            }
            match run_locale_monitor(Arc::clone(&locale_state)).await {
                Ok(()) => {
                    tracing::info!("locale monitor exited cleanly; reconnecting");
                    backoff_ms = 1000;
                }
                Err(e) => {
                    tracing::warn!(
                        "locale monitor error: {e}; reconnecting in {}ms",
                        backoff_ms
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                    backoff_ms = (backoff_ms * 2).min(30_000);
                }
            }
        }
    });
    let tz_state = Arc::clone(&state);
    tokio::spawn(async move {
        // W6 (Vex review): same reconnect pattern for the timezone monitor.
        let mut backoff_ms = 1000u64;
        loop {
            if tz_state.shutdown.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            match run_timezone_monitor(Arc::clone(&tz_state)).await {
                Ok(()) => {
                    tracing::info!("timezone monitor exited cleanly; reconnecting");
                    backoff_ms = 1000;
                }
                Err(e) => {
                    tracing::warn!(
                        "timezone monitor error: {e}; reconnecting in {}ms",
                        backoff_ms
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                    backoff_ms = (backoff_ms * 2).min(30_000);
                }
            }
        }
    });
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn broadcast(tx: &tokio::sync::broadcast::Sender<DeskbridEvent>, event: DeskbridEvent) {
    if tx.send(event).is_err() {
        // No subscribers — that's fine, just don't spam logs.
        tracing::debug!("locale/timezone event dropped (no subscribers)");
    }
}

// ---------- Locale monitor ----------

async fn run_locale_monitor(state: Arc<DaemonState>) -> Result<()> {
    let conn = Connection::system()
        .await
        .context("locale monitor: system bus connection")?;
    let rule = match build_properties_changed_rule(LOCALE1_PATH) {
        Some(r) => r,
        None => return Ok(()),
    };
    let mut stream = zbus::MessageStream::for_match_rule(rule, &conn, None).await?;
    // Seed initial value so we don't fire a spurious event for the
    // state already in place at startup.
    let mut last_locale = read_locale_value(&conn).await.unwrap_or_default();
    tracing::info!(
        "locale monitor: subscribed to {} (initial: {} entries)",
        LOCALE1_SERVICE,
        last_locale.len()
    );
    use futures_util::StreamExt;
    while let Some(msg) = stream.next().await {
        let Ok(_msg) = msg else { continue };
        match read_locale_value(&conn).await {
            Ok(current) => {
                if current != last_locale {
                    tracing::debug!(
                        "locale changed: {} -> {} entries",
                        last_locale.len(),
                        current.len()
                    );
                    last_locale = current.clone();
                    broadcast(
                        &state.event_tx,
                        DeskbridEvent::LocaleChanged {
                            locale: current,
                            timestamp: unix_now(),
                        },
                    );
                }
            }
            Err(e) => {
                tracing::debug!("locale monitor: GetAll failed: {}", e);
            }
        }
    }
    Ok(())
}

/// Read the `Locale` property from org.freedesktop.locale1 via GetAll.
/// Returns the array of `KEY=VALUE` strings (e.g. `["LANG=en_US.UTF-8"]`).
async fn read_locale_value(conn: &Connection) -> Result<Vec<String>> {
    use zbus::zvariant::{Array, OwnedValue};
    let reply = conn
        .call_method(
            Some(LOCALE1_SERVICE),
            LOCALE1_PATH,
            Some("org.freedesktop.DBus.Properties"),
            "GetAll",
            &("org.freedesktop.locale1",),
        )
        .await
        .context("locale monitor: GetAll call")?;
    let props: std::collections::HashMap<String, OwnedValue> = reply
        .body()
        .deserialize()
        .context("locale monitor: GetAll body")?;
    let locale = props
        .get("Locale")
        .ok_or_else(|| anyhow::anyhow!("locale monitor: Locale property missing"))?;
    let arr = locale
        .downcast_ref::<Array>()
        .map_err(|e| anyhow::anyhow!("locale monitor: Locale not array: {e}"))?;
    let mut out: Vec<String> = Vec::with_capacity(arr.len());
    for entry in arr.iter() {
        // Each entry is a Str
        if let Ok(s) = entry.downcast_ref::<zbus::zvariant::Str>() {
            out.push(s.to_string());
        }
    }
    Ok(out)
}

// ---------- Timezone monitor ----------

async fn run_timezone_monitor(state: Arc<DaemonState>) -> Result<()> {
    let conn = Connection::system()
        .await
        .context("timezone monitor: system bus connection")?;
    let rule = match build_properties_changed_rule(TIMEDATE1_PATH) {
        Some(r) => r,
        None => return Ok(()),
    };
    let mut stream = zbus::MessageStream::for_match_rule(rule, &conn, None).await?;
    let mut last_timezone = read_timezone_value(&conn).await.unwrap_or_default();
    tracing::info!(
        "timezone monitor: subscribed to {} (initial: {:?})",
        TIMEDATE1_SERVICE,
        last_timezone
    );
    use futures_util::StreamExt;
    while let Some(msg) = stream.next().await {
        let Ok(_msg) = msg else { continue };
        match read_timezone_value(&conn).await {
            Ok(Some(current)) => {
                if Some(&current) != last_timezone.as_ref() {
                    tracing::debug!("timezone changed: {:?} -> {:?}", last_timezone, current);
                    last_timezone = Some(current.clone());
                    broadcast(
                        &state.event_tx,
                        DeskbridEvent::TimezoneChanged {
                            timezone: current,
                            timestamp: unix_now(),
                        },
                    );
                }
            }
            Ok(None) => {
                // Empty value (rare) — don't emit
                tracing::debug!("timezone monitor: GetAll returned empty Timezone");
            }
            Err(e) => {
                tracing::debug!("timezone monitor: GetAll failed: {}", e);
            }
        }
    }
    Ok(())
}

/// Read the `Timezone` property from org.freedesktop.timedate1 via GetAll.
async fn read_timezone_value(conn: &Connection) -> Result<Option<String>> {
    use zbus::zvariant::OwnedValue;
    let reply = conn
        .call_method(
            Some(TIMEDATE1_SERVICE),
            TIMEDATE1_PATH,
            Some("org.freedesktop.DBus.Properties"),
            "GetAll",
            &("org.freedesktop.timedate1",),
        )
        .await
        .context("timezone monitor: GetAll call")?;
    let props: std::collections::HashMap<String, OwnedValue> = reply
        .body()
        .deserialize()
        .context("timezone monitor: GetAll body")?;
    let tz = props
        .get("Timezone")
        .ok_or_else(|| anyhow::anyhow!("timezone monitor: Timezone property missing"))?;
    let s = tz
        .downcast_ref::<zbus::zvariant::Str>()
        .map_err(|e| anyhow::anyhow!("timezone monitor: Timezone not str: {e}"))?;
    let val = s.to_string();
    if val.is_empty() {
        Ok(None)
    } else {
        Ok(Some(val))
    }
}

/// Build a MatchRule for `org.freedesktop.DBus.Properties.PropertiesChanged`
/// on the given object path. Returns None if the rule builder fails.
fn build_properties_changed_rule(path: &str) -> Option<zbus::MatchRule<'static>> {
    use zbus::MatchRule;
    use zbus::message::Type as MsgType;
    // Leak path into a 'static string so it satisfies the builder's
    // 'static lifetime. Safe because the rule is consumed at startup
    // and never deallocated.
    let path_static: &'static str = Box::leak(path.to_string().into_boxed_str());
    let iface_static: &'static str = "org.freedesktop.DBus.Properties";
    let member_static: &'static str = "PropertiesChanged";
    let rule = MatchRule::builder()
        .msg_type(MsgType::Signal)
        .interface(iface_static)
        .ok()?
        .member(member_static)
        .ok()?
        .path(path_static)
        .ok()?
        .build();
    Some(rule.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_now_returns_recent_seconds() {
        let t = unix_now();
        assert!(t > 1_700_000_000, "got {}", t);
    }

    #[test]
    fn build_rule_produces_valid_rule() {
        let rule = build_properties_changed_rule(LOCALE1_PATH);
        assert!(rule.is_some());
        let rule = build_properties_changed_rule(TIMEDATE1_PATH);
        assert!(rule.is_some());
    }

    #[test]
    fn broadcast_drops_without_subscribers() {
        let (tx, _rx) = tokio::sync::broadcast::channel::<DeskbridEvent>(1);
        drop(_rx);
        // Should not panic; just drops the event.
        broadcast(
            &tx,
            DeskbridEvent::LocaleChanged {
                locale: vec!["LANG=en_US.UTF-8".to_string()],
                timestamp: 1234,
            },
        );
    }
}
