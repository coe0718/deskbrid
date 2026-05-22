//! COSMIC desktop helper — bridges Wayland protocol to JSON-over-stdin/stdout.
//!
//! Runs inside the COSMIC compositor session. Exposes window/workspace
//! operations as simple CLI commands. The main deskbrid daemon spawns this
//! binary as a subprocess.

mod commands;
mod dispatch;

use commands::*;
use cosmic_protocols::toplevel_info::v1::client::zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1;
use cosmic_protocols::toplevel_management::v1::client::zcosmic_toplevel_manager_v1;
use cosmic_protocols::toplevel_management::v1::client::zcosmic_toplevel_manager_v1::ZcosmicToplevelManagerV1;
use std::collections::HashMap;
use wayland_client::{
    Connection, Dispatch, QueueHandle,
    protocol::{wl_output, wl_seat, wl_surface},
};
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1;
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_list_v1;
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1;

#[derive(serde::Serialize, Clone, Debug)]
struct WindowInfo {
    window_id: u64,
    title: Option<String>,
    app_id: Option<String>,
    pid: Option<u32>,
    x: Option<i32>,
    y: Option<i32>,
    width: Option<u32>,
    height: Option<u32>,
    focused: bool,
    minimized: bool,
    maximized: bool,
    fullscreen: bool,
    workspace_id: Option<u32>,
}

#[allow(dead_code)]
struct CosmicState {
    windows: HashMap<u64, WindowInfo>,
    last_activate_window_id: Option<u64>,
    activate_timestamp_ms: Option<u128>,
    toplevel_manager: Option<ZcosmicToplevelManagerV1>,
    ext_handle_ids: HashMap<u64, u64>,
    done: bool,
}

impl CosmicState {
    #[allow(dead_code)]
    fn new() -> Self {
        Self {
            windows: HashMap::new(),
            last_activate_window_id: None,
            activate_timestamp_ms: None,
            toplevel_manager: None,
            ext_handle_ids: HashMap::new(),
            done: false,
        }
    }
}

fn parse_u64_arg(args: &[String], name: &str) -> Option<u64> {
    let pos = args.iter().position(|a| a == name)?;
    args.get(pos + 1).and_then(|v| v.parse::<u64>().ok())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cosmic-helper <command> [options]");
        eprintln!(
            "Commands: probe, list-windows, focused-window, activate, close, maximize, minimize, fullscreen, unmaximize, unminimize, unfullscreen, workspace-list, workspace-activate, move-to-workspace"
        );
        std::process::exit(1);
    }

    match args[1].as_str() {
        "probe" => {
            cmd_probe();
            Ok(())
        }
        "list-windows" => cmd_list_windows(),
        "focused-window" => cmd_focused_window(),
        "activate" => {
            let window_id =
                parse_u64_arg(&args, "--window-id").ok_or("Missing --window-id argument")?;
            cmd_activate(window_id)
        }
        "close" => {
            let window_id =
                parse_u64_arg(&args, "--window-id").ok_or("Missing --window-id argument")?;
            cmd_close(window_id)
        }
        "maximize" => {
            let window_id =
                parse_u64_arg(&args, "--window-id").ok_or("Missing --window-id argument")?;
            cmd_maximize(window_id, true)
        }
        "unmaximize" => {
            let window_id =
                parse_u64_arg(&args, "--window-id").ok_or("Missing --window-id argument")?;
            cmd_maximize(window_id, false)
        }
        "minimize" => {
            let window_id =
                parse_u64_arg(&args, "--window-id").ok_or("Missing --window-id argument")?;
            cmd_minimize(window_id, true)
        }
        "unminimize" => {
            let window_id =
                parse_u64_arg(&args, "--window-id").ok_or("Missing --window-id argument")?;
            cmd_minimize(window_id, false)
        }
        "fullscreen" => {
            let window_id =
                parse_u64_arg(&args, "--window-id").ok_or("Missing --window-id argument")?;
            cmd_fullscreen(window_id, true)
        }
        "unfullscreen" => {
            let window_id =
                parse_u64_arg(&args, "--window-id").ok_or("Missing --window-id argument")?;
            cmd_fullscreen(window_id, false)
        }
        "workspace-list" => cmd_workspace_list(),
        "workspace-activate" => {
            let id = parse_u64_arg(&args, "--id").ok_or("Missing --id argument")? as u32;
            cmd_workspace_activate(id)
        }
        "move-to-workspace" => {
            let window_id =
                parse_u64_arg(&args, "--window-id").ok_or("Missing --window-id argument")?;
            let workspace_id = parse_u64_arg(&args, "--workspace-id")
                .ok_or("Missing --workspace-id argument")? as u32;
            cmd_move_to_workspace(window_id, workspace_id)
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            std::process::exit(1);
        }
    }
}
