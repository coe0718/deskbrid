//! System tray icon for Deskbrid — shows update status, controls the daemon,
//! and provides quick actions via StatusNotifierItem (KDE/GNOME/XFCE/etc.).
//!
//! TESTING_NEEDED: Requires a running desktop with StatusNotifierItem support
//! (GNOME with AppIndicator extension, KDE, XFCE, Budgie, etc.).

use ksni::{Handle, ToolTip, TrayMethods};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, Default)]
struct UpdateState {
    current_version: String,
    latest_version: String,
    update_available: bool,
    checked: bool,
    daemon_running: bool,
}

struct DeskbridTray {
    state: Arc<Mutex<UpdateState>>,
    handle: Arc<Mutex<Option<Handle<Self>>>>,
    shutdown: Arc<tokio::sync::Notify>,
}

impl DeskbridTray {
    fn new(shutdown: Arc<tokio::sync::Notify>) -> Self {
        Self {
            state: Arc::new(Mutex::new(UpdateState::default())),
            handle: Arc::new(Mutex::new(None)),
            shutdown,
        }
    }
}

impl ksni::Tray for DeskbridTray {
    fn id(&self) -> String {
        env!("CARGO_PKG_NAME").into()
    }

    fn title(&self) -> String {
        "Deskbrid".into()
    }

    fn icon_name(&self) -> String {
        let state = self.state.lock().unwrap();
        if !state.daemon_running {
            "process-stop".into()
        } else if state.update_available {
            "software-update-available".into()
        } else {
            "deskbrid".into()
        }
    }

    fn status(&self) -> ksni::Status {
        let state = self.state.lock().unwrap();
        if state.update_available {
            ksni::Status::NeedsAttention
        } else {
            ksni::Status::Active
        }
    }

    fn tool_tip(&self) -> ToolTip {
        let state = self.state.lock().unwrap();
        let title = if !state.daemon_running {
            "Deskbrid — daemon not running".into()
        } else if state.update_available {
            format!(
                "Deskbrid — Update available: v{} → v{}",
                state.current_version, state.latest_version
            )
        } else if state.checked {
            format!("Deskbrid v{} — up to date", state.current_version)
        } else {
            "Deskbrid — checking for updates...".into()
        };

        ToolTip {
            title,
            description: "Click for menu".into(),
            ..Default::default()
        }
    }

    // Left-click opens the menu
    const MENU_ON_ACTIVATE: bool = true;

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        let state = self.state.lock().unwrap();
        let mut items: Vec<ksni::MenuItem<Self>> = Vec::new();

        // ─── Daemon control ────────────────────────────
        if state.daemon_running {
            items.push(
                StandardItem {
                    label: "Stop Daemon".into(),
                    icon_name: "process-stop".into(),
                    activate: Box::new(|_: &mut Self| {
                        tokio::spawn(async {
                            let result = tokio::process::Command::new("systemctl")
                                .args(["--user", "stop", "deskbrid.service"])
                                .stdout(std::process::Stdio::null())
                                .stderr(std::process::Stdio::null())
                                .status()
                                .await;
                            match result {
                                Ok(s) if s.success() => info!("Daemon stopped"),
                                _ => {
                                    // Fallback: try pkill
                                    let _ = tokio::process::Command::new("pkill")
                                        .arg("deskbrid")
                                        .stdout(std::process::Stdio::null())
                                        .stderr(std::process::Stdio::null())
                                        .status()
                                        .await;
                                }
                            }
                        });
                    }),
                    ..Default::default()
                }
                .into(),
            );
        } else {
            items.push(
                StandardItem {
                    label: "Start Daemon".into(),
                    icon_name: "media-playback-start".into(),
                    activate: Box::new(|_: &mut Self| {
                        tokio::spawn(async {
                            let _ = tokio::process::Command::new("systemctl")
                                .args(["--user", "start", "deskbrid.service"])
                                .stdout(std::process::Stdio::null())
                                .stderr(std::process::Stdio::null())
                                .status()
                                .await;
                        });
                    }),
                    ..Default::default()
                }
                .into(),
            );
        }

        items.push(MenuItem::Separator);

        // ─── Version info ──────────────────────────────
        if state.update_available {
            items.push(
                StandardItem {
                    label: format!(
                        "Update: v{} → v{}",
                        state.current_version, state.latest_version
                    ),
                    enabled: false,
                    ..Default::default()
                }
                .into(),
            );
            items.push(
                StandardItem {
                    label: "Update Now".into(),
                    icon_name: "system-software-update".into(),
                    activate: Box::new(|_: &mut Self| {
                        tokio::spawn(async {
                            match tokio::process::Command::new("deskbrid")
                                .arg("update")
                                .status()
                                .await
                            {
                                Ok(status) if status.success() => {
                                    info!("Self-update completed");
                                }
                                Ok(status) => {
                                    warn!("Self-update exited: {}", status);
                                }
                                Err(e) => {
                                    error!("Self-update failed: {e}");
                                }
                            }
                        });
                    }),
                    ..Default::default()
                }
                .into(),
            );
        } else if state.checked {
            items.push(
                StandardItem {
                    label: format!("v{} — up to date", state.current_version),
                    enabled: false,
                    ..Default::default()
                }
                .into(),
            );
        } else {
            items.push(
                StandardItem {
                    label: "Checking...".into(),
                    enabled: false,
                    ..Default::default()
                }
                .into(),
            );
        }

