//! COSMIC desktop helper — bridges Wayland protocol to JSON-over-stdin/stdout.
//!
//! Usage: cosmic-helper <command> [options]
//!
//! CURRENT STATUS: CLI scaffold complete. Window management via
//! ext_foreign_toplevel_list_v1 is stubbed — returns valid JSON so the
//! daemon won't error, but actual Wayland protocol dispatch needs to be
//! implemented on a COSMIC test machine. Workspace operations via
//! zcosmic_toplevel_manager_v1 are also stubbed.
//!
//! For now the COSMIC backend falls back to wlr-randr for workspace
//! detection and uses ydotool for input on COSMIC sessions.

use std::process;
use serde::Serialize;

// ─── Types ────────────────────────────────────────────

#[derive(Serialize, Clone, Debug)]
#[allow(dead_code)]
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

// ─── CLI helpers ──────────────────────────────────────

fn parse_u64_arg(args: &[String], name: &str) -> Option<u64> {
    let pos = args.iter().position(|a| a == name)?;
    args.get(pos + 1).and_then(|v| v.parse::<u64>().ok())
}

fn ok_json(msg: Option<&str>) {
    match msg {
        Some(m) => println!("{{\"ok\": true, \"note\": \"{}\"}}", m),
        None => println!("{{\"ok\": true}}"),
    }
}

#[allow(dead_code)]
fn err_json(msg: &str) {
    println!("{{\"ok\": false, \"error\": \"{}\"}}", msg);
}

// ─── Stub commands ────────────────────────────────────

/// Try to connect to the Wayland compositor. Returns ok if we can reach
/// the socket at all — real protocol binding happens per-command.
fn probe() {
    match std::env::var("WAYLAND_DISPLAY") {
        Ok(socket) => {
            let xdg = std::env::var("XDG_RUNTIME_DIR")
                .unwrap_or_else(|_| "/run/user/1000".to_string());
            let path = format!("{xdg}/{socket}");
            if std::path::Path::new(&path).exists() {
                println!("{{\"ok\": true, \"compositor\": \"cosmic\", \"socket\": \"{path}\"}}");
            } else {
                println!("{{\"ok\": false, \"error\": \"Wayland socket not found: {path}\"}}");
            }
        }
        Err(_) => {
            println!("{{\"ok\": false, \"error\": \"WAYLAND_DISPLAY not set\"}}");
        }
    }
}

fn list_windows() {
    // STUB: requires ext_foreign_toplevel_list_v1 Wayland protocol binding.
    // Returns empty array — daemon falls back gracefully.
    println!("[]");
}

fn focused_window() {
    // STUB: requires ext_foreign_toplevel_list_v1 protocol with focus tracking.
    println!("null");
}

fn activate(window_id: u64) {
    // STUB: requires ext_foreign_toplevel_handle_v1.activate(seat).
    // COSMIC backend currently falls back to ydotool/wtype for focus.
    ok_json(Some(&format!("activate window_id={window_id} stubbed")));
}

fn close(window_id: u64) {
    ok_json(Some(&format!("close window_id={window_id} stubbed")));
}

fn set_maximized(window_id: u64, on: bool) {
    ok_json(Some(&format!("maximize window_id={window_id} on={on} stubbed")));
}

fn set_minimized(window_id: u64, on: bool) {
    ok_json(Some(&format!("minimize window_id={window_id} on={on} stubbed")));
}

fn set_fullscreen(window_id: u64, on: bool) {
    ok_json(Some(&format!("fullscreen window_id={window_id} on={on} stubbed")));
}

fn workspace_list() {
    // STUB: requires zcosmic_toplevel_manager_v1 COSMIC protocol.
    // Backend falls back to wlr-randr for workspace detection.
    println!("[]");
}

fn workspace_activate(_id: u32) {
    ok_json(Some("workspace-activate stubbed — backend falls back to wlr-randr"));
}

fn move_to_workspace(_window_id: u64, _workspace_id: u32) {
    ok_json(Some("move-to-workspace stubbed — backend falls back to wlr-randr"));
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
        "probe" => probe(),
        "list-windows" => list_windows(),
        "focused-window" => focused_window(),
        "activate" => {
            let wid = parse_u64_arg(&args, "--window-id").unwrap_or(0);
            activate(wid);
        }
        "close" => {
            let wid = parse_u64_arg(&args, "--window-id").unwrap_or(0);
            close(wid);
        }
        "maximize" => {
            let wid = parse_u64_arg(&args, "--window-id").unwrap_or(0);
            set_maximized(wid, true);
        }
        "unmaximize" => {
            let wid = parse_u64_arg(&args, "--window-id").unwrap_or(0);
            set_maximized(wid, false);
        }
        "minimize" => {
            let wid = parse_u64_arg(&args, "--window-id").unwrap_or(0);
            set_minimized(wid, true);
        }
        "unminimize" => {
            let wid = parse_u64_arg(&args, "--window-id").unwrap_or(0);
            set_minimized(wid, false);
        }
        "fullscreen" => {
            let wid = parse_u64_arg(&args, "--window-id").unwrap_or(0);
            set_fullscreen(wid, true);
        }
        "unfullscreen" => {
            let wid = parse_u64_arg(&args, "--window-id").unwrap_or(0);
            set_fullscreen(wid, false);
        }
        "workspace-list" => workspace_list(),
        "workspace-activate" => {
            let id = parse_u64_arg(&args, "--id").unwrap_or(0) as u32;
            workspace_activate(id);
        }
        "move-to-workspace" => {
            let wid = parse_u64_arg(&args, "--window-id").unwrap_or(0);
            let wsid = parse_u64_arg(&args, "--workspace-id").unwrap_or(0) as u32;
            move_to_workspace(wid, wsid);
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            process::exit(1);
        }
    }
}
