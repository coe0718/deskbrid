use crate::DaemonState;
use crate::daemon::helpers::home_dir;
use anyhow::Context;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use tokio::net::UnixListener;
use tracing::{debug, error, info, warn};

pub(crate) mod agent_registry;
pub(crate) mod apps;
pub(crate) mod apps_parse;
pub(crate) mod audit;
mod capabilities;
mod client;
pub(crate) mod clipboard;
mod dashboard;
mod dispatch;
mod dispatch_helpers;
pub(crate) mod execute;
mod execute_a11y;
pub(crate) mod execute_agent;
mod execute_audio;
mod execute_audit;
mod execute_blackboard;
mod execute_bluetooth;
mod execute_browser;
mod execute_capabilities;
mod execute_clipboard;
mod execute_color;
pub(crate) mod execute_confirmation;
mod execute_delegated;
mod execute_desktop;
mod execute_files;
mod execute_hotkeys;
mod execute_input;
mod execute_macro;
mod execute_monitor;
mod execute_network;
mod execute_notification;
mod execute_process;
mod execute_rules;
mod execute_screenshot;
mod execute_search;
pub(crate) mod execute_secrets;
mod execute_sessions;
mod execute_stubs;
mod execute_system;
mod execute_windows;
mod execute_workspace;
pub(crate) mod helpers;
mod layout;
pub(crate) mod locks;
pub(crate) mod macro_engine;
mod mpris;
pub(crate) mod mpris_convert;
pub mod persistence;
pub mod portal;
mod rate_limit;
pub(crate) mod region_watch;
pub(crate) mod rules;
pub(crate) mod schedule;
pub mod search;
mod sysfs;
mod system;
pub(crate) mod tcp;
pub mod terminal;
pub(crate) mod terminal_create;
pub(crate) mod terminal_helpers;
#[cfg(test)]
mod tests;
mod update_check;
mod wait;
pub(crate) mod wait_checks;
pub(crate) mod wait_params;

// Re-export the daemon's public API
pub(crate) use apps::{execute_app_catalog_action, is_app_catalog_action};
pub(crate) use audit::{
    AuditRecord, action_timeout_from_env, audit_capacity_from_env, execute_audit_action,
    is_audit_action,
};
pub use capabilities::{
    apply_gnome_capability_overrides, build_confinement_report, build_system_capabilities,
    build_system_health, normalize_coords, run_system_remediation,
};
pub use client::handle_client;
pub(crate) use clipboard::{
    clipboard_history_capacity_from_env, execute_clipboard_history_action,
    is_clipboard_history_action, record_clipboard_text,
};
pub use dispatch::dispatch_action;
pub(crate) use dispatch::dispatch_action_with_options;
pub use execute::execute_action;
pub use helpers::{
    ensure_safe_pid, expand_path, find_app_window, not_supported_response, ok_response,
    parse_signal, permission_denied_response, spawn_detached_process, unix_timestamp,
};
pub use layout::{
    capture_layout_profile, layout_profile_path, list_layout_profiles, load_layout_profile,
    match_profile_window_index, restore_layout_profile, save_layout_profile,
};
pub(crate) use mpris::{execute_mpris_action, is_mpris_action};
pub(crate) use rate_limit::{
    RateBucket, RateLimitConfig, RateLimitHit, RateLimitStore, action_namespace, check_rate_limit,
    rate_limit_from_env, rate_limited_response,
};
pub(crate) use sysfs::{cpu_frequency, cpu_governor, cpu_set_governor, thermal_get};
pub use system::{execute_system_control_action, is_system_control_action};
pub use terminal::{execute_terminal_action, is_terminal_action};
pub use wait::wait_for_condition;

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
        .unwrap_or_else(|_| {
            let fallback = format!("{}/.deskbrid.sock", home_dir().display());
            tracing::warn!(
                "XDG_RUNTIME_DIR not set — using fallback socket path: {}",
                fallback
            );
            fallback
        })
}

