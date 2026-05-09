pub mod backend;
pub mod capture;
pub mod cli;
pub mod client;
pub mod daemon;
pub mod protocol;

use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Global daemon state shared across all client connections
#[derive(Default)]
pub struct DaemonState {
    pub backend: Arc<RwLock<Option<Box<dyn backend::DesktopBackend>>>>,
}

impl DaemonState {
    pub fn new() -> Self {
        Self::default()
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
