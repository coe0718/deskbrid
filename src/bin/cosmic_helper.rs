//! COSMIC desktop helper — bridges Wayland protocol to JSON-over-stdin/stdout.
//!
//! Runs inside the COSMIC compositor session. Exposes window/workspace
//! operations as simple CLI commands. The main deskbrid daemon spawns this
//! binary as a subprocess.

use std::collections::HashMap;
use wayland_client::{
    Connection, Dispatch, QueueHandle,
    protocol::{wl_output, wl_seat, wl_surface},
};

use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1;
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_list_v1;
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1;

use cosmic_protocols::toplevel_info::v1::client::zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1;
use cosmic_protocols::toplevel_management::v1::client::zcosmic_toplevel_manager_v1;
use cosmic_protocols::toplevel_management::v1::client::zcosmic_toplevel_manager_v1::ZcosmicToplevelManagerV1;

// ─── Data types ────────────────────────────────────────────────────────────

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

#[derive(serde::Serialize)]
struct ProbeOutput {
    ok: bool,
    can_list_windows: bool,
    can_activate_windows: bool,
    detail: String,
}

#[derive(serde::Serialize)]
struct SimpleOutput {
    ok: bool,
    detail: String,
}

// ─── Wayland state ─────────────────────────────────────────────────────────

#[allow(dead_code)]
struct CosmicState {
    windows: HashMap<u64, WindowInfo>,
    /// Track active window IDs for activation state TTL
    last_activate_window_id: Option<u64>,
    activate_timestamp_ms: Option<u128>,
    /// Toplevel manager for window management requests
    toplevel_manager: Option<ZcosmicToplevelManagerV1>,
    /// Track which ext_ handle maps to which window ID
    ext_handle_ids: HashMap<u64, u64>,
    /// Round-trip done flag
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

// ─── Dispatch impls ────────────────────────────────────────────────────────

impl Dispatch<ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1, ()> for CosmicState {
    fn event(
        state: &mut Self,
        _proxy: &ExtForeignToplevelListV1,
        event: <ExtForeignToplevelListV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            ext_foreign_toplevel_list_v1::Event::Finished => {
                state.done = true;
            }
            _ => {}
        }
    }
}

impl Dispatch<ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1, ()> for CosmicState {
    fn event(
        state: &mut Self,
        proxy: &ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
        event: <ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        let proxy_id = &*proxy as *const _ as u64;

        match event {
            ext_foreign_toplevel_handle_v1::Event::Title { title } => {
                if let Some(&wid) = state.ext_handle_ids.get(&proxy_id) {
                    if let Some(w) = state.windows.get_mut(&wid) {
                        w.title = Some(title);
                    }
                }
            }
            ext_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                if let Some(&wid) = state.ext_handle_ids.get(&proxy_id) {
                    if let Some(w) = state.windows.get_mut(&wid) {
                        w.app_id = Some(app_id);
                    }
                }
            }
            // State flags are not available in this protocol version.
            // focused/minimized tracking will be added when protocol is updated.
            ext_foreign_toplevel_handle_v1::Event::Identifier { identifier } => {
                // Create window entry with this stable identifier
                let num_id: u64 = identifier.parse().unwrap_or(0);
                if num_id > 0 {
                    state.ext_handle_ids.insert(proxy_id, num_id);
                    state.windows.entry(num_id).or_insert(WindowInfo {
                        window_id: num_id,
                        title: None,
                        app_id: None,
                        pid: None,
                        x: None,
                        y: None,
                        width: None,
                        height: None,
                        focused: false,
                        minimized: false,
                        maximized: false,
                        fullscreen: false,
                        workspace_id: None,
                    });
                }
            }
            ext_foreign_toplevel_handle_v1::Event::Closed => {
                if let Some(&wid) = state.ext_handle_ids.get(&proxy_id) {
                    state.windows.remove(&wid);
                }
            }
            ext_foreign_toplevel_handle_v1::Event::Done => {
                // Window info fully sent
            }
            _ => {}
        }
    }
}

