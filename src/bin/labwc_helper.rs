//! Labwc desktop helper — bridges wlr-foreign-toplevel-management to CLI.
//!
//! Labwc has no external IPC. Uses Wayland protocols for window management.
//! Architecture follows the COSMIC helper pattern: stub commands.
//!
//! Usage: labwc-helper <command> [args]

mod commands;
mod dispatch;

use commands::*;
use serde::Serialize;
use std::collections::HashMap;
use wayland_client::{
    Connection, Dispatch, QueueHandle,
    protocol::{wl_output, wl_seat, wl_surface},
};
use wayland_protocols::ext::foreign_toplevel_list::v1::client::{
    ext_foreign_toplevel_handle_v1::{self, ExtForeignToplevelHandleV1},
    ext_foreign_toplevel_list_v1::{self, ExtForeignToplevelListV1},
};

#[derive(Serialize, Clone, Debug)]
struct WindowInfo {
    window_id: u64,
    title: Option<String>,
    app_id: Option<String>,
    focused: bool,
    minimized: bool,
    maximized: bool,
    fullscreen: bool,
}

struct LabwcState {
    windows: HashMap<u64, WindowInfo>,
    ext_handle_ids: HashMap<u64, u64>,
    done: bool,
}

impl LabwcState {
    fn new() -> Self {
        Self {
            windows: HashMap::new(),
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
        eprintln!("Usage: labwc-helper <command> [options]");
        eprintln!("Commands: probe, list-windows, activate, close, maximize, minimize, fullscreen");
        std::process::exit(1);
    }
    match args[1].as_str() {
        "probe" => {
            cmd_probe();
            Ok(())
        }
        "list-windows" => cmd_list_windows(),
        "activate" => {
            let id = parse_u64_arg(&args, "--window-id").ok_or("Missing --window-id")?;
            cmd_activate(id)
        }
        "close" => {
            let id = parse_u64_arg(&args, "--window-id").ok_or("Missing --window-id")?;
            cmd_close(id)
        }
        "maximize" => {
            let id = parse_u64_arg(&args, "--window-id").ok_or("Missing --window-id")?;
            cmd_maximize(id)
        }
        "minimize" => {
            let id = parse_u64_arg(&args, "--window-id").ok_or("Missing --window-id")?;
            cmd_minimize(id)
        }
        "fullscreen" => {
            let id = parse_u64_arg(&args, "--window-id").ok_or("Missing --window-id")?;
            cmd_fullscreen(id)
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            std::process::exit(1);
        }
    }
}
