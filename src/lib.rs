pub mod a11y;
pub mod abs_pointer;
pub mod backend;
pub mod browser;
pub mod capture;
pub mod cli;
pub mod client;
pub mod cmd;
pub mod color;
pub mod daemon;
pub mod mcp;
pub mod ocr;
pub mod permissions;
pub mod protocol;
pub mod setup;
pub mod tiling;
pub mod tray;
pub mod util;
pub mod visual;

use dashmap::DashMap;
use permissions::Permissions;
use protocol::DeskbridEvent;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use tokio::process::Child;
use tokio::sync::{Mutex, RwLock, broadcast};
use tracing::{info, warn};

use crate::daemon::persistence::Database;
use crate::daemon::rules::RuleEngine;

/// Session-scoped data for named sessions (#31).
/// Each session isolates variables and metadata per connecting agent.
#[derive(Debug, Clone)]
pub struct SessionData {
    pub name: String,
    pub vars: HashMap<String, String>,
    pub created_at: u64,
    pub last_active: u64,
}

impl SessionData {
    fn new(name: String) -> Self {
        let now = unix_timestamp();
        Self {
            name,
            vars: HashMap::new(),
            created_at: now,
            last_active: now,
        }
    }

    pub fn touch(&mut self) {
        self.last_active = unix_timestamp();
    }
}

fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Global daemon state shared across all client connections
///
/// ## Lock ordering
/// When multiple locks MUST be held simultaneously, acquire in this order:
/// 1. `backend` (RwLock) — always first; shortest-held
/// 2. `database` (tokio Mutex)
/// 3. `rules` (Mutex)
/// 4. `screencast_process` (Mutex)
/// 5. `recording` (Mutex)
///
/// `audit_log`, `clipboard_history` — standalone, never held with other locks.
/// DashMap fields (`inhibitors`, `terminals`, `rate_limits`, `sessions`,
/// `pending_confirmations`) are lock-free sharded maps — no ordering needed.
/// `rate_limit_store`, `schedule`, `agent_mailbox`, `search_index` —
/// internally synchronized, never held with other DaemonState locks.
pub struct DaemonState {
    pub backend: Arc<RwLock<Option<Box<dyn backend::DesktopBackend>>>>,
    /// Broadcast channel for push events (file changes, etc.)
    pub event_tx: broadcast::Sender<DeskbridEvent>,
    /// Scoped permissions per UID
    pub permissions: Permissions,
    /// Active systemd-inhibit helper processes keyed by Deskbrid handle ID.
    pub inhibitors: DashMap<u32, Child>,
    /// Active pseudo-terminal sessions keyed by Deskbrid terminal ID.
    pub terminals: DashMap<String, daemon::terminal::TerminalSession>,
    /// Recent action audit entries, kept in memory as a bounded ring.
    pub audit_log: Arc<Mutex<VecDeque<protocol::AuditEntry>>>,
    pub audit_capacity: usize,
    pub action_timeout_ms: Option<u64>,
    pub(crate) rate_limits: DashMap<u32, daemon::RateBucket>,
    pub(crate) rate_limit: Option<daemon::RateLimitConfig>,
    /// Per-namespace, per-UID rate limiting (#129)
    pub rate_limit_store: Arc<daemon::RateLimitStore>,
    pub clipboard_history: Arc<Mutex<VecDeque<protocol::ClipboardHistoryEntry>>>,
    pub clipboard_history_capacity: usize,
    pub schedule: Arc<daemon::schedule::ScheduleState>,
    pub recording: Arc<Mutex<Option<daemon::macro_engine::ActiveRecording>>>,
    pub database: Arc<tokio::sync::Mutex<Database>>,
    /// Named sessions — map of session name to session data (#31)
    pub sessions: DashMap<String, SessionData>,
    /// Rules engine state — registered rules with runtime tracking (#83)
    pub rules: Arc<Mutex<RuleEngine>>,
    /// Active portal screencast process (GStreamer pipeline)
    pub screencast_process: Arc<Mutex<Option<daemon::portal::ActiveScreencast>>>,
    pub pending_confirmations: DashMap<String, daemon::execute_confirmation::PendingConfirmation>,
    pub agent_mailbox: Arc<daemon::execute_agent::AgentMailboxStore>,
    pub search_index: Arc<daemon::search::SearchIndex>,
    next_confirmation_id: AtomicU64,
    next_inhibitor_id: AtomicU32,
    next_terminal_id: AtomicU32,
    next_audit_id: AtomicU64,
    next_clipboard_history_id: AtomicU64,
}

