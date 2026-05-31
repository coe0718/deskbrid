//! Portal helpers — session management, response waiting, type conversion.

use serde_json::{Value, json};
use std::os::unix::io::AsRawFd;
use zbus::Connection;

const PORTAL_SERVICE: &str = "org.freedesktop.portal.Desktop";
const SCREENCAST_IFACE: &str = "org.freedesktop.portal.ScreenCast";

/// Create a ScreenCast session, returns the session handle object path.
pub(crate) async fn create_screencast_session(
    conn: &Connection,
    token: &str,
) -> anyhow::Result<String> {
    let mut options: std::collections::HashMap<&str, zbus::zvariant::Value<'_>> =
        std::collections::HashMap::new();
    options.insert("session_handle_token", zbus::zvariant::Value::new(token));

    let reply = conn
        .call_method(
            Some(PORTAL_SERVICE),
            "/org/freedesktop/portal/desktop",
            Some(SCREENCAST_IFACE),
            "CreateSession",
            &(options,),
        )
        .await?;

    let handle: zbus::zvariant::OwnedObjectPath = reply.body().deserialize()?;
    Ok(handle.to_string())
}

/// Select sources for the screencast session (monitor capture).
pub(crate) async fn select_screencast_sources(
    conn: &Connection,
    session_handle: &str,
    token: &str,
) -> anyhow::Result<()> {
    let mut options: std::collections::HashMap<&str, zbus::zvariant::Value<'_>> =
        std::collections::HashMap::new();
    options.insert("handle_token", zbus::zvariant::Value::new(token));
    options.insert("types", zbus::zvariant::Value::U32(1));
    options.insert("multiple", zbus::zvariant::Value::Bool(false));

    let _reply = conn
        .call_method(
            Some(PORTAL_SERVICE),
            session_handle,
            Some(SCREENCAST_IFACE),
            "SelectSources",
            &(options,),
        )
        .await?;

    Ok(())
}

/// Start the screencast session, returns the PipeWire fd and first stream node ID.
pub(crate) async fn start_screencast(
    conn: &Connection,
    session_handle: &str,
) -> anyhow::Result<(std::os::unix::io::OwnedFd, u32)> {
    let options: std::collections::HashMap<&str, zbus::zvariant::Value<'_>> =
        std::collections::HashMap::new();

    let reply = conn
        .call_method(
            Some(PORTAL_SERVICE),
            session_handle,
            Some(SCREENCAST_IFACE),
            "Start",
            &("", options),
        )
        .await?;

    let body = reply.body();
    let pw_fd: zbus::zvariant::OwnedFd = body.deserialize()?;
    let streams: Vec<(u32, u32)> = body.deserialize()?;
    let stream_node_id = streams.first().map(|(node_id, _)| *node_id).unwrap_or(0);

    Ok((pw_fd.into(), stream_node_id))
}

/// Build the response path for the given token.
pub(crate) async fn build_response_path(conn: &Connection, token: &str) -> anyhow::Result<String> {
    let sender = conn
        .unique_name()
        .map(|n| n.as_str().replace('.', "_"))
        .unwrap_or_default();
    Ok(format!(
        "/org/freedesktop/portal/desktop/request/{sender}/{token}"
    ))
}

/// Wait for a portal Response signal on the given object path.
///
/// Portal Response signals have signature (u, a{sv}): (response_code, results).
/// Response code 0 = success, 1 = cancelled by user, 2 = error.
pub(crate) async fn wait_for_portal_response(
    conn: &Connection,
    expected_path: &str,
) -> anyhow::Result<(u32, std::collections::HashMap<String, Value>)> {
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

            let json_results: std::collections::HashMap<String, Value> = results
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
fn owned_value_to_json(value: &zbus::zvariant::OwnedValue) -> Value {
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
