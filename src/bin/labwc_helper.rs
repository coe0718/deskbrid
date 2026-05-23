//! Labwc desktop helper — bridges wlr-foreign-toplevel-management to CLI.
//!
//! Usage: labwc-helper <command> [args]
//!
//! CURRENT STATUS: CLI scaffold complete. Window management via
//! ext_foreign_toplevel_list_v1 is stubbed — returns valid JSON so the
//! daemon won't error. Actual Wayland protocol dispatch needs to be
//! implemented on a Labwc test machine.
//!
//! The Labwc backend currently uses wlrctl as the primary window management
//! interface and has_labwc_helper is hardcoded to false.

use serde::Serialize;
use std::process;

// ─── Types ────────────────────────────────────────────

#[derive(Serialize, Clone, Debug)]
#[allow(dead_code)]
struct WindowInfo {
    window_id: u64,
    title: Option<String>,
    app_id: Option<String>,
    focused: bool,
    minimized: bool,
    maximized: bool,
    fullscreen: bool,
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

// ─── Stub commands ────────────────────────────────────

fn probe() {
    match std::env::var("WAYLAND_DISPLAY") {
        Ok(socket) => {
            let xdg =
                std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/run/user/1000".to_string());
            let path = format!("{xdg}/{socket}");
            if std::path::Path::new(&path).exists() {
                println!("{{\"ok\": true, \"compositor\": \"labwc\", \"socket\": \"{path}\"}}");
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
    // STUB: requires ext_foreign_toplevel_list_v1 protocol binding.
    // Labwc backend currently uses wlrctl toplevel list.
    println!("[]");
}

fn activate(window_id: u64) {
    ok_json(Some(&format!("activate window_id={window_id} stubbed")));
}

fn close(window_id: u64) {
    ok_json(Some(&format!("close window_id={window_id} stubbed")));
}

fn set_maximized(window_id: u64) {
    ok_json(Some(&format!("maximize window_id={window_id} stubbed")));
}

fn set_minimized(window_id: u64) {
    ok_json(Some(&format!("minimize window_id={window_id} stubbed")));
}

fn set_fullscreen(window_id: u64) {
    ok_json(Some(&format!("fullscreen window_id={window_id} stubbed")));
}

// ─── Main ─────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: labwc-helper <command> [options]");
        eprintln!("Commands: probe, list-windows, activate, close, maximize, minimize, fullscreen");
        process::exit(1);
    }

    match args[1].as_str() {
        "probe" => probe(),
        "list-windows" => list_windows(),
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
            set_maximized(wid);
        }
        "minimize" => {
            let wid = parse_u64_arg(&args, "--window-id").unwrap_or(0);
            set_minimized(wid);
        }
        "fullscreen" => {
            let wid = parse_u64_arg(&args, "--window-id").unwrap_or(0);
            set_fullscreen(wid);
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            process::exit(1);
        }
    }
}
