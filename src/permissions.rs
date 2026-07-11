use crate::daemon::helpers::home_dir;
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
    /// Rate limit overrides from permissions.toml [rate_limits]
    #[serde(default)]
    rate_limits: HashMap<String, String>,
    /// Named per-agent/session profiles from permissions.toml [profile.NAME]
    #[serde(default)]
    profile: HashMap<String, ProfileEntry>,
    /// Agent safety automation from permissions.toml [auto_suspend]
    #[serde(default)]
    auto_suspend: AutoSuspendConfig,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct PermissionEntry {
    #[serde(default)]
    allow: Vec<String>,
    #[serde(default)]
    deny: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProfileEntry {
    #[serde(default)]
    pub allow: Vec<String>,
    #[serde(default)]
    pub deny: Vec<String>,
    #[serde(default)]
    pub confirm: Vec<String>,
    #[serde(default)]
    pub audit_level: Option<String>,
    #[serde(default)]
    pub rate_limits: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AutoSuspendConfig {
    #[serde(default = "default_auto_suspend_enabled")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub suspend_on_heartbeat_timeout: bool,
    #[serde(default = "default_true")]
    pub suspend_actions: bool,
}

impl Default for AutoSuspendConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            suspend_on_heartbeat_timeout: true,
            suspend_actions: true,
        }
    }
}

fn default_auto_suspend_enabled() -> bool {
    true
}

fn default_true() -> bool {
    true
}

impl Permissions {
    /// Rate limit overrides from permissions.toml [rate_limits] section.
    /// Keys are namespace prefixes (e.g. "secrets."), values are limit strings (e.g. "5/m").
    pub fn rate_limits(&self) -> &HashMap<String, String> {
        &self.inner.rate_limits
    }

    pub fn profile(&self, name: &str) -> Option<&ProfileEntry> {
        self.inner.profile.get(name)
    }

