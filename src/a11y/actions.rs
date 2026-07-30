//! AT-SPI2 action invocation: click, activate, focus.

use serde_json::json;

use super::bus;

pub async fn perform_action(
    object_ref: &str,
    action_name: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    let conn = bus::connect_a11y().await?;
    let (dest, obj_path) = bus::decode_object_ref(object_ref)?;

    let action_count: i32 = conn
        .call_method(
            Some(dest.as_str()),
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
        anyhow::bail!("element has no actions");
    }

    let (action_index, matched_name) = if let Some(name) = action_name {
        let mut found = None;
        for i in 0..action_count {
            let an: String = conn
                .call_method(
                    Some(dest.as_str()),
                    &obj_path,
                    Some("org.a11y.atspi.Action"),
                    "GetName",
                    &(i,),
                )
                .await
                .ok()
                .and_then(|r| r.body().deserialize().ok())
                .unwrap_or_default();
            if an.eq_ignore_ascii_case(name) {
                found = Some((i, an));
                break;
            }
        }
        found.unwrap_or((0, "".into()))
    } else {
        (0, "".into())
    };

    let success: bool = conn
        .call_method(
            Some(dest.as_str()),
            &obj_path,
            Some("org.a11y.atspi.Action"),
            "DoAction",
            &(action_index,),
        )
        .await
        .ok()
        .and_then(|r| r.body().deserialize().ok())
        .unwrap_or(false);

    Ok(json!({
        "ok": success,
        "action_index": action_index,
        "action_name": if matched_name.is_empty() { None } else { Some(matched_name) }
    }))
}

/// Click an element — tries Action interface first, falls back to coordinate click.
pub async fn click_element(object_ref: &str) -> anyhow::Result<serde_json::Value> {
    // Try AT-SPI Action first
    if let Ok(result) = perform_action(object_ref, Some("click")).await
        && result["ok"].as_bool().unwrap_or(false)
    {
        return Ok(result);
    }
    if let Ok(result) = perform_action(object_ref, Some("activate")).await
        && result["ok"].as_bool().unwrap_or(false)
    {
        return Ok(result);
    }

    // Fallback: coordinate click via bounds
    let conn = bus::connect_a11y().await?;
    let (dest, obj_path) = bus::decode_object_ref(object_ref)?;
    let bounds = super::tree::get_bounds(&conn, &dest, &obj_path).await;

    if let Some(b) = bounds {
        let x = b.x + b.width / 2;
        let y = b.y + b.height / 2;
        Ok(json!({
            "ok": true,
            "method": "coordinate_fallback",
            "x": x,
            "y": y,
            "action_index": -1,
            "note": "AT-SPI action failed; coordinate fallback provided for client-side click"
        }))
    } else {
        anyhow::bail!("no actions and no bounds available for coordinate fallback")
    }
}
