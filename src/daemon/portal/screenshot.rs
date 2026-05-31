//! Portal screenshot via org.freedesktop.portal.Screenshot.

use serde_json::{Value, json};
use zbus::Connection;

use super::helpers::{build_response_path, wait_for_portal_response};

const PORTAL_SERVICE: &str = "org.freedesktop.portal.Desktop";
const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
const SCREENSHOT_IFACE: &str = "org.freedesktop.portal.Screenshot";

/// Take a screenshot via the XDG Screenshot portal.
///
/// Calls the Screenshot method on the portal, then listens for the Response
/// signal on the returned handle path to obtain the URI of the captured image.
pub(crate) async fn portal_screenshot(interactive: bool) -> anyhow::Result<Value> {
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

    let response_path = build_response_path(&conn, &token).await?;
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
