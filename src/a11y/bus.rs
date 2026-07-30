use anyhow::Context;
use tokio::sync::OnceCell;
use zbus::zvariant::ObjectPath;
use zbus::{Connection, conn::Builder};

use super::util::{parse_states, role_name};

pub(crate) const DEST: &str = "org.a11y.atspi.Registry";
pub const ROOT: &str = "/org/a11y/atspi/accessible/root";

/// Encode the destination and object path into the opaque reference returned to
/// clients. `|` is not valid in either a D-Bus bus name or object path.
pub fn encode_object_ref(dest: &str, path: &ObjectPath<'_>) -> String {
    format!("{dest}|{path}")
}

/// Decode a reference produced by `encode_object_ref`. Bare object paths from
/// older clients remain valid and target the AT-SPI registry destination.
pub fn decode_object_ref(reference: &str) -> anyhow::Result<(String, ObjectPath<'static>)> {
    let (dest, path) = reference.split_once('|').unwrap_or((DEST, reference));
    if dest.is_empty() {
        anyhow::bail!("AT-SPI object reference has an empty bus name");
    }
    zbus::names::BusName::try_from(dest)
        .map_err(|error| anyhow::anyhow!("invalid AT-SPI bus name: {error}"))?;
    let path = ObjectPath::try_from(path)
        .map_err(|error| anyhow::anyhow!("invalid AT-SPI object path: {error}"))?
        .into_owned();
    Ok((dest.to_string(), path))
}

/// Cached AT-SPI2 connection — created once, cloned cheaply thereafter.
static A11Y_CONN: OnceCell<Connection> = OnceCell::const_new();

pub async fn connect_a11y() -> anyhow::Result<Connection> {
    let conn = A11Y_CONN
        .get_or_try_init(|| async {
            let session = Connection::session()
                .await
                .context("D-Bus session bus unavailable")?;

            let addr: String = session
                .call_method(
                    Some("org.a11y.Bus"),
                    "/org/a11y/bus",
                    Some("org.a11y.Bus"),
                    "GetAddress",
                    &(),
                )
                .await
                .context("AT-SPI2 bus not available - is accessibility enabled?")?
                .body()
                .deserialize()?;

            Builder::address(addr.as_str())?
                .build()
                .await
                .context("failed to connect to AT-SPI2 bus")
        })
        .await?;
    Ok(conn.clone())
}

async fn get_str(conn: &Connection, dest: &str, path: &ObjectPath<'_>, prop: &str) -> String {
    conn.call_method(
        Some(dest),
        path,
        Some("org.freedesktop.DBus.Properties"),
        "Get",
        &("org.a11y.atspi.Accessible", prop),
    )
    .await
    .ok()
    .and_then(|r| {
        let body = r.body();
        // Compliant peers variant-wrap the value; some at-spi2-atk adaptors
        // return the bare value. Try 'v' first, then the raw payload.
        if let Ok(val) = body.deserialize::<zbus::zvariant::Value>() {
            let s: String = (&val).try_into().ok()?;
            return Some(s);
        }
        body.deserialize::<String>().ok()
    })
    .unwrap_or_default()
}

pub async fn get_i32(conn: &Connection, dest: &str, path: &ObjectPath<'_>, prop: &str) -> i32 {
    conn.call_method(
        Some(dest),
        path,
        Some("org.freedesktop.DBus.Properties"),
        "Get",
        &("org.a11y.atspi.Accessible", prop),
    )
    .await
    .ok()
    .and_then(|r| {
        let body = r.body();
        // AT-SPI exposes some int properties as 'u' (e.g. Role) and others as
        // 'i' (e.g. ChildCount); zvariant's TryInto is strict per-type.
        if let Ok(val) = body.deserialize::<zbus::zvariant::Value>() {
            return match &val {
                zbus::zvariant::Value::I32(v) => Some(*v),
                zbus::zvariant::Value::U32(v) => i32::try_from(*v).ok(),
                zbus::zvariant::Value::I64(v) => i32::try_from(*v).ok(),
                zbus::zvariant::Value::U64(v) => i32::try_from(*v).ok(),
                _ => None,
            };
        }
        // Non-compliant peers: bare int payload instead of 'v'.
        if let Ok(v) = body.deserialize::<i32>() {
            return Some(v);
        }
        body.deserialize::<u32>()
            .ok()
            .and_then(|v| i32::try_from(v).ok())
    })
    .unwrap_or(0)
}

async fn get_states(conn: &Connection, dest: &str, path: &ObjectPath<'_>) -> Vec<String> {
    conn.call_method(
        Some(dest),
        path,
        Some("org.freedesktop.DBus.Properties"),
        "Get",
        &("org.a11y.atspi.Accessible", "State"),
    )
    .await
    .ok()
    .and_then(|r| {
        let body = r.body();
        if let Ok(val) = body.deserialize::<zbus::zvariant::Value>() {
            let bits: Vec<u32> = val.try_into().ok()?;
            return Some(parse_states(&bits));
        }
        body.deserialize::<Vec<u32>>()
            .ok()
            .map(|bits| parse_states(&bits))
    })
    .unwrap_or_default()
}

pub async fn element_json(
    conn: &Connection,
    dest: &str,
    path: &ObjectPath<'_>,
) -> serde_json::Value {
    let name = get_str(conn, dest, path, "Name").await;
    let role_id = get_i32(conn, dest, path, "Role").await as u32;
    let description = get_str(conn, dest, path, "Description").await;
    let child_count = get_i32(conn, dest, path, "ChildCount").await;
    let states = get_states(conn, dest, path).await;

    serde_json::json!({
        "name": name,
        "role": role_name(role_id),
        "role_id": role_id,
        "description": description,
        "child_count": child_count,
        "states": states,
        "path": path.as_str(),
    })
}

/// Resolve a child reference. AT-SPI returns `(so)` — the bus name owning the
/// object plus its path on that connection. Callers MUST use the returned bus
/// name as the destination for all subsequent calls on the child: accessible
/// objects live on each application's own connection, not on the registry.
pub async fn child_path(
    conn: &Connection,
    dest: &str,
    parent: &ObjectPath<'_>,
    index: i32,
) -> Option<(String, ObjectPath<'static>)> {
    let reply = conn
        .call_method(
            Some(dest),
            parent,
            Some("org.a11y.atspi.Accessible"),
            "GetChildAtIndex",
            &(index,),
        )
        .await
        .ok()?;

    let body = reply.body();
    let (bus_name, cp): (String, ObjectPath) = body.deserialize().ok()?;
    Some((bus_name, cp.into_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_ref_round_trips_application_destination() {
        let path = ObjectPath::try_from("/org/a11y/atspi/accessible/42").unwrap();
        let encoded = encode_object_ref(":1.234", &path);
        let (dest, decoded) = decode_object_ref(&encoded).unwrap();

        assert_eq!(dest, ":1.234");
        assert_eq!(decoded.as_str(), path.as_str());
    }

    #[test]
    fn legacy_bare_object_ref_targets_registry() {
        let (dest, path) = decode_object_ref(ROOT).unwrap();

        assert_eq!(dest, DEST);
        assert_eq!(path.as_str(), ROOT);
    }
}
