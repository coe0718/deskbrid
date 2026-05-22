//! AT-SPI2 accessibility tree access for agent UI automation.
//!
//! Uses zbus (D-Bus) to query the accessibility tree, find elements by
//! role/name, click them, and read their text content. Supports full
//! snapshot building with bounds, actions, value, and text data.

pub mod actions;
pub(crate) mod bus;
pub mod setup;
pub mod tree;
mod util;
pub mod value;

use bus::{DEST, ROOT, child_path, connect_a11y, element_json, get_i32};
use serde_json::Value;
use std::collections::VecDeque;
use zbus::{Connection, zvariant::ObjectPath};

/// Build a tree of accessible elements up to given depth (BFS).
pub async fn tree(depth: Option<u32>) -> anyhow::Result<Value> {
    let conn = connect_a11y().await?;
    let max_depth = depth.unwrap_or(5).min(10) as usize;
    let root: ObjectPath = ObjectPath::try_from(ROOT)?;

    let mut elements = vec![element_json(&conn, &root).await];
    let mut queue: VecDeque<(ObjectPath<'static>, usize)> = VecDeque::new();
    queue.push_back((root.into_owned(), 0));

    while let Some((path, d)) = queue.pop_front() {
        if d >= max_depth {
            continue;
        }
        let cc = get_i32(&conn, &path, "ChildCount").await.min(50);
        for i in 0..cc {
            if let Some(cp) = child_path(&conn, &path, i).await {
                let mut info = element_json(&conn, &cp).await;
                info["depth"] = serde_json::json!(d + 1);
                elements.push(info);
                queue.push_back((cp, d + 1));
            }
        }
    }

    let count = elements.len();
    Ok(serde_json::json!({"elements": elements, "count": count}))
}

/// Find all elements matching role/name filters (BFS).
async fn find_all(
    conn: &Connection,
    role_filter: Option<&str>,
    name_filter: Option<&str>,
    max_depth: usize,
) -> anyhow::Result<Vec<(String, serde_json::Value)>> {
    let root: ObjectPath = ObjectPath::try_from(ROOT)?;
    let mut results = Vec::new();
    let mut queue: VecDeque<(ObjectPath<'static>, usize)> = VecDeque::new();
    queue.push_back((root.into_owned(), 0));

    while let Some((path, d)) = queue.pop_front() {
        let info = element_json(conn, &path).await;

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
            let cc = get_i32(conn, &path, "ChildCount").await.min(50);
            for i in 0..cc {
                if let Some(cp) = child_path(conn, &path, i).await {
                    queue.push_back((cp, d + 1));
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
    let conn = connect_a11y().await?;
    let results = find_all(&conn, role, name, 10).await?;

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
    let conn = connect_a11y().await?;
    let results = find_all(&conn, role, name, 10).await?;

    if results.is_empty() {
        anyhow::bail!("no element found matching role={role:?} name={name:?}");
    }

    let (path, info) = results
        .get(idx as usize)
        .ok_or_else(|| anyhow::anyhow!("index {idx} out of range ({} matches)", results.len()))?;

    let obj_path: ObjectPath = ObjectPath::try_from(path.as_str())?;

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
    let conn = connect_a11y().await?;
    let results = find_all(&conn, role, name, 10).await?;

    if results.is_empty() {
        anyhow::bail!("no element found matching role={role:?} name={name:?}");
    }

    let (path, info) = results
        .get(idx as usize)
        .ok_or_else(|| anyhow::anyhow!("index {idx} out of range ({} matches)", results.len()))?;

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
