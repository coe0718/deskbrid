//! XDG Desktop Portal integration for screenshots and screencasting.
//!
//! Talks to org.freedesktop.portal.Screenshot / ScreenCast via zbus on the session bus.
//! Uses the portal's request/response pattern: call method → get a handle →
//! listen for the Response signal → parse the result.

mod helpers;
mod screencast;
mod screenshot;

pub(crate) use screencast::{ActiveScreencast, portal_screencast_start, portal_screencast_stop};
pub(crate) use screenshot::portal_screenshot;
