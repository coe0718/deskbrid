//! AT-SPI2 accessibility tree access for agent UI automation.
//!
//! Uses zbus (D-Bus) to query the accessibility tree, find elements by
//! role/name, click them, and read their text content.

use anyhow::Context;
use serde_json::Value;
use zbus::zvariant::ObjectPath;
use zbus::{Connection, conn::Builder};

/// Map AT-SPI2 role IDs to human-readable names.
fn role_name(id: u32) -> String {
    match id {
        0 => "invalid",
        1 => "alert",
        4 => "check_box",
        7 => "combo_box",
        11 => "dialog",
        17 => "frame",
        24 => "label",
        26 => "list",
        27 => "list_item",
        28 => "menu",
        29 => "menu_bar",
        30 => "menu_item",
        34 => "panel",
        35 => "password_text",
        38 => "push_button",
        39 => "radio_button",
        44 => "scroll_pane",
        50 => "table",
        51 => "table_cell",
        55 => "terminal",
        56 => "text",
        57 => "toggle_button",
        58 => "tool_bar",
        64 => "window",
        70 => "application",
        74 => "entry",
        94 => "grouping",
        _ => "unknown",
    }
    .into()
}

/// Parse AT-SPI2 state bitflags into human-readable strings.
fn parse_states(bits: &[u32]) -> Vec<String> {
    let names = [
        "active",
        "armed",
        "busy",
        "checked",
        "collapsed",
        "defunct",
        "editable",
        "enabled",
        "expandable",
        "expanded",
        "focusable",
        "focused",
        "has_tooltip",
        "horizontal",
        "iconified",
        "modal",
        "multi_line",
        "multiselectable",
        "opaque",
        "pressed",
        "resizable",
        "selectable",
        "selected",
        "sensitive",
        "showing",
        "single_line",
        "stale",
        "transient",
        "vertical",
        "visible",
        "manages_descendants",
        "indeterminate",
        "required",
        "truncated",
        "animated",
        "invalid_entry",
        "supports_autocompletion",
        "selectable_text",
        "is_default",
        "visited",
        "checkable",
        "has_popup",
        "read_only",
    ];
    let mut states = Vec::new();
    for (i, name) in names.iter().enumerate() {
        let word = i / 32;
        let bit = i % 32;
        if let Some(mask) = bits.get(word)
            && mask & (1u32 << bit) != 0
        {
            states.push((*name).to_string());
        }
    }
    states
}

const DEST: &str = "org.a11y.atspi.Registry";
const ROOT: &str = "/org/a11y/atspi/accessible/root";

/// Connect to the AT-SPI2 bus.
async fn connect_a11y() -> anyhow::Result<Connection> {
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
        .context("AT-SPI2 bus not available — is accessibility enabled?")?
        .body()
        .deserialize()?;

    Builder::address(addr.as_str())?
        .build()
        .await
        .context("failed to connect to AT-SPI2 bus")
}

/// Get a string property from an accessible object.
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

