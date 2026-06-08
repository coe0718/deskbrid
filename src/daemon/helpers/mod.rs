//! Daemon helper utilities: path resolution, process management, JSON responses.

mod paths;
mod process;
mod responses;

pub use paths::{expand_path, home_dir, screenshot_temp_path, unix_timestamp};
pub use process::{ensure_safe_pid, find_app_window, parse_signal, spawn_detached_process};
pub use responses::{not_supported_response, ok_response, permission_denied_response};
