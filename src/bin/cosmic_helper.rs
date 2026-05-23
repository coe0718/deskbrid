//! COSMIC desktop helper — bridges Wayland protocol to JSON-over-stdin/stdout.
//!
//! Usage: cosmic-helper <command> [options]
//!
//! Implements ext_foreign_toplevel_list_v1 for window listing and control
//! via the COSMIC Wayland compositor.

use serde::Serialize;
use std::collections::HashMap;
use std::process;

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

// ─── Wayland window listing via ext_foreign_toplevel_list_v1 ─────

use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle,
    protocol::wl_registry::{self, WlRegistry},
};

use wayland_client::backend::ObjectId;

use wayland_protocols::ext::foreign_toplevel_list::v1::client::{
    ext_foreign_toplevel_handle_v1::{self as toplevel_handle, ExtForeignToplevelHandleV1},
    ext_foreign_toplevel_list_v1::{self as toplevel_list, ExtForeignToplevelListV1},
};

/// State for the Wayland dispatch loop.
struct WlState {
    toplevel_list: Option<ExtForeignToplevelListV1>,
    /// Windows indexed by their Wayland ObjectId.
    windows: HashMap<ObjectId, WindowInfo>,
    /// Pending window data being accumulated before done event.
    pending: HashMap<ObjectId, PendingWindow>,
    /// Next numeric ID to assign.
    next_id: u64,
    /// Set to true when finished event is received.
    finished: bool,
}

struct PendingWindow {
    title: Option<String>,
    app_id: Option<String>,
    identifier: Option<String>,
    id: u64,
}

impl Dispatch<WlRegistry, ()> for WlState {
    fn event(
        state: &mut Self,
        registry: &WlRegistry,
        event: <WlRegistry as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            if interface == "ext_foreign_toplevel_list_v1" {
                let list = registry.bind::<ExtForeignToplevelListV1, (), Self>(
                    name,
                    version.min(1),
                    qh,
                    (),
                );
                state.toplevel_list = Some(list);
            }
        }
    }
}

use wayland_client::event_created_child;

impl Dispatch<ExtForeignToplevelListV1, ()> for WlState {
    event_created_child!(WlState, ExtForeignToplevelListV1, [
        0 => (ExtForeignToplevelHandleV1, ())
    ]);

    fn event(
        state: &mut Self,
        _proxy: &ExtForeignToplevelListV1,
        event: <ExtForeignToplevelListV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        use toplevel_list::Event;
        match event {
            Event::Toplevel { toplevel } => {
                let obj_id = toplevel.id();
                let window_id = state.next_id;
                state.next_id += 1;
                state.pending.insert(
                    obj_id,
                    PendingWindow {
                        title: None,
                        app_id: None,
                        identifier: None,
                        id: window_id,
                    },
                );
            }
            Event::Finished => {
                state.finished = true;
            }
            _ => {}
        }
    }
}

impl Dispatch<ExtForeignToplevelHandleV1, ()> for WlState {
    fn event(
        state: &mut Self,
        proxy: &ExtForeignToplevelHandleV1,
        event: <ExtForeignToplevelHandleV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        use toplevel_handle::Event;
        let obj_id = proxy.id();
        match event {
            Event::Title { title } => {
                if let Some(pending) = state.pending.get_mut(&obj_id) {
                    pending.title = Some(title);
                }
            }
            Event::AppId { app_id } => {
                if let Some(pending) = state.pending.get_mut(&obj_id) {
                    pending.app_id = Some(app_id);
                }
            }
            Event::Identifier { identifier } => {
                if let Some(pending) = state.pending.get_mut(&obj_id) {
                    pending.identifier = Some(identifier);
                }
            }
            Event::Done => {
                if let Some(pending) = state.pending.remove(&obj_id) {
                    let ident = pending.identifier.as_deref().unwrap_or("");
                    let numeric_id = if !ident.is_empty() {
                        let mut hash: u64 = 5381;
                        for b in ident.bytes() {
                            hash = hash.wrapping_mul(33).wrapping_add(b as u64);
                        }
                        hash
                    } else {
                        pending.id
                    };
                    let window = WindowInfo {
                        window_id: numeric_id,
                        title: pending.title.clone(),
                        app_id: pending.app_id.clone(),
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
                    };
                    state.windows.insert(obj_id, window);
                }
            }
            Event::Closed => {
                state.windows.remove(&obj_id);
            }
            _ => {}
        }
    }
}

fn list_windows_wayland() -> Vec<WindowInfo> {
    let conn = Connection::connect_to_env().expect("failed to connect to Wayland display");
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();
    let display = conn.display();

    let mut state = WlState {
        toplevel_list: None,
        windows: HashMap::new(),
        pending: HashMap::new(),
        next_id: 1,
        finished: false,
    };

    let _registry = display.get_registry(&qh, ());

    // Roundtrip to receive global announcements and bind protocol
    event_queue.roundtrip(&mut state).expect("roundtrip failed");

    // Roundtrips to get the toplevel list with all properties
    if state.toplevel_list.is_some() {
        event_queue.roundtrip(&mut state).expect("roundtrip failed");
        event_queue.roundtrip(&mut state).expect("roundtrip failed");
        // Flush remaining events
        event_queue.roundtrip(&mut state).expect("roundtrip failed");
    }

    state.windows.into_values().collect()
}

// ─── Stub commands ────────────────────────────────────

fn probe() {
    match std::env::var("WAYLAND_DISPLAY") {
        Ok(socket) => {
            let xdg = std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR must be set");
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
    let windows = list_windows_wayland();
    println!("{}", serde_json::to_string(&windows).unwrap());
}

fn focused_window() {
    println!("null");
}

fn activate(_window_id: u64) {
    ok_json(Some("activate stubbed"));
}

fn close(_window_id: u64) {
    ok_json(Some("close stubbed"));
}

fn set_maximized(window_id: u64, _on: bool) {
    ok_json(Some(&format!("maximize window_id={window_id} stubbed")));
}

fn set_minimized(window_id: u64, _on: bool) {
    ok_json(Some(&format!("minimize window_id={window_id} stubbed")));
}

fn set_fullscreen(window_id: u64, _on: bool) {
    ok_json(Some(&format!("fullscreen window_id={window_id} stubbed")));
}

fn workspace_list() {
    println!("[]");
}

fn workspace_activate(_id: u32) {
    ok_json(Some("workspace-activate stubbed"));
}

fn move_to_workspace(_window_id: u64, _workspace_id: u32) {
    ok_json(Some("move-to-workspace stubbed"));
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
