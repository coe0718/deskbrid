pub mod backend;
pub mod capture;
pub mod cli;
pub mod client;
pub mod daemon;
pub mod protocol;

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