        items.push(MenuItem::Separator);

        // ─── Actions ────────────────────────────────────
        items.push(
            StandardItem {
                label: "Check Now".into(),
                icon_name: "view-refresh".into(),
                activate: Box::new(|this: &mut Self| {
                    let state = this.state.clone();
                    // Scope: lock released before await
                    let handle_opt = {
                        let h = this.handle.lock().unwrap();
                        h.clone()
                    };
                    tokio::spawn(async move {
                        if let Err(e) = check_and_update(&state, handle_opt.as_ref()).await {
                            error!("Manual check failed: {e}");
                        }
                    });
                }),
                ..Default::default()
            }
            .into(),
        );

        items.push(
            StandardItem {
                label: "Open Dashboard".into(),
                icon_name: "applications-internet".into(),
                activate: Box::new(|_: &mut Self| {
                    tokio::spawn(async {
                        let _ = tokio::process::Command::new("xdg-open")
                            .arg("http://localhost:20129")
                            .status()
                            .await;
                    });
                }),
                ..Default::default()
            }
            .into(),
        );

        items.push(MenuItem::Separator);

        items.push(
            StandardItem {
                label: "Quit".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|this: &mut Self| {
                    this.shutdown.notify_one();
                }),
                ..Default::default()
            }
            .into(),
        );

        items
    }
}

/// Check for updates from the daemon, update tray state.
async fn check_and_update(
    state: &Arc<Mutex<UpdateState>>,
    handle: Option<&Handle<DeskbridTray>>,
) -> anyhow::Result<()> {
    let runtime = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| format!("/run/user/{}", unsafe { libc::getuid() }));
    let sock = format!("{}/deskbrid.sock", runtime);

    let mut stream = tokio::net::UnixStream::connect(&sock).await?;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let request = serde_json::json!({
        "type": "system.update",
        "id": "tray-check",
        "check": true,
        "force": false
    });
    stream
        .write_all(format!("{}\n", serde_json::to_string(&request)?).as_bytes())
        .await?;

    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).await?;
    let response: serde_json::Value = serde_json::from_slice(&buf[..n])?;

    let update_available = response["data"]["update_available"]
        .as_bool()
        .unwrap_or(false);
    let current = response["data"]["current_version"]
        .as_str()
        .unwrap_or("?")
        .to_string();
    let latest = response["data"]["latest_version"]
        .as_str()
        .unwrap_or("?")
        .to_string();

    // Scope the lock so it's dropped before await
    {
        let mut s = state.lock().unwrap();
        s.current_version = current;
        s.latest_version = latest;
        s.update_available = update_available;
        s.checked = true;
        s.daemon_running = true;
    }

    if let Some(h) = handle {
        h.update(|_| {}).await;
    }

    Ok(())
}

/// Check if daemon is running and update state.
async fn check_daemon_status(state: &Arc<Mutex<UpdateState>>) {
    let running = tokio::process::Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", "deskbrid.service"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false);

    let mut s = state.lock().unwrap();
    s.daemon_running = running;
}

/// Background loop: monitor daemon status and updates.
async fn event_loop(
    state: Arc<Mutex<UpdateState>>,
    handle: Arc<Mutex<Option<Handle<DeskbridTray>>>>,
    shutdown: Arc<tokio::sync::Notify>,
) {
    // Initial daemon check
    check_daemon_status(&state).await;

    // Initial update check if daemon is running
    {
        let running = state.lock().unwrap().daemon_running;
        if running {
            let h = { handle.lock().unwrap().clone() };
            if let Err(e) = check_and_update(&state, h.as_ref()).await {
                debug!("Initial update check failed: {e}");
            }
        }
    }

    // Refresh tray after initial checks
    {
        let h = handle.lock().unwrap().clone();
        if let Some(ref h) = h {
            h.update(|_| {}).await;
        }
    }

    loop {
        tokio::select! {
            _ = shutdown.notified() => {
                info!("Tray shutdown requested");
                break;
            }
            _ = sleep(Duration::from_secs(30)) => {
                // Periodic daemon status check
                check_daemon_status(&state).await;

                let running = state.lock().unwrap().daemon_running;
                if running {
                    let h = { handle.lock().unwrap().clone() };
                    if let Err(e) = check_and_update(&state, h.as_ref()).await {
                        debug!("Periodic check failed: {e}");
                    }
                }
            }
        }
    }
}

/// Run the tray icon. Blocks until Quit is selected or Ctrl+C received.
pub async fn run() -> anyhow::Result<()> {
    let shutdown = Arc::new(tokio::sync::Notify::new());
    let tray = DeskbridTray::new(shutdown.clone());
    let state = tray.state.clone();
    let handle_ref = tray.handle.clone();

    let tray_handle = tray.spawn().await?;
    *handle_ref.lock().unwrap() = Some(tray_handle);

    info!("Deskbrid tray icon started");

    // Background loop
    let bg_state = state.clone();
    let bg_handle = handle_ref.clone();
    let bg_shutdown = shutdown.clone();
    tokio::spawn(async move {
        event_loop(bg_state, bg_handle, bg_shutdown).await;
    });

    // Wait for shutdown
    shutdown.notified().await;
    info!("Tray shutting down");

    Ok(())
}
