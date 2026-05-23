pub mod a11y;
pub mod abs_pointer;
pub mod backend;
pub mod browser;
pub mod capture;
pub mod cli;
pub mod client;
pub mod color;
pub mod daemon;
pub mod mcp;
pub mod ocr;
pub mod permissions;
pub mod protocol;
pub mod setup;
pub mod tiling;
pub mod visual;

use permissions::Permissions;
use protocol::DeskbridEvent;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use tokio::process::Child;
use tokio::sync::{Mutex, RwLock, broadcast};

/// Global daemon state shared across all client connections
pub struct DaemonState {
    pub backend: Arc<RwLock<Option<Box<dyn backend::DesktopBackend>>>>,
    /// Broadcast channel for push events (file changes, etc.)
    pub event_tx: broadcast::Sender<DeskbridEvent>,
    /// Scoped permissions per UID
    pub permissions: Permissions,
    /// Active systemd-inhibit helper processes keyed by Deskbrid handle ID.
    pub inhibitors: Arc<Mutex<HashMap<u32, Child>>>,
    /// Active pseudo-terminal sessions keyed by Deskbrid terminal ID.
    pub terminals: Arc<Mutex<HashMap<String, daemon::terminal::TerminalSession>>>,
    /// Recent action audit entries, kept in memory as a bounded ring.
    pub audit_log: Arc<Mutex<VecDeque<protocol::AuditEntry>>>,
    pub audit_capacity: usize,
    pub action_timeout_ms: Option<u64>,
    pub(crate) rate_limits: Arc<Mutex<HashMap<u32, daemon::RateBucket>>>,
    pub(crate) rate_limit: Option<daemon::RateLimitConfig>,
    pub clipboard_history: Arc<Mutex<VecDeque<protocol::ClipboardHistoryEntry>>>,
    pub clipboard_history_capacity: usize,
    next_inhibitor_id: AtomicU32,
    next_terminal_id: AtomicU32,
    next_audit_id: AtomicU64,
    next_clipboard_history_id: AtomicU64,
}

impl DaemonState {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            backend: Arc::new(RwLock::new(None)),
            event_tx,
            permissions: Permissions::load(),
            inhibitors: Arc::new(Mutex::new(HashMap::new())),
            terminals: Arc::new(Mutex::new(HashMap::new())),
            audit_log: Arc::new(Mutex::new(VecDeque::new())),
            audit_capacity: daemon::audit_capacity_from_env(),
            action_timeout_ms: daemon::action_timeout_from_env(),
            rate_limits: Arc::new(Mutex::new(HashMap::new())),
            rate_limit: daemon::rate_limit_from_env(),
            clipboard_history: Arc::new(Mutex::new(VecDeque::new())),
            clipboard_history_capacity: daemon::clipboard_history_capacity_from_env(),
            next_inhibitor_id: AtomicU32::new(1),
            next_terminal_id: AtomicU32::new(1),
            next_audit_id: AtomicU64::new(1),
            next_clipboard_history_id: AtomicU64::new(1),
        }
    }

    pub fn next_inhibitor_id(&self) -> u32 {
        self.next_inhibitor_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn next_terminal_id(&self) -> String {
        format!(
            "term-{}",
            self.next_terminal_id.fetch_add(1, Ordering::Relaxed)
        )
    }

    pub fn next_audit_id(&self) -> u64 {
        self.next_audit_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn next_clipboard_history_id(&self) -> u64 {
        self.next_clipboard_history_id
            .fetch_add(1, Ordering::Relaxed)
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