/// Get an i32 property from an accessible object.
async fn get_i32(conn: &Connection, path: &ObjectPath<'_>, prop: &str) -> i32 {
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

/// Get state bits from an accessible object.
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

/// Get element info as JSON.
async fn element_json(conn: &Connection, path: &ObjectPath<'_>) -> serde_json::Value {
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

/// Get a child's object path by index.
async fn child_path(
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

    // AT-SPI2 returns (so, path) — Accessible object reference
    let body = reply.body();
    let (_, cp): (zbus::zvariant::OwnedValue, ObjectPath) = body.deserialize().ok()?;
    Some(cp.into_owned())
}

/// Build a tree of accessible elements up to given depth (BFS).
pub async fn tree(depth: Option<u32>) -> anyhow::Result<Value> {
    let conn = connect_a11y().await?;
    let max_depth = depth.unwrap_or(5).min(10) as usize;
    let root: ObjectPath = ObjectPath::try_from(ROOT)?;

    let mut elements = vec![element_json(&conn, &root).await];
    let mut queue: Vec<(ObjectPath<'static>, usize)> = vec![(root.into_owned(), 0)];

    while let Some((path, d)) = queue.pop() {
        if d >= max_depth {
            continue;
        }
        let cc = get_i32(&conn, &path, "ChildCount").await.min(50);
        for i in 0..cc {
            if let Some(cp) = child_path(&conn, &path, i).await {
                let mut info = element_json(&conn, &cp).await;
                info["depth"] = serde_json::json!(d + 1);
                elements.push(info);
                queue.push((cp, d + 1));
            }
        }
    }

    Ok(serde_json::json!({"elements": elements, "count": elements.len()}))
}

/// Find all elements matching role/name filters (BFS).
async fn find_all(
    role_filter: Option<&str>,
    name_filter: Option<&str>,
    max_depth: usize,
) -> anyhow::Result<Vec<(String, serde_json::Value)>> {
    let conn = connect_a11y().await?;
    let root: ObjectPath = ObjectPath::try_from(ROOT)?;
    let mut results = Vec::new();
    let mut queue: Vec<(ObjectPath<'static>, usize)> = vec![(root.into_owned(), 0)];

    while let Some((path, d)) = queue.pop() {
        let info = element_json(&conn, &path).await;

        let role_ok = role_filter.is_none_or(|r| {
            info["role"]
                .as_str()
                .is_some_and(|v| v.eq_ignore_ascii_case(r))
        });
        let name_ok = name_filter.is_none_or(|n| {
            info["name"]
                .as_str()
                .is_some_and(|v| v.to_lowercase().contains(&n.to_lowercase()))
        });

        if role_ok && name_ok {
            results.push((path.to_string(), info));
        }

        if d < max_depth {
            let cc = get_i32(&conn, &path, "ChildCount").await.min(50);
            for i in 0..cc {
                if let Some(cp) = child_path(&conn, &path, i).await {
                    queue.push((cp, d + 1));
                }
            }
        }
    }

    Ok(results)
}

/// Get info about a specific element found by role/name.
pub async fn get_element(
    role: Option<&str>,
    name: Option<&str>,
    index: Option<u32>,
) -> anyhow::Result<Value> {
    let idx = index.unwrap_or(0);
    let results = find_all(role, name, 10).await?;

    if results.is_empty() {
        anyhow::bail!("no element found matching role={role:?} name={name:?}");
    }

    let (path, info) = results
        .get(idx as usize)
        .ok_or_else(|| anyhow::anyhow!("index {idx} out of range ({} matches)", results.len()))?;

    let mut result = info.clone();
    result["path"] = serde_json::json!(path);
    Ok(result)
}

/// Click an element via AT-SPI2 Action interface.
pub async fn click_element(
    role: Option<&str>,
    name: Option<&str>,
    index: Option<u32>,
) -> anyhow::Result<Value> {
    let idx = index.unwrap_or(0);
    let results = find_all(role, name, 10).await?;

    if results.is_empty() {
        anyhow::bail!("no element found matching role={role:?} name={name:?}");
    }

    let (path, info) = results
        .get(idx as usize)
        .ok_or_else(|| anyhow::anyhow!("index {idx} out of range ({} matches)", results.len()))?;

    let conn = connect_a11y().await?;
    let obj_path: ObjectPath = ObjectPath::try_from(path.as_str())?;

    // Get action count
    let action_count: i32 = conn
        .call_method(
            Some(DEST),
            &obj_path,
            Some("org.a11y.atspi.Action"),
            "GetActionCount",
            &(),
        )
        .await
        .ok()
        .and_then(|r| r.body().deserialize().ok())
        .unwrap_or(0);

    if action_count == 0 {
        anyhow::bail!(
            "element '{}' ({}) has no actions",
            info["name"],
            info["role"]
        );
    }

    let action_name: String = conn
        .call_method(
            Some(DEST),
            &obj_path,
            Some("org.a11y.atspi.Action"),
            "GetName",
            &(0i32,),
        )
        .await
        .ok()
        .and_then(|r| r.body().deserialize().ok())
        .unwrap_or_default();

    let clicked: bool = conn
        .call_method(
            Some(DEST),
            &obj_path,
            Some("org.a11y.atspi.Action"),
            "DoAction",
            &(0i32,),
        )
        .await
        .ok()
        .and_then(|r| r.body().deserialize().ok())
        .unwrap_or(false);

    Ok(serde_json::json!({
        "clicked": true,
        "element": {"name": info["name"], "role": info["role"], "path": path},
        "action": action_name,
        "success": clicked,
    }))
}

/// Get text content from an element via AT-SPI2 Text interface.
pub async fn get_text(
    role: Option<&str>,
    name: Option<&str>,
    index: Option<u32>,
) -> anyhow::Result<Value> {
    let idx = index.unwrap_or(0);
    let results = find_all(role, name, 10).await?;

    if results.is_empty() {
        anyhow::bail!("no element found matching role={role:?} name={name:?}");
    }

    let (path, info) = results
        .get(idx as usize)
        .ok_or_else(|| anyhow::anyhow!("index {idx} out of range ({} matches)", results.len()))?;

    let conn = connect_a11y().await?;
    let obj_path: ObjectPath = ObjectPath::try_from(path.as_str())?;

    let char_count: i32 = conn
        .call_method(
            Some(DEST),
            &obj_path,
            Some("org.a11y.atspi.Text"),
            "GetCharacterCount",
            &(),
        )
        .await
        .ok()
        .and_then(|r| r.body().deserialize().ok())
        .unwrap_or(0);

    let text: String = conn
        .call_method(
            Some(DEST),
            &obj_path,
            Some("org.a11y.atspi.Text"),
            "GetText",
            &(0i32, char_count),
        )
        .await
        .ok()
        .and_then(|r| r.body().deserialize().ok())
        .unwrap_or_default();

    Ok(serde_json::json!({
        "text": text,
        "character_count": char_count,
        "element": {"name": info["name"], "role": info["role"], "path": path},
    }))
}