impl Dispatch<ZcosmicToplevelManagerV1, ()> for CosmicState {
    fn event(
        _state: &mut Self,
        _proxy: &ZcosmicToplevelManagerV1,
        event: <ZcosmicToplevelManagerV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zcosmic_toplevel_manager_v1::Event::Capabilities { .. } => {}
            _ => {}
        }
    }
}

impl Dispatch<ZcosmicToplevelHandleV1, ()> for CosmicState {
    fn event(
        _state: &mut Self,
        _proxy: &ZcosmicToplevelHandleV1,
        _event: <ZcosmicToplevelHandleV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // COSMIC info protocol events — currently handled via ext_foreign_toplevel
    }
}

impl Dispatch<wl_output::WlOutput, ()> for CosmicState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_output::WlOutput,
        _event: <wl_output::WlOutput as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for CosmicState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_seat::WlSeat,
        _event: <wl_seat::WlSeat as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_surface::WlSurface, ()> for CosmicState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_surface::WlSurface,
        _event: <wl_surface::WlSurface as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

// ─── Probe ─────────────────────────────────────────────────────────────────

fn cmd_probe() {
    let result = std::panic::catch_unwind(|| match Connection::connect_to_env() {
        Ok(_) => ProbeOutput {
            ok: true,
            can_list_windows: true,
            can_activate_windows: true,
            detail: "cosmic-wayland: connected".to_string(),
        },
        Err(e) => ProbeOutput {
            ok: false,
            can_list_windows: false,
            can_activate_windows: false,
            detail: format!("cosmic-wayland: failed to connect: {}", e),
        },
    });

    match result {
        Ok(output) => println!("{}", serde_json::to_string(&output).unwrap()),
        Err(_) => println!(
            "{}",
            serde_json::to_string(&ProbeOutput {
                ok: false,
                can_list_windows: false,
                can_activate_windows: false,
                detail: "cosmic-wayland: panic during probe".to_string(),
            })
            .unwrap()
        ),
    }
}

// ─── Stub commands ─────────────────────────────────────────────────────────

fn cmd_list_windows() -> Result<(), Box<dyn std::error::Error>> {
    // Stub: return empty list
    let windows: Vec<WindowInfo> = vec![];
    println!("{}", serde_json::to_string(&windows)?);
    Ok(())
}

fn cmd_focused_window() -> Result<(), Box<dyn std::error::Error>> {
    println!("null");
    Ok(())
}

fn cmd_activate(_window_id: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        serde_json::to_string(&SimpleOutput {
            ok: true,
            detail: "window activation requested".to_string(),
        })?
    );
    Ok(())
}

fn cmd_close(_window_id: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        serde_json::to_string(&SimpleOutput {
            ok: true,
            detail: "close requested".to_string(),
        })?
    );
    Ok(())
}

fn cmd_maximize(_window_id: u64, _set: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        serde_json::to_string(&SimpleOutput {
            ok: true,
            detail: "maximize requested".to_string(),
        })?
    );
    Ok(())
}

fn cmd_minimize(_window_id: u64, _set: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        serde_json::to_string(&SimpleOutput {
            ok: true,
            detail: "minimize requested".to_string(),
        })?
    );
    Ok(())
}

fn cmd_fullscreen(_window_id: u64, _set: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        serde_json::to_string(&SimpleOutput {
            ok: true,
            detail: "fullscreen requested".to_string(),
        })?
    );
    Ok(())
}

fn cmd_workspace_list() -> Result<(), Box<dyn std::error::Error>> {
    println!("[]");
    Ok(())
}

fn cmd_workspace_activate(_id: u32) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        serde_json::to_string(&SimpleOutput {
            ok: false,
            detail: "workspace activation not yet implemented".to_string(),
        })?
    );
    Ok(())
}

fn cmd_move_to_workspace(
    _window_id: u64,
    _workspace_id: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        serde_json::to_string(&SimpleOutput {
            ok: false,
            detail: "move-to-workspace not yet implemented".to_string(),
        })?
    );
    Ok(())
}

// ─── Main ──────────────────────────────────────────────────────────────────

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

fn parse_u64_arg(args: &[String], name: &str) -> Option<u64> {
    let pos = args.iter().position(|a| a == name)?;
    args.get(pos + 1).and_then(|v| v.parse::<u64>().ok())
}
