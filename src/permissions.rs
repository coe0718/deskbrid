use crate::protocol::Action;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn};

/// Loaded permissions config shared across all client connections
#[derive(Debug, Clone)]
pub struct Permissions {
    inner: Arc<PermissionsInner>,
}

#[derive(Debug, Deserialize, Clone)]
struct PermissionsInner {
    #[serde(default)]
    default: PermissionEntry,
    /// Keyed by "uid:N" — e.g. "uid:1000"
    #[serde(default)]
    permissions: HashMap<String, PermissionEntry>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct PermissionEntry {
    #[serde(default)]
    allow: Vec<String>,
    #[serde(default)]
    deny: Vec<String>,
}

impl Permissions {
    /// Load from config file, or return allow-all if no file exists.
    /// On read/parse error, returns deny-all to prevent accidental over-permission.
    pub fn load() -> Self {
        let path = config_path();
        if !path.exists() {
            info!(
                "No permissions file at {}, defaulting to allow-all",
                path.display()
            );
            return Self::allow_all();
        }

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                warn!(
                    "Failed to read permissions file {}: {}. Denying all actions.",
                    path.display(),
                    e
                );
                return Self::deny_all();
            }
        };

        match toml::from_str::<PermissionsInner>(&content) {
            Ok(inner) => {
                info!("Loaded permissions from {}", path.display());
                Self {
                    inner: Arc::new(inner),
                }
            }
            Err(e) => {
                warn!(
                    "Failed to parse permissions file {}: {}. Denying all actions.",
                    path.display(),
                    e
                );
                Self::deny_all()
            }
        }
    }

    /// No restrictions — backward compatible with existing installs
    pub fn allow_all() -> Self {
        Self {
            inner: Arc::new(PermissionsInner {
                default: PermissionEntry {
                    allow: vec!["*".to_string()],
                    deny: vec![],
                },
                permissions: HashMap::new(),
            }),
        }
    }

    /// Deny everything — used when the config file exists but can't be parsed
    pub fn deny_all() -> Self {
        Self {
            inner: Arc::new(PermissionsInner {
                default: PermissionEntry {
                    allow: vec![],
                    deny: vec!["*".to_string()],
                },
                permissions: HashMap::new(),
            }),
        }
    }

    /// Check if an action is permitted for the given UID.
    /// Returns true if allowed, false if denied.
    pub fn check(&self, uid: u32, action: &Action) -> bool {
        let entry = self
            .inner
            .permissions
            .get(&uid_key(uid))
            .unwrap_or(&self.inner.default);

        let action_name = action_name(action);

        // Deny list checked first — explicit deny always wins
        for pattern in &entry.deny {
            if glob_match(pattern, action_name) {
                return false;
            }
        }

        // Allow list
        for pattern in &entry.allow {
            if glob_match(pattern, action_name) {
                // High-risk actions require explicit naming, not wildcards.
                // `"*"`, `"browser.*"`, `"browser.eval*"` won't work — only `"browser.evaluate"`.
                if is_high_risk(action_name) && pattern != action_name {
                    continue;
                }
                return true;
            }
        }

        // Default deny if no pattern matched
        false
    }
}

fn uid_key(uid: u32) -> String {
    format!("uid:{}", uid)
}

fn config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    PathBuf::from(home)
        .join(".config")
        .join("deskbrid")
        .join("permissions.toml")
}

/// Actions that are never authorized by wildcard patterns.
/// These require explicit naming in the allow list — `"*"` or `"browser.*"` won't cut it.
const HIGH_RISK_ACTIONS: &[&str] = &["browser.evaluate", "process.start", "terminal.create"];

fn is_high_risk(action_name: &str) -> bool {
    HIGH_RISK_ACTIONS.contains(&action_name)
}

/// Map an Action to its permission name string.
/// Uses the same dot-separated convention as the JSON protocol.
fn action_name(action: &Action) -> &'static str {
    match action {
        Action::Subscribe { .. } => "_subscribe",
        Action::Unsubscribe { .. } => "_unsubscribe",
        Action::Disconnect => "_disconnect",
        _ => action.action_type(),
    }
}

/// Simple glob matching.
/// Supports `"*"` for everything and `"prefix.*"` for category wildcards.
///
/// Examples:
/// - `"*"` matches everything
/// - `"windows.*"` matches `"windows.list"`, `"windows.focus"`, etc.
/// - `"windows.list"` matches exactly `"windows.list"`
/// - `"input.*"` matches `"input.keyboard"`, `"input.mouse"`
/// - `"screenshot"` matches exactly `"screenshot"`
fn glob_match(pattern: &str, name: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if pattern == name {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix(".*") {
        if name == prefix {
            return true;
        }
        if name.starts_with(prefix) && name.as_bytes().get(prefix.len()) == Some(&b'.') {
            return true;
        }
    }
    false
}

/// Extract the peer UID from a Unix socket connection (Linux SO_PEERCRED).
pub fn socket_peer_uid(stream: &tokio::net::UnixStream) -> Option<u32> {
    use std::os::unix::io::AsRawFd;

    let fd = stream.as_raw_fd();
    let mut cred: libc::ucred = unsafe { std::mem::zeroed() };
    let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;

    let ret = unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            &mut cred as *mut _ as *mut libc::c_void,
            &mut len,
        )
    };

    if ret == 0 {
        let expected_len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
        if len != expected_len {
            return None;
        }
        Some(cred.uid)
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
