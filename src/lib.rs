pub mod backend;
pub mod capture;
pub mod cli;
pub mod client;
pub mod daemon;
pub mod protocol;

use protocol::DeskbridEvent;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

/// Global daemon state shared across all client connections
pub struct DaemonState {
    pub backend: Arc<RwLock<Option<Box<dyn backend::DesktopBackend>>>>,
    /// Broadcast channel for push events (file changes, etc.)
    pub event_tx: broadcast::Sender<DeskbridEvent>,
}

impl DaemonState {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            backend: Arc::new(RwLock::new(None)),
            event_tx,
        }
    }
}

impl Default for DaemonState {
    fn default() -> Self {
        Self::new()
    }
}

/// Per-client connection state
#[derive(Default)]
pub struct ConnectionState {
    /// Glob-pattern subscriptions (e.g., "window.*", "clipboard.changed")
    pub subscriptions: HashSet<String>,
    /// Registered hotkey IDs
    pub hotkeys: HashSet<String>,
    /// Watched file paths
    pub watched_paths: HashSet<String>,
}
