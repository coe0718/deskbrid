//! AT-SPI2 D-Bus query functions for accessibility tree data.
//!
//! These are called by the BFS tree builder to populate each node.

use super::super::bus;
use super::{AccessibilityAction, AccessibilityText, AccessibilityValue, Bounds};
use zbus::zvariant::ObjectPath;

pub(crate) async fn get_bounds(conn: &zbus::Connection, path: &ObjectPath<'_>) -> Option<Bounds> {
    let reply = conn
        .call_method(
            Some(bus::DEST),
            path,
            Some("org.a11y.atspi.Component"),
            "GetExtents",
            &(0u32),
        )
        .await
        .ok()?;

    let body = reply.body();
    let (x, y, width, height): (i32, i32, i32, i32) = body.deserialize().ok()?;

    Some(Bounds {
        x,
        y,
        width,
        height,
    })
}

pub(crate) async fn get_actions(
    conn: &zbus::Connection,
    path: &ObjectPath<'_>,
) -> Vec<AccessibilityAction> {
    let action_count: i32 = conn
        .call_method(
            Some(bus::DEST),
            path,
            Some("org.a11y.atspi.Action"),
            "GetActionCount",
            &(),
        )
        .await
        .ok()
        .and_then(|r| r.body().deserialize().ok())
        .unwrap_or(0);

    let mut actions = Vec::with_capacity(action_count as usize);
    for i in 0..action_count {
        let name: String = conn
            .call_method(
                Some(bus::DEST),
                path,
                Some("org.a11y.atspi.Action"),
                "GetName",
                &(i,),
            )
            .await
            .ok()
            .and_then(|r| r.body().deserialize().ok())
            .unwrap_or_default();
        let description: String = conn
            .call_method(
                Some(bus::DEST),
                path,
                Some("org.a11y.atspi.Action"),
                "GetDescription",
                &(i,),
            )
            .await
            .ok()
            .and_then(|r| r.body().deserialize().ok())
            .unwrap_or_default();
        actions.push(AccessibilityAction {
            index: i,
            name,
            description,
        });
    }
    actions
}

pub(crate) async fn get_value(
    conn: &zbus::Connection,
    path: &ObjectPath<'_>,
) -> Option<AccessibilityValue> {
    let current: f64 = conn
        .call_method(
            Some(bus::DEST),
            path,
            Some("org.a11y.atspi.Value"),
            "GetCurrentValue",
            &(),
        )
        .await
        .ok()
        .and_then(|r| r.body().deserialize().ok())?;

    let minimum: f64 = conn
        .call_method(
            Some(bus::DEST),
            path,
            Some("org.a11y.atspi.Value"),
            "GetMinimumValue",
            &(),
        )
        .await
        .ok()
        .and_then(|r| r.body().deserialize().ok())
        .unwrap_or(0.0);

    let maximum: f64 = conn
        .call_method(
            Some(bus::DEST),
            path,
            Some("org.a11y.atspi.Value"),
            "GetMaximumValue",
            &(),
        )
        .await
        .ok()
        .and_then(|r| r.body().deserialize().ok())
        .unwrap_or(0.0);

    Some(AccessibilityValue {
        current,
        minimum,
        maximum,
    })
}

pub(crate) async fn get_text(
    conn: &zbus::Connection,
    path: &ObjectPath<'_>,
    max_chars: i32,
) -> Option<AccessibilityText> {
    let char_count: i32 = conn
        .call_method(
            Some(bus::DEST),
            path,
            Some("org.a11y.atspi.Text"),
            "GetCharacterCount",
            &(),
        )
        .await
        .ok()
        .and_then(|r| r.body().deserialize().ok())?;

    if char_count == 0 {
        return Some(AccessibilityText {
            character_count: 0,
            caret_offset: 0,
            content: String::new(),
            selections: Vec::new(),
        });
    }

    let content: String = conn
        .call_method(
            Some(bus::DEST),
            path,
            Some("org.a11y.atspi.Text"),
            "GetText",
            &(0i32, char_count.min(max_chars)),
        )
        .await
        .ok()
        .and_then(|r| r.body().deserialize().ok())
        .unwrap_or_default();

    let caret: i32 = conn
        .call_method(
            Some(bus::DEST),
            path,
            Some("org.a11y.atspi.Text"),
            "GetCaretOffset",
            &(),
        )
        .await
        .ok()
        .and_then(|r| r.body().deserialize().ok())
        .unwrap_or(0);

    Some(AccessibilityText {
        character_count: char_count,
        caret_offset: caret,
        content,
        selections: Vec::new(),
    })
}

pub(crate) async fn check_editable(conn: &zbus::Connection, path: &ObjectPath<'_>) -> bool {
    conn.call_method(
        Some(bus::DEST),
        path,
        Some("org.a11y.atspi.EditableText"),
        "SetTextContents",
        &("test"),
    )
    .await
    .is_ok()
}
