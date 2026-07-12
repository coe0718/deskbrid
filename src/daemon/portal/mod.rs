//! XDG Desktop Portal integration for screenshots and screencasting.
//!
//! Talks to org.freedesktop.portal.Screenshot / ScreenCast via zbus on the session bus.
//! Uses the portal's request/response pattern: call method → get a handle →
//! listen for the Response signal → parse the result.

mod helpers;
mod screencast;
mod screenshot;

pub use screencast::{ActiveScreencast, portal_screencast_start, portal_screencast_stop};
pub(crate) use screenshot::portal_screenshot;

/// W18 (Vex review): extracted from `daemon/execute.rs` to keep that
/// file under the 250-line AGENTS.md cap. Dispatches
/// `PortalScreenshot`/`PortalScreencastStart`/`PortalScreencastStop`.
pub async fn execute_portal(
    action: crate::protocol::Action,
    state: &crate::DaemonState,
) -> anyhow::Result<serde_json::Value> {
    use crate::protocol::Action::*;
    Ok(match action {
        PortalScreenshot { interactive } => portal_screenshot(interactive).await?,
        PortalScreencastStart { output_path } => {
            portal_screencast_start(&output_path, &state.screencast_process).await?
        }
        PortalScreencastStop => portal_screencast_stop(&state.screencast_process).await?,
        _ => anyhow::bail!("internal dispatch error: not a portal action"),
    })
}
