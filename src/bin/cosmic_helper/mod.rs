//! COSMIC desktop helper — bridges Wayland protocol to JSON-over-stdin/stdout.
//!
//! Uses ext_foreign_toplevel_list_v1 for window discovery and
//! zcosmic_toplevel_info_v1 + zcosmic_toplevel_manager_v1 for window
//! properties (state, geometry) and control (close, activate, etc.).
//!
//! Usage: cosmic-helper <command> [options]

mod commands;
mod types;
mod windows_action;
mod windows_list;
mod workspaces;

use std::process;

// ─── CLI helpers ──────────────────────────────────────

fn parse_u64_arg(args: &[String], name: &str) -> Option<u64> {
    let pos = args.iter().position(|a| a == name)?;
    args.get(pos + 1).and_then(|v| v.parse::<u64>().ok())
}

pub(crate) fn ok_json(msg: Option<&str>) {
    match msg {
        Some(m) => println!("{{\"ok\": true, \"note\": \"{}\"}}", m),
        None => println!("{{\"ok\": true}}"),
    }
}

pub(crate) fn err_json(msg: &str) {
    println!("{{\"ok\": false, \"error\": \"{}\"}}", msg);
}

pub(crate) fn id_from_identifier(ident: &str) -> u64 {
    if !ident.is_empty() {
        let mut hash: u64 = 5381;
        for b in ident.bytes() {
            hash = hash.wrapping_mul(33).wrapping_add(b as u64);
        }
        hash
    } else {
        0
    }
}

// ─── Main ─────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cosmic-helper <command> [options]");
        eprintln!("Commands: probe, list-windows, focused-window, activate, close,");
        eprintln!("          maximize, unmaximize, minimize, unminimize, fullscreen,");
        eprintln!("          unfullscreen, workspace-list, workspace-activate, move-to-workspace");
        process::exit(1);
    }

    match args[1].as_str() {
        "probe" => commands::probe(),
        "list-windows" => commands::list_windows(),
        "focused-window" => commands::focused_window(),
        "activate" => {
            let wid = parse_u64_arg(&args, "--window-id").unwrap_or(0);
            commands::activate_window(wid);
        }
        "close" => {
            let wid = parse_u64_arg(&args, "--window-id").unwrap_or(0);
            commands::close_window(wid);
        }
        "maximize" => {
            let wid = parse_u64_arg(&args, "--window-id").unwrap_or(0);
            commands::set_maximized(wid, true);
        }
        "unmaximize" => {
            let wid = parse_u64_arg(&args, "--window-id").unwrap_or(0);
            commands::set_maximized(wid, false);
        }
        "minimize" => {
            let wid = parse_u64_arg(&args, "--window-id").unwrap_or(0);
            commands::set_minimized(wid, true);
        }
        "unminimize" => {
            let wid = parse_u64_arg(&args, "--window-id").unwrap_or(0);
            commands::set_minimized(wid, false);
        }
        "fullscreen" => {
            let wid = parse_u64_arg(&args, "--window-id").unwrap_or(0);
            commands::set_fullscreen(wid, true);
        }
        "unfullscreen" => {
            let wid = parse_u64_arg(&args, "--window-id").unwrap_or(0);
            commands::set_fullscreen(wid, false);
        }
        "workspace-list" => commands::workspace_list(),
        "workspace-activate" => {
            let id = parse_u64_arg(&args, "--id").unwrap_or(0) as u32;
            commands::workspace_activate(id);
        }
        "move-to-workspace" => {
            let wid = parse_u64_arg(&args, "--window-id").unwrap_or(0);
            let wsid = parse_u64_arg(&args, "--workspace-id").unwrap_or(0) as u32;
            commands::move_to_workspace(wid, wsid);
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            process::exit(1);
        }
    }
}
