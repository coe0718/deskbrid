//! Periodic update check — polls GitHub for new releases and broadcasts
//! `update.available` events to subscribed clients.

use crate::DaemonState;
use crate::protocol::DeskbridEvent;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info};

/// Check interval: poll GitHub once every 24 hours after the first check.
const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 3600);

/// Spawn a background task that checks for Deskbrid updates on startup
/// and periodically thereafter. When a newer release is found, it broadcasts
/// an `UpdateAvailable` event to all subscribed clients.
pub fn spawn_update_checker(state: Arc<DaemonState>) {
    tokio::spawn(async move {
        loop {
            match crate::cmd::update::run_json(true, false).await {
                Ok(result) => {
                    if result["update_available"].as_bool().unwrap_or(false) {
                        let current = result["current_version"]
                            .as_str()
                            .unwrap_or("unknown")
                            .to_string();
                        let latest = result["latest_version"]
                            .as_str()
                            .unwrap_or("unknown")
                            .to_string();
                        info!("Update available: v{current} → v{latest}");
                        let event = DeskbridEvent::UpdateAvailable {
                            current_version: current,
                            latest_version: latest,
                        };
                        let receivers = state.event_tx.receiver_count();
                        let _ = state.event_tx.send(event);
                        debug!("Broadcast update.available to {receivers} subscriber(s)");
                    } else {
                        let current = result["current_version"].as_str().unwrap_or("unknown");
                        debug!("No update available (v{current})");
                    }
                }
                Err(e) => {
                    error!("Update check failed: {e:#}");
                }
            }

            tokio::time::sleep(CHECK_INTERVAL).await;
        }
    });
}