/// Start the Unix socket daemon and accept connections.
pub async fn run(
    no_dashboard: bool,
    dashboard_bind: String,
    tcp_bind: Option<String>,
    tcp_token: Option<String>,
    mcp_port: Option<u16>,
    mcp_token: Option<String>,
    mock_backend: bool,
) -> anyhow::Result<()> {
    let sock = socket_path();
    let _ = tokio::fs::remove_file(&sock).await;

    if let Some(parent) = std::path::Path::new(&sock).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let listener = UnixListener::bind(&sock).context("failed to bind Unix socket")?;

    // Restrict socket to owner-only — prevents other local users from connecting
    std::fs::set_permissions(&sock, std::fs::Permissions::from_mode(0o600))
        .context("failed to set socket permissions")?;

    info!("Deskbrid daemon listening on {}", sock);

    let state = Arc::new(DaemonState::new());
    state.load_persistent_state().await;

    // Start the web dashboard (runs regardless of backend status)
    if !no_dashboard {
        let dash_state = Arc::clone(&state);
        tokio::spawn(async move {
            dashboard::start(dash_state, dashboard_bind).await;
        });
    }

    // Load the desktop backend
    let backend_tx = state.event_tx.clone();
    let backend_result = if mock_backend {
        crate::backend::create_mock_backend(backend_tx).await
    } else {
        crate::backend::create_backend(backend_tx).await
    };
    match backend_result {
        Ok(backend) => {
            let backend_name = backend
                .system_info()
                .await
                .map(|info| info.desktop)
                .unwrap_or_else(|_| "desktop".to_string());
            let mut guard = state.backend.write().await;
            *guard = Some(backend);
            info!("{} backend loaded successfully", backend_name);
        }
        Err(e) => {
            warn!(
                "Failed to load desktop backend (running without desktop features): {}",
                e
            );
        }
    }

    // Start periodic update checker — polls GitHub for new releases
    update_check::spawn_update_checker(Arc::clone(&state));

    // Start confirmation TTL sweeper — purges expired pending confirmations
    execute_confirmation::spawn_confirmation_sweeper(Arc::clone(&state));

    // Start coordination sweepers — heartbeat timeouts and stale lock TTLs
    agent_registry::spawn_heartbeat_sweeper(Arc::clone(&state));
    locks::spawn_lock_sweeper(Arc::clone(&state));

    // Start schedule engine — runs configured actions on a timer
    schedule::spawn_schedule_engine(Arc::clone(&state.schedule), Arc::clone(&state));

    // Start rules engine — evaluates event-driven rules (#83)
    rules::spawn_rules_engine(Arc::clone(&state));
    // Start TimeRange rules evaluator — checks TimeRange triggers on a 60s timer
    rules::spawn_timerange_evaluator(Arc::clone(&state));

    // Start TCP listener if configured
    if let Some(bind) = tcp_bind {
        let token = tcp_token.unwrap_or_else(|| {
            let t = tcp::generate_token();
            info!("Generated TCP auth token: {}", t);
            t
        });
        let tcp_state = Arc::clone(&state);
        tokio::spawn(async move {
            if let Err(e) = tcp::run_tcp_listener(bind, token, tcp_state).await {
                error!("TCP listener exited: {}", e);
            }
        });
    }

    // Start MCP TCP listener if configured (shares daemon state)
    if let Some(port) = mcp_port {
        let mcp_token = mcp_token.unwrap_or_else(|| {
            let t = tcp::generate_token();
            info!("Generated MCP auth token: {}", t);
            t
        });
        let mcp_state = Arc::clone(&state);
        tokio::spawn(async move {
            if let Err(e) = crate::mcp::server::run_mcp_tcp(mcp_state, port, mcp_token).await {
                error!("MCP TCP listener exited: {}", e);
            }
        });
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
