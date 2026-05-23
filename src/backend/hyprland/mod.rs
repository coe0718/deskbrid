use crate::protocol;
use crate::protocol::DeskbridEvent;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::process::Command;
use tokio::sync::broadcast;

pub(crate) mod audio;
pub(crate) mod bluetooth;
pub(crate) mod clipboard;
pub(crate) mod core;
pub(crate) mod files;
pub(crate) mod free_functions;
pub(crate) mod helpers;
pub(crate) mod input;
pub(crate) mod keyboard_layout;
pub(crate) mod monitor;
pub(crate) mod networking;
pub(crate) mod notifications;
pub(crate) mod screenshot;
pub(crate) mod system_info;
pub(crate) mod trait_impl;
pub(crate) mod windows;
pub(crate) mod workspace;

use free_functions::*;

pub struct HyprBackend {
    /// Broadcast sender for push events to subscribed clients.
    pub(super) event_tx: broadcast::Sender<DeskbridEvent>,
    /// Active file watchers keyed by path.
    pub(super) watchers: Arc<Mutex<HashMap<String, notify::RecommendedWatcher>>>,
    /// Last known mouse position for relative delta calculation.
    pub(super) last_mouse: std::sync::Mutex<(f64, f64)>,
    /// Cached monitor info from hyprctl monitors.
    pub(super) monitors: std::sync::Mutex<Vec<protocol::MonitorInfo>>,
    /// Auto-detected Hyprland instance signature for IPC.
    pub(super) instance_sig: Option<String>,
    /// Auto-detected WAYLAND_DISPLAY value.
    pub(super) wl_socket: Option<String>,
    /// XDG_RUNTIME_DIR for Wayland client connections.
    pub(super) xdg_runtime: String,
}
