//! Rule matching: trigger matching against events and condition evaluation.

use tracing::debug;

use crate::DaemonState;
use crate::protocol::{DeskbridEvent, EventTrigger, RuleCondition};

pub(crate) async fn resolve_event_app_id(
    mut event: DeskbridEvent,
    state: &DaemonState,
) -> DeskbridEvent {
    let window_id = match &event {
        DeskbridEvent::WindowFocused {
            window_id,
            app_id: None,
            ..
        } => Some(window_id.clone()),
        DeskbridEvent::WindowOpened {
            window_id,
            app_id: None,
            ..
        } => Some(window_id.clone()),
        DeskbridEvent::WindowClosed {
            window_id,
            app_id: None,
            ..
        } => Some(window_id.clone()),
        _ => None,
    };

    let Some(wid) = window_id else {
        return event; // already has app_id, or not a window event
    };

    // Resolve app_id from backend if available
    let app_id = {
        let backend_guard = state.backend.read().await;
        if let Some(ref backend) = *backend_guard {
            match backend.windows_list().await {
                Ok(windows) => windows
                    .iter()
                    .find(|w| w.id == wid)
                    .map(|w| w.app_id.clone()),
                Err(e) => {
                    debug!("resolve_event_app_id: windows_list() failed: {}", e);
                    None
                }
            }
        } else {
            None
        }
    };

    match &mut event {
        DeskbridEvent::WindowFocused { app_id: a, .. }
        | DeskbridEvent::WindowOpened { app_id: a, .. }
        | DeskbridEvent::WindowClosed { app_id: a, .. }
            if a.is_none() && app_id.is_some() =>
        {
            *a = app_id;
            debug!(
                "resolve_event_app_id: resolved app_id={:?} for window {}",
                a, wid
            );
        }
        _ => {}
    }

    event
}

/// Check whether a given EventTrigger matches a DeskbridEvent.
pub(crate) fn trigger_matches_event(trigger: &EventTrigger, event: &DeskbridEvent) -> bool {
    match trigger {
        EventTrigger::ClipboardChanged => {
            // No dedicated clipboard-changed event yet — reserved for future use.
            // TODO: emit ClipboardChanged from clipboard write path.
            false
        }
        EventTrigger::WindowOpened { app_id: filter } => {
            if let DeskbridEvent::WindowOpened {
                window_id: _,
                app_id: event_app_id,
                timestamp: _,
            } = event
            {
                match (filter, event_app_id) {
                    (None, _) => true,
                    (Some(f), Some(e)) => f == e,
                    (Some(_), None) => {
                        debug!(
                            "WindowOpened: trigger has app_id filter but event lacks app_id \
                             (backend doesn't emit it) — no match"
                        );
                        false
                    }
                }
            } else {
                false
            }
        }
        EventTrigger::WindowClosed { app_id: filter } => {
            if let DeskbridEvent::WindowClosed {
                window_id: _,
                app_id: event_app_id,
                timestamp: _,
            } = event
            {
                match (filter, event_app_id) {
                    (None, _) => true,
                    (Some(f), Some(e)) => f == e,
                    (Some(_), None) => {
                        debug!(
                            "WindowClosed: trigger has app_id filter but event lacks app_id \
                             (backend doesn't emit it) — no match"
                        );
                        false
                    }
                }
            } else {
                false
            }
        }
        EventTrigger::WindowFocused { app_id: filter } => {
            if let DeskbridEvent::WindowFocused {
                window_id: _,
                app_id: event_app_id,
                timestamp: _,
            } = event
            {
                match (filter, event_app_id) {
                    (None, _) => true,
                    (Some(f), Some(e)) => f == e,
                    (Some(_), None) => {
                        debug!(
                            "WindowFocused: trigger has app_id filter but event lacks app_id \
                             (backend doesn't provide it) — no match"
                        );
                        false
                    }
                }
            } else {
                false
            }
        }
        EventTrigger::SessionLocked
        | EventTrigger::SessionUnlocked
        | EventTrigger::IdleStarted
        | EventTrigger::IdleEnded => {
            // These triggers are reserved for future DeskbridEvent variants.
            false
        }
        EventTrigger::FileChanged { path } => match event {
            DeskbridEvent::FileCreated {
                path: ev_path,
                timestamp: _,
            }
            | DeskbridEvent::FileModified {
                path: ev_path,
                timestamp: _,
            }
            | DeskbridEvent::FileDeleted {
                path: ev_path,
                timestamp: _,
            } => ev_path.starts_with(path),
            _ => false,
        },
        EventTrigger::TimeRange {
            start_hour: _,
            end_hour: _,
            days: _,
        } => {
            // TimeRange triggers are evaluated on a timer, not per-event.
            false
        }
        EventTrigger::PresenceChanged { to: _ } => {
            // Reserved for future presence events.
            false
        }
    }
}

/// Evaluate a RuleCondition against the current state.
/// Returns true if the condition passes (or if there is no condition).
/// Called from within `evaluate()`, which holds no external locks.
pub(crate) fn condition_matches(
    condition: &Option<RuleCondition>,
    state: &DaemonState,
    _event: &DeskbridEvent,
) -> bool {
    let Some(cond) = condition else {
        return true; // no condition → always passes
    };

    match cond {
        RuleCondition::VarEquals { name, value } => {
            // Try to read session "default". If locked (rare), skip this tick.
            let Ok(sessions) = state.sessions.try_lock() else {
                debug!("condition_matches: sessions lock held, skipping");
                return false;
            };
            if let Some(session) = sessions.get("default") {
                session.vars.get(name).map(|v| v == value).unwrap_or(false)
            } else {
                false
            }
        }
        RuleCondition::VarExists { name } => {
            let Ok(sessions) = state.sessions.try_lock() else {
                return false;
            };
            if let Some(session) = sessions.get("default") {
                session.vars.contains_key(name)
            } else {
                false
            }
        }
    }
}
