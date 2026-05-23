use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::Action;
use serde_json::Value;

use super::{build_system_capabilities, run_system_remediation};

pub(crate) async fn execute_stubs(
    action: Action,
    backend: &dyn DesktopBackend,
    _state: &DaemonState,
) -> anyhow::Result<Value> {
    use Action::*;
    Ok(match action {
        // ─── System ──────────────────────────────────

        SystemInfo => serde_json::json!(backend.system_info().await?),
        SystemCapabilities => serde_json::json!(build_system_capabilities(backend).await?),
        SystemConfinement => serde_json::json!(crate::daemon::build_confinement_report().await?),
        SystemIdle => serde_json::json!({"idle_seconds": backend.idle_seconds().await?}),
        SystemRemediate { ref check, apply } => {
            serde_json::json!(run_system_remediation(check, apply).await?)
        }

        // ─── Ping ────────────────────────────────────

        Ping => serde_json::json!({"ok": true}),

        // ─── Location ────────────────────────────────

        LocationGet => {
            serde_json::json!(get_location().await)
        }

        // ─── AT-SPI: core (existing protocol) ────────

        A11yTree { depth } => {
            crate::a11y::tree(depth).await?
        }
        A11yGetElement {
            role,
            ref name,
            index,
        } => crate::a11y::get_element(role.as_deref(), name.as_deref(), index).await?,
        A11yClickElement {
            role,
            ref name,
            index,
        } => crate::a11y::click_element(role.as_deref(), name.as_deref(), index).await?,
        A11yGetText {
            role,
            ref name,
            index,
        } => crate::a11y::get_text(role.as_deref(), name.as_deref(), index).await?,

        // ─── AT-SPI: expanded (computer-use-linux compat) ──

        A11ySnapshotTree {
            app_name,
            pid,
            max_nodes,
            max_depth,
        } => {
            crate::a11y::tree::snapshot_tree(
                app_name.as_deref(),
                pid,
                max_nodes,
                max_depth,
            ).await?
        }
        A11yPerformAction {
            ref object_ref,
            action_name,
        } => crate::a11y::actions::perform_action(object_ref, action_name.as_deref()).await?,
        A11ySetValue {
            ref object_ref,
            ref value,
        } => crate::a11y::value::set_element_value(object_ref, value).await?,
        A11yGetElementText {
            ref object_ref,
            max_chars,
        } => crate::a11y::value::get_element_text(object_ref, max_chars).await?,
        A11yListApps { limit } => {
            serde_json::json!(crate::a11y::list_apps(limit).await?)
        }
        A11yDoctor => crate::a11y::setup::doctor_report().await,
        A11ySetupAccessibility => {
            let ok = crate::a11y::setup::enable_accessibility().await.unwrap_or(false);
            serde_json::json!({"ok": ok, "note": if ok { "accessibility enabled" } else { "may need logout/login" }})
        }
        A11yClickElementByRef { ref object_ref } => {
            crate::a11y::actions::click_element(object_ref).await?
        }

        // ─── UI automation (browser-side — not AT-SPI) ──

        UiTreeGet => {
            // AT-SPI tree via a11y module (for desktop UI, not browser DOM)
            crate::a11y::tree(Some(5)).await?
        }
        UiElementClick { ref selector, tab_index } => {
            crate::browser::click(tab_index, selector).await?
        }
        UiElementSetText {
            ref selector,
            ref text,
            tab_index,
        } => {
            crate::browser::set_text(tab_index, selector, text).await?
        }

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
        | Disconnect => unreachable!(
            "action reached execute_stubs but should have been handled earlier"
        ),

        _ => unreachable!("unexpected action in execute_stubs"),
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
