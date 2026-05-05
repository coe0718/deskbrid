//! Event bus — in-process pub/sub for desktop events.
//!
//! The event bus connects DBus signal watchers to connected clients.
//! When a DBus watcher receives a signal, it pushes an event through the bus
//! which fans out to all subscribed client connections.

use tokio::sync::broadcast;

/// A single desktop event.
#[derive(Clone, Debug)]
pub struct DesktopEvent {
    pub event_type: String,
    pub data: serde_json::Value,
}

/// Event bus — broadcast channel for desktop events.
#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<DesktopEvent>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    /// Create a new event bus.
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(256);
        Self { tx }
    }

    /// Get a sender handle.
    pub fn sender(&self) -> broadcast::Sender<DesktopEvent> {
        self.tx.clone()
    }

    /// Get a receiver handle.
    pub fn subscribe(&self) -> broadcast::Receiver<DesktopEvent> {
        self.tx.subscribe()
    }

    /// Emit an event to all subscribers.
    pub fn emit(&self, event_type: impl Into<String>, data: serde_json::Value) {
        let _ = self.tx.send(DesktopEvent {
            event_type: event_type.into(),
            data,
        });
    }
}