impl DaemonState {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(256);

        // Attempt to open the persistent SQLite database.
        // If it fails, fall back to an in-memory DB so the daemon can still start.
        let database = match Database::open() {
            Ok(db) => {
                info!("SQLite persistence layer initialized");
                db
            }
            Err(e) => {
                warn!("Failed to open on-disk SQLite database (falling back to in-memory): {e}");
                Database::memory().expect("in-memory SQLite fallback must succeed")
            }
        };

        // Load persisted sessions from DB
        let sessions: HashMap<String, SessionData> = {
            let mut map = HashMap::new();
            if let Ok(persisted) = database.load_sessions() {
                for s in persisted {
                    map.insert(s.name.clone(), s);
                }
            }
            map
        };

        let permissions = Permissions::load();

        // Initialize rate limit store with hardcoded defaults, then override from permissions.toml
        let mut rate_limit_store = daemon::RateLimitStore::new();
        rate_limit_store.load_overrides(permissions.rate_limits());

        Self {
            backend: Arc::new(RwLock::new(None)),
            event_tx,
            permissions,
            inhibitors: DashMap::new(),
            terminals: DashMap::new(),
            audit_log: Arc::new(Mutex::new(VecDeque::new())),
            audit_capacity: daemon::audit_capacity_from_env(),
            action_timeout_ms: daemon::action_timeout_from_env(),
            rate_limits: DashMap::new(),
            rate_limit: daemon::rate_limit_from_env(),
            rate_limit_store: Arc::new(rate_limit_store),
            clipboard_history: Arc::new(Mutex::new(VecDeque::new())),
            clipboard_history_capacity: daemon::clipboard_history_capacity_from_env(),
            schedule: Arc::new(daemon::schedule::ScheduleState::new()),
            recording: Arc::new(Mutex::new(None)),
            database: Arc::new(tokio::sync::Mutex::new(database)),
            sessions: DashMap::from_iter(sessions),
            rules: Arc::new(Mutex::new(RuleEngine::new())),
            screencast_process: Arc::new(Mutex::new(None)),
            pending_confirmations: DashMap::new(),
            agent_mailbox: Arc::new(daemon::execute_agent::AgentMailboxStore::new()),
            search_index: Arc::new(daemon::search::SearchIndex::new()),
            next_confirmation_id: AtomicU64::new(1),
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

    pub fn next_confirmation_id(&self) -> String {
        format!(
            "confirm-{}",
            self.next_confirmation_id.fetch_add(1, Ordering::Relaxed)
        )
    }

    /// Load persistent state from the SQLite database on daemon startup.
    /// Populates in-memory audit log and clipboard history caches.
    pub async fn load_persistent_state(&self) {
        daemon::audit::load_audit_from_db(self).await;
        daemon::clipboard::load_clipboard_from_db(self).await;
    }
}

impl Default for DaemonState {
    fn default() -> Self {
        Self::new()
    }
}

/// Per-client connection state
pub struct ConnectionState {
    /// Named session ID this client is connected to (defaults to "default")
    pub session_id: String,
    /// Glob-pattern subscriptions (e.g., "window.*", "clipboard.changed")
    pub subscriptions: HashSet<String>,
    /// Registered hotkey IDs
    pub hotkeys: HashSet<String>,
    /// Watched file paths
    pub watched_paths: HashSet<String>,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self {
            session_id: "default".to_string(),
            subscriptions: HashSet::new(),
            hotkeys: HashSet::new(),
            watched_paths: HashSet::new(),
        }
    }
}
