use crate::protocol::DeskbridEvent;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

pub(crate) mod audio;
pub(crate) mod bluetooth;
pub(crate) mod clipboard;
pub(crate) mod core;
pub(crate) mod files;
pub(crate) mod init;
pub(crate) mod inner;
pub(crate) mod input;
pub(crate) mod keysym;
pub(crate) mod monitor;
pub(crate) mod network;
pub(crate) mod notifications;
pub(crate) mod parsers;
pub(crate) mod screenshot;
pub(crate) mod system;
pub(crate) mod trait_impl;
pub(crate) mod windows;
pub(crate) mod workspace;

pub struct GnomeBackend {
    pub(super) conn: zbus::Connection,
    pub(super) event_tx: broadcast::Sender<DeskbridEvent>,
    pub(super) watchers: Arc<Mutex<HashMap<String, notify::RecommendedWatcher>>>,
    pub(super) rd_session_path: String,
    pub(super) sc_stream_path: String,
    pub(super) last_mouse: std::sync::Mutex<(f64, f64)>,
}
