use anyhow::Context;
use zbus::zvariant::ObjectPath;
use zbus::{Connection, conn::Builder};

use super::util::{parse_states, role_name};

pub const DEST: &str = "org.a11y.atspi.Registry";
pub const ROOT: &str = "/org/a11y/atspi/accessible/root";

pub async fn connect_a11y() -> anyhow::Result<Connection> {
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
}

async fn get_str(conn: &Connection, path: &ObjectPath<'_>, prop: &str) -> String {
    conn.call_method(
        Some(DEST),
        path,
        Some("org.freedesktop.DBus.Properties"),
        "Get",
        &("org.a11y.atspi.Accessible", prop),
    )
    .await
    .ok()
    .and_then(|r| {
        let body = r.body();
        let val: zbus::zvariant::Value = body.deserialize().ok()?;
        val.try_into().ok()
    })
    .unwrap_or_default()
}

pub async fn get_i32(conn: &Connection, path: &ObjectPath<'_>, prop: &str) -> i32 {
    conn.call_method(
        Some(DEST),
        path,
        Some("org.freedesktop.DBus.Properties"),
        "Get",
        &("org.a11y.atspi.Accessible", prop),
    )
    .await
    .ok()
    .and_then(|r| {
        let body = r.body();
        let val: zbus::zvariant::Value = body.deserialize().ok()?;
        val.try_into().ok()
    })
    .unwrap_or(0)
}

async fn get_states(conn: &Connection, path: &ObjectPath<'_>) -> Vec<String> {
    conn.call_method(
        Some(DEST),
        path,
        Some("org.freedesktop.DBus.Properties"),
        "Get",
        &("org.a11y.atspi.Accessible", "State"),
    )
    .await
    .ok()
    .and_then(|r| {
        let body = r.body();
        let val: zbus::zvariant::Value = body.deserialize().ok()?;
        let bits: Vec<u32> = val.try_into().ok()?;
        Some(parse_states(&bits))
    })
    .unwrap_or_default()
}

pub async fn element_json(conn: &Connection, path: &ObjectPath<'_>) -> serde_json::Value {
    let name = get_str(conn, path, "Name").await;
    let role_id = get_i32(conn, path, "Role").await as u32;
    let description = get_str(conn, path, "Description").await;
    let child_count = get_i32(conn, path, "ChildCount").await;
    let states = get_states(conn, path).await;

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

pub async fn child_path(
    conn: &Connection,
    parent: &ObjectPath<'_>,
    index: i32,
) -> Option<ObjectPath<'static>> {
    let reply = conn
        .call_method(
            Some(DEST),
            parent,
            Some("org.a11y.atspi.Accessible"),
            "GetChildAtIndex",
            &(index,),
        )
        .await
        .ok()?;

    let body = reply.body();
    let (_, cp): (zbus::zvariant::OwnedValue, ObjectPath) = body.deserialize().ok()?;
    Some(cp.into_owned())
}
