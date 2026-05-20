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
    /// On parse error, logs a warning and falls back to allow-all.
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
                warn!("Failed to read permissions file {}: {}", path.display(), e);
                return Self::allow_all();
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
                warn!("Failed to parse permissions file {}: {}", path.display(), e);
                Self::allow_all()
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

/// Map an Action to its permission name string.
/// Uses the same dot-separated convention as the JSON protocol.
fn action_name(action: &Action) -> &'static str {
    use Action::*;
    match action {
        Ping => "ping",
        WindowsList => "windows.list",
        WindowsFocus(_) => "windows.focus",
        WindowsGet(_) => "windows.get",
        WindowsClose(_) => "windows.close",
        WindowsMinimize(_) => "windows.minimize",
        WindowsMaximize(_) => "windows.maximize",
        WindowsMoveResize { .. } => "windows.move_resize",
        WindowsActivateOrLaunch { .. } => "windows.activate_or_launch",
        WorkspacesList => "workspaces.list",
        WorkspaceSwitch(_) => "workspaces.switch",
        WorkspaceMoveWindow { .. } => "workspaces.move_window",
        LayoutProfilesList => "layout_profiles.list",
        LayoutProfileGet { .. } => "layout_profiles.get",
        LayoutProfileSave { .. } => "layout_profiles.save",
        LayoutProfileDelete { .. } => "layout_profiles.delete",
        LayoutProfileRestore { .. } => "layout_profiles.restore",
        InputKeyboardType { .. } | InputKeyboardKey { .. } | InputKeyboardCombo { .. } => {
            "input.keyboard"
        }
        InputMouse { .. } => "input.mouse",
        ClipboardRead => "clipboard.read",
        ClipboardWrite { .. } => "clipboard.write",
        Screenshot { .. } => "screenshot",
        NotificationSend { .. } => "notification.send",
        NotificationClose { .. } => "notification.close",
        SystemInfo => "system.info",
        SystemCapabilities => "system.capabilities",
        SystemHealth => "system.health",
        SystemRemediate { .. } => "system.remediate",
        SystemNormalizeCoords { .. } => "system.normalize_coords",
        SystemIdle => "system.idle",
        SystemPower { .. } => "system.power",
        SystemBattery => "system.battery",
        NetworkStatus => "network.status",
        NetworkInterfaces => "network.interfaces",
        NetworkWifiScan => "network.wifi.scan",
        NetworkWifiConnect { .. } => "network.wifi.connect",
        BluetoothList => "bluetooth.list",
        BluetoothScan { .. } => "bluetooth.scan",
        BluetoothStopScan => "bluetooth.stop_scan",
        BluetoothConnect { .. } => "bluetooth.connect",
        BluetoothDisconnect { .. } => "bluetooth.disconnect",
        BluetoothPair { .. } => "bluetooth.pair",
        BluetoothForget { .. } => "bluetooth.forget",
        FilesWatch { .. } => "files.watch",
        FilesUnwatch { .. } => "files.unwatch",
        FilesSearch { .. } => "files.search",
        FilesRead { .. } => "files.read",
        FilesWrite { .. } => "files.write",
        FilesCopy { .. } => "files.copy",
        FilesMove { .. } => "files.move",
        FilesDelete { .. } => "files.delete",
        FilesMkdir { .. } => "files.mkdir",
        FilesList { .. } => "files.list",
        ProcessList => "process.list",
        ProcessStart { .. } => "process.start",
        ProcessStop { .. } => "process.stop",
        ProcessSignal { .. } => "process.signal",
        ProcessExists { .. } => "process.exists",
        ProcessWait { .. } => "process.wait",
        HotkeysRegister { .. } => "hotkeys.register",
        HotkeysUnregister { .. } => "hotkeys.unregister",
        AudioListSinks => "audio.list_sinks",
        AudioSetSinkVolume { .. } => "audio.set_sink_volume",
        MonitorList => "monitor.list",
        MonitorSetPrimary { .. } => "monitor.set_primary",
        MonitorSetResolution { .. } => "monitor.set_resolution",
        MonitorSetScale { .. } => "monitor.set_scale",
        MonitorSetRotation { .. } => "monitor.set_rotation",
        MonitorEnable { .. } => "monitor.enable",
        MonitorDisable { .. } => "monitor.disable",
        LocationGet => "location.get",
        UiTreeGet => "ui.tree.get",
        UiElementClick { .. } => "ui.element.click",
        UiElementSetText { .. } => "ui.element.set_text",
        CapabilitiesList => "capabilities.list",
        Subscribe { .. } => "_subscribe",
        Unsubscribe { .. } => "_unsubscribe",
        Disconnect => "_disconnect",
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

    if ret == 0 { Some(cred.uid) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match_exact() {
        assert!(glob_match("screenshot", "screenshot"));
        assert!(glob_match("windows.list", "windows.list"));
        assert!(!glob_match("windows.list", "windows.focus"));
    }

    #[test]
    fn test_glob_match_wildcard() {
        assert!(glob_match("*", "screenshot"));
        assert!(glob_match("*", "windows.list"));
        assert!(glob_match("*", "anything.at.all"));
    }

    #[test]
    fn test_glob_match_category() {
        assert!(glob_match("windows.*", "windows.list"));
        assert!(glob_match("windows.*", "windows.focus"));
        assert!(glob_match("windows.*", "windows.get"));
        assert!(glob_match("input.*", "input.keyboard"));
        assert!(glob_match("input.*", "input.mouse"));
    }

    #[test]
    fn test_glob_match_no_false_positives() {
        assert!(!glob_match("windows.*", "screenshot"));
        assert!(!glob_match("windows.*", "clipboard.read"));
        assert!(!glob_match("screenshot", "clipboard.read"));
    }

    #[test]
    fn test_glob_match_prefix_not_segment() {
        // "window.*" should NOT match "windows.list" — different segment
        assert!(!glob_match("window.*", "windows.list"));
        // "clip.*" should NOT match "clipboard.read"
        assert!(!glob_match("clip.*", "clipboard.read"));
    }

    #[test]
    fn test_permissions_allow_all() {
        let p = Permissions::allow_all();
        assert!(p.check(
            1000,
            &Action::Screenshot {
                monitor: None,
                region: None,
                window_id: None
            }
        ));
        assert!(p.check(1000, &Action::ClipboardRead));
        assert!(p.check(
            2000,
            &Action::ProcessStart {
                command: vec!["rm".into(), "-rf".into(), "/".into()],
                workdir: None,
                env: None,
            }
        ));
    }

    #[test]
    fn test_permissions_deny_screenshot() {
        let inner = PermissionsInner {
            default: PermissionEntry {
                allow: vec!["*".into()],
                deny: vec!["screenshot".into()],
            },
            permissions: HashMap::new(),
        };
        let p = Permissions {
            inner: Arc::new(inner),
        };

        assert!(!p.check(
            1000,
            &Action::Screenshot {
                monitor: None,
                region: None,
                window_id: None
            }
        ));
        assert!(p.check(1000, &Action::ClipboardRead));
        assert!(p.check(1000, &Action::WindowsList));
    }

    #[test]
    fn test_permissions_per_uid() {
        let mut per_uid = HashMap::new();
        per_uid.insert(
            "uid:1000".into(),
            PermissionEntry {
                allow: vec!["*".into()],
                deny: vec![],
            },
        );
        per_uid.insert(
            "uid:1001".into(),
            PermissionEntry {
                allow: vec!["windows.*".into(), "clipboard.read".into()],
                deny: vec!["screenshot".into()],
            },
        );

        let inner = PermissionsInner {
            default: PermissionEntry {
                allow: vec![],
                deny: vec!["*".into()],
            },
            permissions: per_uid,
        };
        let p = Permissions {
            inner: Arc::new(inner),
        };

        // uid:1000 — full access
        assert!(p.check(
            1000,
            &Action::Screenshot {
                monitor: None,
                region: None,
                window_id: None
            }
        ));

        // uid:1001 — windows + clipboard only, no screenshot
        assert!(p.check(1001, &Action::WindowsList));
        assert!(p.check(1001, &Action::ClipboardRead));
        assert!(!p.check(
            1001,
            &Action::Screenshot {
                monitor: None,
                region: None,
                window_id: None
            }
        ));
        assert!(!p.check(
            1001,
            &Action::InputKeyboardType {
                text: "hello".into()
            }
        ));

        // uid:9999 — falls back to default (deny-all)
        assert!(!p.check(9999, &Action::WindowsList));
        assert!(!p.check(9999, &Action::Ping));
    }

    #[test]
    fn test_permissions_ping_always_allowed_in_default_deny() {
        // If default denies everything, even ping is denied
        let inner = PermissionsInner {
            default: PermissionEntry {
                allow: vec![],
                deny: vec!["*".into()],
            },
            permissions: HashMap::new(),
        };
        let p = Permissions {
            inner: Arc::new(inner),
        };
        assert!(!p.check(9999, &Action::Ping));
    }

    #[test]
    fn test_action_name_mapping() {
        assert_eq!(action_name(&Action::WindowsList), "windows.list");
        assert_eq!(
            action_name(&Action::Screenshot {
                monitor: None,
                region: None,
                window_id: None
            }),
            "screenshot"
        );
        assert_eq!(
            action_name(&Action::InputKeyboardType { text: "".into() }),
            "input.keyboard"
        );
        assert_eq!(
            action_name(&Action::InputMouse {
                action: "click".into(),
                x: None,
                y: None,
                button: None,
                dx: None,
                dy: None
            }),
            "input.mouse"
        );
        assert_eq!(action_name(&Action::ClipboardRead), "clipboard.read");
        assert_eq!(
            action_name(&Action::ProcessStart {
                command: vec![],
                workdir: None,
                env: None
            }),
            "process.start"
        );
        assert_eq!(
            action_name(&Action::WindowsActivateOrLaunch {
                app_id: "code".into(),
                command: vec!["code".into()],
                workdir: None,
                env: None,
            }),
            "windows.activate_or_launch"
        );
        assert_eq!(
            action_name(&Action::LayoutProfileRestore {
                name: "coding".into()
            }),
            "layout_profiles.restore"
        );
        assert_eq!(
            action_name(&Action::MonitorSetScale {
                output: "DP-1".into(),
                scale: 1.25,
            }),
            "monitor.set_scale"
        );
        assert_eq!(
            action_name(&Action::MonitorDisable {
                output: "HDMI-A-1".into(),
            }),
            "monitor.disable"
        );
    }
}
