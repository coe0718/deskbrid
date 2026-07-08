use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::Action;
use serde_json::Value;

use super::{build_system_capabilities, run_system_remediation};

pub(crate) async fn execute_stubs(
    action: Action,
    backend: &dyn DesktopBackend,
    state: &DaemonState,
) -> anyhow::Result<Value> {
    use Action::*;
    Ok(match action {
        // ─── System ──────────────────────────────────
        SystemInfo => serde_json::json!(backend.system_info().await?),
        SystemCapabilities => serde_json::json!(build_system_capabilities(backend).await?),
        SystemConfinement => serde_json::json!(crate::daemon::build_confinement_report().await?),
        SystemIdle => serde_json::json!({"idle_seconds": backend.idle_seconds().await?}),
        PresenceGet => {
            let snapshot = crate::daemon::presence::current_snapshot(state).await;
            snapshot.to_json()
        }
        PresenceConfig {
            idle_threshold_secs,
            away_threshold_secs,
        } => {
            let new_cfg = crate::daemon::presence::update_config(
                state,
                idle_threshold_secs,
                away_threshold_secs,
            )
            .await;
            new_cfg.to_json()
        }
        TimeOfDay => {
            let snapshot = crate::daemon::presence::current_time_of_day_snapshot(state).await;
            snapshot.to_json()
        }
        TimeOfDayConfig {
            latitude,
            longitude,
            format_24h,
        } => {
            let new_cfg = crate::daemon::presence::update_time_of_day_config(
                state, latitude, longitude, format_24h,
            )
            .await;
            new_cfg.to_json()
        }
        SystemRemediate { ref check, apply } => {
            serde_json::json!(run_system_remediation(check, apply).await?)
        }

        // ─── Ping ────────────────────────────────────
        Ping => serde_json::json!({"ok": true}),

        // ─── Location ────────────────────────────────
        LocationGet => {
            serde_json::json!(get_location().await)
        }

        // ─── UI automation (browser-side — not AT-SPI) ──
        UiTreeGet => {
            // AT-SPI tree via a11y module (for desktop UI, not browser DOM)
            crate::a11y::tree(Some(5)).await?
        }
        UiElementClick {
            ref selector,
            tab_index,
        } => crate::browser::click(tab_index, selector).await?,
        UiElementSetText {
            ref selector,
            ref text,
            tab_index,
        } => crate::browser::set_text(tab_index, selector, text).await?,

        // ─── Catch-all for actions handled before desktop dispatch ──
        SystemInhibit { .. }
        | SystemReleaseInhibit { .. }
        | SystemListSessions
        | SystemLockSession { .. }
        | SystemSwitchUser { .. }
        | SystemCheckAuth { .. }
        | SystemElevate { .. }
        | ServiceStatus { .. }
        | ServiceStart { .. }
        | ServiceStop { .. }
        | ServiceRestart { .. }
        | ServiceEnable { .. }
        | ServiceDisable { .. }
        | ServiceList { .. }
        | JournalQuery { .. }
        | TimerList
        | TimerStart { .. }
        | TimerStop { .. }
        | WaitFor { .. }
        | TerminalCreate { .. }
        | TerminalWrite { .. }
        | TerminalRead { .. }
        | TerminalResize { .. }
        | TerminalList
        | TerminalKill { .. }
        | Subscribe { .. }
        | Unsubscribe { .. }
        | SystemPrintFile { .. }
        | Disconnect => {
            anyhow::bail!(
                "internal dispatch error: action reached execute_stubs but should have been handled earlier"
            )
        }

        _ => anyhow::bail!("internal dispatch error: unexpected action in execute_stubs"),
    })
}

// ─── Location helpers ─────────────────────────────────

async fn get_location() -> serde_json::Value {
    // Try geoclue D-Bus first (GNOME location service)
    if let Ok(loc) = geoclue_lookup().await {
        return loc;
    }

    // Fall back to IP-based geolocation
    if let Ok(loc) = ip_geo_lookup().await {
        return loc;
    }

    serde_json::json!({
        "source": "none",
        "error": "no location provider available"
    })
}

async fn geoclue_lookup() -> anyhow::Result<serde_json::Value> {
    let output = tokio::process::Command::new("where-am-i")
        .args(["-f", "json"])
        .output()
        .await?;

    if output.status.success() {
        let body = String::from_utf8_lossy(&output.stdout);
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
            let mut loc = serde_json::json!({"source": "geoclue"});
            if let Some(lat) = v.get("latitude") {
                loc["latitude"] = lat.clone();
            }
            if let Some(lon) = v.get("longitude") {
                loc["longitude"] = lon.clone();
            }
            if let Some(acc) = v.get("accuracy") {
                loc["accuracy_meters"] = acc.clone();
            }
            return Ok(loc);
        }
    }

    anyhow::bail!("geoclue lookup failed")
}

async fn ip_geo_lookup() -> anyhow::Result<serde_json::Value> {
    let response = reqwest::get("https://ipapi.co/json/")
        .await?
        .json::<serde_json::Value>()
        .await?;

    let mut loc = serde_json::json!({"source": "ip"});
    if let Some(lat) = response.get("latitude") {
        loc["latitude"] = lat.clone();
    }
    if let Some(lon) = response.get("longitude") {
        loc["longitude"] = lon.clone();
    }
    if let Some(city) = response.get("city") {
        loc["city"] = city.clone();
    }
    if let Some(region) = response.get("region") {
        loc["region"] = region.clone();
    }
    if let Some(country) = response.get("country_name") {
        loc["country"] = country.clone();
    }

    Ok(loc)
}
