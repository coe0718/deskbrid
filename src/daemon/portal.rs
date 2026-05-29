// TESTING_NEEDED: This feature requires manual testing on a live desktop environment
//! XDG Desktop Portal integration for screenshots and screencasting.
//!
//! Talks to org.freedesktop.portal.Screenshot via zbus on the session bus.
//! Uses the portal's request/response pattern: call Screenshot → get a handle →
//! listen for the Response signal → parse the URI.

use serde_json::{Value, json};
use zbus::Connection;

const PORTAL_SERVICE: &str = "org.freedesktop.portal.Desktop";
const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
const SCREENSHOT_IFACE: &str = "org.freedesktop.portal.Screenshot";

/// Take a screenshot via the XDG Screenshot portal.
///
/// Calls the Screenshot method on the portal, then listens for the Response
/// signal on the returned handle path to obtain the URI of the captured image.
pub async fn portal_screenshot(interactive: bool) -> anyhow::Result<Value> {
    let conn = Connection::session().await?;

    let token = format!("deskbrid_{}", std::process::id());
    let handle_token = zbus::zvariant::Value::new(token.as_str());

    let mut options: std::collections::HashMap<&str, zbus::zvariant::Value<'_>> =
        std::collections::HashMap::new();
    options.insert("handle_token", handle_token);
    options.insert("interactive", zbus::zvariant::Value::Bool(interactive));

    let reply = conn
        .call_method(
            Some(PORTAL_SERVICE),
            PORTAL_PATH,
            Some(SCREENSHOT_IFACE),
            "Screenshot",
            &("", options),
        )
        .await?;

    let _handle_path: zbus::zvariant::OwnedObjectPath = reply.body().deserialize()?;

    let sender = conn
        .unique_name()
        .map(|n| n.as_str().replace('.', "_"))
        .unwrap_or_default();
    let response_path = format!("/org/freedesktop/portal/desktop/request/{sender}/{token}");

    let result = wait_for_portal_response(&conn, &response_path).await?;

    if result.0 != 0 {
        anyhow::bail!(
            "portal screenshot request was cancelled or failed (response={})",
            result.0
        );
    }

    let uri = result
        .1
        .get("uri")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    Ok(json!({
        "ok": true,
        "method": "xdg_portal_screenshot",
        "uri": uri,
        "interactive": interactive,
    }))
}

/// Start a screencast session via the XDG ScreenCast portal.
///
/// NOTE: Placeholder — full ScreenCast portal support requires PipeWire stream handling.
pub async fn portal_screencast_start(output_path: &str) -> anyhow::Result<Value> {
    let _ = output_path;
    Ok(json!({
        "ok": false,
        "method": "xdg_portal_screencast",
        "message": "Portal screencast requires PipeWire stream handling (not yet implemented)"
    }))
}

/// Stop a running portal screencast.
pub async fn portal_screencast_stop() -> anyhow::Result<Value> {
    Ok(json!({
        "ok": true,
        "message": "Portal screencast stopped"
    }))
}

/// Wait for a portal Response signal on the given object path.
///
/// Portal Response signals have signature (u, a{sv}): (response_code, results).
/// Response code 0 = success, 1 = cancelled by user, 2 = error.
async fn wait_for_portal_response(
    conn: &Connection,
    expected_path: &str,
) -> anyhow::Result<(u32, std::collections::HashMap<String, serde_json::Value>)> {
    let expected = expected_path.to_string();
    let mut stream = zbus::MessageStream::from(conn.clone());

    let result = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        use futures_util::StreamExt;
        while let Some(msg) = stream.next().await {
            let Ok(msg) = msg else { continue };
            let header = msg.header();
            if header.message_type() != zbus::message::Type::Signal {
                continue;
            }
            let Some(iface) = header.interface() else {
                continue;
            };
            if iface.as_str() != "org.freedesktop.portal.Request" {
                continue;
            }
            let Some(member) = header.member() else {
                continue;
            };
            if member.as_str() != "Response" {
                continue;
            }
            let Some(path) = header.path() else {
                continue;
            };
            if path.as_str() != expected {
                continue;
            }
            return Some(msg);
        }
        None
    })
    .await;

    match result {
        Ok(Some(msg)) => {
            let body = msg.body();
            let response_code: u32 = body.deserialize()?;
            let results: std::collections::HashMap<String, zbus::zvariant::OwnedValue> =
                body.deserialize()?;

            let json_results: std::collections::HashMap<String, serde_json::Value> = results
                .into_iter()
                .map(|(k, v)| (k, owned_value_to_json(&v)))
                .collect();

            Ok((response_code, json_results))
        }
        Ok(None) => anyhow::bail!("portal response stream ended unexpectedly"),
        Err(_) => anyhow::bail!("portal response timed out after 30 seconds"),
    }
}

/// Convert a zvariant OwnedValue to a serde_json Value (best effort).
fn owned_value_to_json(value: &zbus::zvariant::OwnedValue) -> serde_json::Value {
    match value.value_signature().to_string().as_str() {
        "s" => value
            .downcast_ref::<String>()
            .map(|s| json!(s.as_str()))
            .unwrap_or(json!(null)),
        "b" => value
            .downcast_ref::<bool>()
            .map(|b| json!(b))
            .unwrap_or(json!(null)),
        "u" => value
            .downcast_ref::<u32>()
            .map(|u| json!(u))
            .unwrap_or(json!(null)),
        "i" => value
            .downcast_ref::<i32>()
            .map(|i| json!(i))
            .unwrap_or(json!(null)),
        "d" => value
            .downcast_ref::<f64>()
            .map(|d| json!(d))
            .unwrap_or(json!(null)),
        _ => json!(null),
    }
}
