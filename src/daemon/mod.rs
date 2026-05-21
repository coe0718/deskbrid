use crate::DaemonState;
use anyhow::Context;
use std::sync::Arc;
use tokio::net::UnixListener;
use tracing::{debug, error, info, warn};

mod capabilities;
mod client;
mod dispatch;
mod execute;
mod helpers;
mod layout;
mod system;
#[cfg(test)]
mod tests;

// Re-export the daemon's public API
pub use capabilities::{
    apply_gnome_capability_overrides, build_system_capabilities, build_system_health,
    normalize_coords, run_system_remediation,
};
pub use client::handle_client;
pub use dispatch::dispatch_action;
pub use execute::execute_action;
pub use helpers::{
    ensure_safe_pid, expand_path, find_app_window, not_supported_response, ok_response,
    parse_signal, permission_denied_response, spawn_detached_process, unix_timestamp,
};
pub use layout::{
    capture_layout_profile, list_layout_profiles, load_layout_profile, match_profile_window_index,
    restore_layout_profile, save_layout_profile,
};
pub use system::{execute_system_control_action, is_system_control_action};

pub(crate) const MONITOR_CONTROL_ACTIONS: &[&str] = &[
    "monitor.set_primary",
    "monitor.set_resolution",
    "monitor.set_scale",
    "monitor.set_rotation",
    "monitor.enable",
    "monitor.disable",
];

pub(crate) fn socket_path() -> String {
    std::env::var("XDG_RUNTIME_DIR")
        .map(|d| format!("{}/deskbrid.sock", d))
        .unwrap_or_else(|_| "/run/user/1000/deskbrid.sock".into())
}

/// Start the Unix socket daemon and accept connections.
pub async fn run() -> anyhow::Result<()> {
    let sock = socket_path();
    let _ = tokio::fs::remove_file(&sock).await;

    if let Some(parent) = std::path::Path::new(&sock).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let listener = UnixListener::bind(&sock).context("failed to bind Unix socket")?;

    info!("Deskbrid daemon listening on {}", sock);

    let state = Arc::new(DaemonState::new());

    // Load the desktop backend
    let backend_tx = state.event_tx.clone();
    match crate::backend::create_backend(backend_tx).await {
        Ok(backend) => {
            let mut guard = state.backend.write().await;
            *guard = Some(backend);
            info!("GNOME backend loaded successfully");
        }
        Err(e) => {
            warn!(
                "Failed to load GNOME backend (running without desktop features): {}",
                e
            );
        }
    }

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                debug!("New connection from {:?}", addr);
                let state = Arc::clone(&state);
                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream, &state).await {
                        error!("Client error: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("Accept error: {}", e);
            }
        }
    }
}