    pub fn profile_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.inner.profile.keys().cloned().collect();
        names.sort();
        names
    }

    pub fn auto_suspend(&self) -> &AutoSuspendConfig {
        &self.inner.auto_suspend
    }

    /// Load from config file, or return safe defaults if no file exists.
    /// On read/parse error, returns deny-all to prevent accidental over-permission.
    pub fn load() -> Self {
        let path = config_path();
        if !path.exists() {
            info!(
                "No permissions file at {}, using safe defaults. Create this file to customize.",
                path.display()
            );
            return Self::default_safe();
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
                rate_limits: HashMap::new(),
                profile: HashMap::new(),
                auto_suspend: AutoSuspendConfig::default(),
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
                rate_limits: HashMap::new(),
                profile: HashMap::new(),
                auto_suspend: AutoSuspendConfig::default(),
            }),
        }
    }

    /// Safe defaults for fresh installs — allows core automation primitives,
    /// denies destructive or sensitive actions. High-risk actions (process.start,
    /// secrets.*, files.write/delete, etc.) require explicit opt-in.
    /// Users can edit ~/.config/deskbrid/permissions.toml to customize.
    pub fn default_safe() -> Self {
        Self {
            inner: Arc::new(PermissionsInner {
                default: PermissionEntry {
                    allow: vec![
                        "windows.*".to_string(),
                        "workspaces.*".to_string(),
                        "system.*".to_string(),
                        "audio.*".to_string(),
                        "mpris.*".to_string(),
                        "network.*".to_string(),
                        "bluetooth.*".to_string(),
                        "input.layouts.*".to_string(),
                        "input.layout.*".to_string(),
                        "notification.*".to_string(),
                        "monitor.*".to_string(),
                        "search.*".to_string(),
                        "agent.*".to_string(),
                        "lock.*".to_string(),
                        "macro.*".to_string(),
                        "process.*".to_string(),
                        "audit.*".to_string(),
                        "apps.*".to_string(),
                        "capabilities.*".to_string(),
                        "desktop.*".to_string(),
                        "color.*".to_string(),
                        "blackboard.*".to_string(),
                        "confirmation.*".to_string(),
                        "rule.*".to_string(),
                        "session.*".to_string(),
                        "schedule.*".to_string(),
                        "a11y.*".to_string(),
                        "hotkeys.*".to_string(),
                        "power.profile.list".to_string(),
                        "power.profile.get".to_string(),
                        "power.profile.set".to_string(),
                        "battery.threshold.get".to_string(),
                        "battery.threshold.set".to_string(),
                        "env.get".to_string(),
                        "env.set".to_string(),
                        // Explicit — require exact naming because they're in HIGH_RISK_ACTIONS
                        "screenshot".to_string(),
                        "screenshot.ocr".to_string(),
                        "screenshot.diff".to_string(),
                        "region_watch.create".to_string(),
                        "region_watch.update".to_string(),
                        "region_watch.remove".to_string(),
                        "region_watch.list".to_string(),
                        "text_watch.create".to_string(),
                        "text_watch.remove".to_string(),
                        "text_watch.list".to_string(),
                        "clipboard.read".to_string(),
                        "input.keyboard".to_string(),
                        "input.mouse".to_string(),
                        "input.mouse.drag".to_string(),
                    ],
                    deny: vec![],
                },
                permissions: HashMap::new(),
                rate_limits: HashMap::new(),
                profile: HashMap::new(),
                auto_suspend: AutoSuspendConfig::default(),
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

    /// Check a named profile, when one is attached to a session. A profile is an
    /// additional sandbox: it can only narrow the UID permissions already granted.
    pub fn check_profile(&self, profile_name: Option<&str>, action: &Action) -> ProfileCheck {
        let Some(profile_name) = profile_name else {
            return ProfileCheck::Allowed;
        };

        let Some(profile) = self.inner.profile.get(profile_name) else {
            return ProfileCheck::Denied {
                profile: profile_name.to_string(),
                reason: "profile is not defined".to_string(),
            };
        };

        let action_name = action_name(action);
        if matches_patterns(&profile.deny, action_name, false) {
            return ProfileCheck::Denied {
                profile: profile_name.to_string(),
                reason: format!("action '{action_name}' denied by profile"),
            };
        }

        if profile.allow.is_empty() || !matches_patterns(&profile.allow, action_name, true) {
            return ProfileCheck::Denied {
                profile: profile_name.to_string(),
                reason: format!("action '{action_name}' not allowed by profile"),
            };
        }

        ProfileCheck::Allowed
    }

    pub fn profile_requires_confirmation(
        &self,
        profile_name: Option<&str>,
        action: &Action,
    ) -> bool {
        let Some(profile_name) = profile_name else {
            return false;
        };
        let Some(profile) = self.inner.profile.get(profile_name) else {
            return false;
        };
        matches_patterns(&profile.confirm, action_name(action), true)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileCheck {
    Allowed,
    Denied { profile: String, reason: String },
}

fn uid_key(uid: u32) -> String {
    format!("uid:{}", uid)
}

fn config_path() -> PathBuf {
    let home = home_dir().to_string_lossy().to_string();
    PathBuf::from(home)
        .join(".config")
        .join("deskbrid")
        .join("permissions.toml")
}

/// Actions that are never authorized by wildcard patterns.
/// These require explicit naming in the allow list — `"*"` or `"process.*"` won't cut it.
pub(crate) const HIGH_RISK_ACTIONS: &[&str] = &[
    "browser.evaluate",
    "process.start",
    "process.stop",
    "process.signal",
    "terminal.create",
    "system.update",
    "system.power",
    "dbus.call",
    "files.write",
    "files.delete",
    "files.move",
    "clipboard.read",
    "clipboard.history",
    "screenshot",
    "screenshot.ocr",
    "screenshot.diff",
    "region_watch.create",
    "region_watch.update",
    "text_watch.create",
    "input.keyboard",
    "input.mouse",
    "input.mouse.drag",
    "secrets.get_secret",
    "secrets.store_secret",
];

pub(crate) fn is_high_risk(action_name: &str) -> bool {
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

fn matches_patterns(patterns: &[String], action_name: &str, high_risk_exact: bool) -> bool {
    for pattern in patterns {
        if glob_match(pattern, action_name) {
            if high_risk_exact && is_high_risk(action_name) && pattern != action_name {
                continue;
            }
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
