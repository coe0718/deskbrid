//! COSMIC desktop helper — bridges Wayland protocol to JSON-over-stdin/stdout.
//!
//! Uses ext_foreign_toplevel_list_v1 for window discovery and
//! zcosmic_toplevel_info_v1 + zcosmic_toplevel_manager_v1 for window
//! properties (state, geometry) and control (close, activate, etc.).
//!
//! Usage: cosmic-helper <command> [options]

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
#[derive(Serialize, Clone, Debug)]
#[allow(dead_code)]
struct WorkspaceInfo {
    id: u32,
    name: String,
    is_active: bool,
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

fn err_json(msg: &str) {
    println!("{{\"ok\": false, \"error\": \"{}\"}}", msg);
}

fn id_from_identifier(ident: &str) -> u64 {
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

// ─── Wayland protocol types ──────────────────────────

use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle,
    backend::ObjectId,
    event_created_child,
    protocol::{
        wl_output::WlOutput,
        wl_registry::{self, WlRegistry},
        wl_seat::WlSeat,
    },
};

use wayland_protocols::ext::foreign_toplevel_list::v1::client::{
    ext_foreign_toplevel_handle_v1::{self as ext_handle, ExtForeignToplevelHandleV1},
    ext_foreign_toplevel_list_v1::{self as ext_list, ExtForeignToplevelListV1},
};

use cosmic_protocols::toplevel_info::v1::client::{
    zcosmic_toplevel_handle_v1::{
        self as cosmic_handle, State as CosmicState, ZcosmicToplevelHandleV1,
    },
    zcosmic_toplevel_info_v1::ZcosmicToplevelInfoV1,
};

use cosmic_protocols::toplevel_management::v1::client::zcosmic_toplevel_manager_v1::ZcosmicToplevelManagerV1;

use wayland_protocols::ext::workspace::v1::client::{
    ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1,
    ext_workspace_handle_v1::{self as ws_handle, ExtWorkspaceHandleV1},
    ext_workspace_manager_v1::{self as ws_mgr, ExtWorkspaceManagerV1},
};

// ─── Window listing state ────────────────────────────

struct ListState {
    toplevel_list: Option<ExtForeignToplevelListV1>,
    toplevel_info: Option<ZcosmicToplevelInfoV1>,
    windows: Vec<WindowInfo>,
    pending_ext: HashMap<ObjectId, PendingExt>,
    ext_id_map: HashMap<ObjectId, usize>,
    cosmic_id_map: HashMap<ObjectId, usize>,
    finished: bool,
}

struct PendingExt {
    window_idx: usize,
    title: Option<String>,
    app_id: Option<String>,
    identifier: Option<String>,
}

impl Dispatch<WlRegistry, ()> for ListState {
    fn event(
        state: &mut Self,
        registry: &WlRegistry,
        event: <WlRegistry as Proxy>::Event,
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
            match interface.as_str() {
                "ext_foreign_toplevel_list_v1" => {
                    let list = registry.bind::<ExtForeignToplevelListV1, (), Self>(
                        name,
                        version.min(1),
                        qh,
                        (),
                    );
                    state.toplevel_list = Some(list);
                }
                "zcosmic_toplevel_info_v1" => {
                    let info = registry.bind::<ZcosmicToplevelInfoV1, (), Self>(
                        name,
                        version.min(2),
                        qh,
                        (),
                    );
                    state.toplevel_info = Some(info);
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<ExtForeignToplevelListV1, ()> for ListState {
    event_created_child!(ListState, ExtForeignToplevelListV1, [
        0 => (ExtForeignToplevelHandleV1, ())
    ]);

    fn event(
        state: &mut Self,
        _proxy: &ExtForeignToplevelListV1,
        event: <ExtForeignToplevelListV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            ext_list::Event::Toplevel { toplevel } => {
                let obj_id = toplevel.id();
                let idx = state.windows.len();
                state.windows.push(WindowInfo {
                    window_id: 0,
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
                state.pending_ext.insert(
                    obj_id.clone(),
                    PendingExt {
                        window_idx: idx,
                        title: None,
                        app_id: None,
                        identifier: None,
                    },
                );
                state.ext_id_map.insert(obj_id, idx);

                if let Some(info) = &state.toplevel_info {
                    let _cosmic_h = info.get_cosmic_toplevel(&toplevel, qh, ());
                }
            }
            ext_list::Event::Finished => {
                state.finished = true;
            }
            _ => {}
        }
    }
}

impl Dispatch<ExtForeignToplevelHandleV1, ()> for ListState {
    fn event(
        state: &mut Self,
        proxy: &ExtForeignToplevelHandleV1,
        event: <ExtForeignToplevelHandleV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        let obj_id = proxy.id();
        match event {
            ext_handle::Event::Title { title } => {
                if let Some(p) = state.pending_ext.get_mut(&obj_id) {
                    p.title = Some(title);
                }
            }
            ext_handle::Event::AppId { app_id } => {
                if let Some(p) = state.pending_ext.get_mut(&obj_id) {
                    p.app_id = Some(app_id);
                }
            }
            ext_handle::Event::Identifier { identifier } => {
                if let Some(p) = state.pending_ext.get_mut(&obj_id) {
                    p.identifier = Some(identifier);
                }
            }
            ext_handle::Event::Done => {
                if let Some(p) = state.pending_ext.remove(&obj_id)
                    && let Some(win) = state.windows.get_mut(p.window_idx)
                {
                    let ident = p.identifier.as_deref().unwrap_or("");
                    let nid = if !ident.is_empty() {
                        id_from_identifier(ident)
                    } else {
                        0
                    };
                    if nid != 0 {
                        win.window_id = nid;
                    }
                    win.title = p.title.clone();
                    win.app_id = p.app_id.clone();
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<ZcosmicToplevelInfoV1, ()> for ListState {
    fn event(
        _state: &mut Self,
        _proxy: &ZcosmicToplevelInfoV1,
        event: <ZcosmicToplevelInfoV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        let _ = event;
    }
}

impl Dispatch<ZcosmicToplevelHandleV1, ()> for ListState {
    fn event(
        state: &mut Self,
        proxy: &ZcosmicToplevelHandleV1,
        event: <ZcosmicToplevelHandleV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        let obj_id = proxy.id();
        match event {
            cosmic_handle::Event::State { state: raw_states } => {
                let states: Vec<CosmicState> = raw_states
                    .iter()
                    .filter_map(|&s| match s {
                        0 => Some(CosmicState::Maximized),
                        1 => Some(CosmicState::Minimized),
                        2 => Some(CosmicState::Activated),
                        3 => Some(CosmicState::Fullscreen),
                        _ => None,
                    })
                    .collect();

                if let Some(&idx) = state.cosmic_id_map.get(&obj_id) {
                    if let Some(win) = state.windows.get_mut(idx) {
                        win.focused = states.contains(&CosmicState::Activated);
                        win.minimized = states.contains(&CosmicState::Minimized);
                        win.maximized = states.contains(&CosmicState::Maximized);
                        win.fullscreen = states.contains(&CosmicState::Fullscreen);
                    }
                } else {
                    let idx = state.windows.len();
                    state.windows.push(WindowInfo {
                        window_id: 0,
                        title: None,
                        app_id: None,
                        pid: None,
                        x: None,
                        y: None,
                        width: None,
                        height: None,
                        focused: states.contains(&CosmicState::Activated),
                        minimized: states.contains(&CosmicState::Minimized),
                        maximized: states.contains(&CosmicState::Maximized),
                        fullscreen: states.contains(&CosmicState::Fullscreen),
                        workspace_id: None,
                    });
                    state.cosmic_id_map.insert(obj_id, idx);
                }
            }
            cosmic_handle::Event::Geometry {
                x,
                y,
                width,
                height,
                ..
            } => {
                if let Some(idx) = state.cosmic_id_map.get(&obj_id).copied()
                    && let Some(win) = state.windows.get_mut(idx)
                {
                    win.x = Some(x);
                    win.y = Some(y);
                    win.width = Some(width as u32);
                    win.height = Some(height as u32);
                }
            }
            _ => {}
        }
    }
}

// ─── Action state (close, activate, etc.) ────────────

/// State for performing a single window action.
/// Tracks both ext handles (for discovery) and cosmic handles (for control).
struct ActionState {
    toplevel_list: Option<ExtForeignToplevelListV1>,
    toplevel_info: Option<ZcosmicToplevelInfoV1>,
    manager: Option<ZcosmicToplevelManagerV1>,
    seat: Option<WlSeat>,
    target_id: u64,
    // ext_foreign_toplevel_handle_v1 -> its identifier hash
    ext_handles: HashMap<ObjectId, u64>,
    // zcosmic_toplevel_handle_v1 -> the matching ext handle id
    cosmic_handle_ids: HashMap<ObjectId, u64>,
    target_cosmic: Option<ZcosmicToplevelHandleV1>,
    got_globals: bool,
}

impl Dispatch<WlRegistry, ()> for ActionState {
    fn event(
        state: &mut Self,
        registry: &WlRegistry,
        event: <WlRegistry as Proxy>::Event,
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
            match interface.as_str() {
                "ext_foreign_toplevel_list_v1" => {
                    let list = registry.bind::<ExtForeignToplevelListV1, (), Self>(
                        name,
                        version.min(1),
                        qh,
                        (),
                    );
                    state.toplevel_list = Some(list);
                }
                "zcosmic_toplevel_info_v1" => {
                    let info = registry.bind::<ZcosmicToplevelInfoV1, (), Self>(
                        name,
                        version.min(2),
                        qh,
                        (),
                    );
                    state.toplevel_info = Some(info);
                }
                "zcosmic_toplevel_manager_v1" => {
                    let mgr = registry.bind::<ZcosmicToplevelManagerV1, (), Self>(
                        name,
                        version.min(4),
                        qh,
                        (),
                    );
                    state.manager = Some(mgr);
                }
                "wl_seat" => {
                    let seat = registry.bind::<WlSeat, (), Self>(name, version.min(1), qh, ());
                    state.seat = Some(seat);
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<WlSeat, ()> for ActionState {
    fn event(
        _state: &mut Self,
        _proxy: &WlSeat,
        _event: <WlSeat as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtForeignToplevelListV1, ()> for ActionState {
    event_created_child!(ActionState, ExtForeignToplevelListV1, [
        0 => (ExtForeignToplevelHandleV1, ())
    ]);

    fn event(
        state: &mut Self,
        _proxy: &ExtForeignToplevelListV1,
        event: <ExtForeignToplevelListV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let ext_list::Event::Toplevel { toplevel } = event {
            let obj_id = toplevel.id();
            state.ext_handles.insert(obj_id, 0);
            if let Some(info) = &state.toplevel_info {
                let _cosmic_h = info.get_cosmic_toplevel(&toplevel, qh, ());
            }
        }
    }
}

impl Dispatch<ExtForeignToplevelHandleV1, ()> for ActionState {
    fn event(
        state: &mut Self,
        proxy: &ExtForeignToplevelHandleV1,
        event: <ExtForeignToplevelHandleV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let ext_handle::Event::Identifier { identifier } = event {
            let nid = id_from_identifier(&identifier);
            let obj_id = proxy.id();
            state.ext_handles.insert(obj_id, nid);
            if nid != 0 && nid == state.target_id {
                state.got_globals = true;
            }
        }
    }
}

impl Dispatch<ZcosmicToplevelInfoV1, ()> for ActionState {
    fn event(
        _state: &mut Self,
        _proxy: &ZcosmicToplevelInfoV1,
        _event: <ZcosmicToplevelInfoV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZcosmicToplevelHandleV1, ()> for ActionState {
    fn event(
        state: &mut Self,
        proxy: &ZcosmicToplevelHandleV1,
        _event: <ZcosmicToplevelHandleV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // COSMIC handle created — check if it matches our target
        let obj_id = proxy.id();
        // We can't directly map cosmic handle -> ext handle in all cases,
        // so look through ext handles for a match
        if state.target_cosmic.is_none() {
            // Check if there's an ext handle with our target id
            if state.ext_handles.values().any(|&id| id == state.target_id) {
                state.target_cosmic = Some(proxy.clone());
            }
            // Also handle small sequential IDs
            if state.target_id > 0 && state.target_id < 100 {
                state.target_cosmic = Some(proxy.clone());
            }
        }
        state.cosmic_handle_ids.insert(obj_id, state.target_id);
    }
}

impl Dispatch<ZcosmicToplevelManagerV1, ()> for ActionState {
    fn event(
        _state: &mut Self,
        _proxy: &ZcosmicToplevelManagerV1,
        _event: <ZcosmicToplevelManagerV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

#[allow(clippy::type_complexity)]
fn do_action(
    window_id: u64,
    f: Box<dyn FnOnce(&ZcosmicToplevelManagerV1, &ZcosmicToplevelHandleV1)>,
) {
    let conn = match Connection::connect_to_env() {
        Ok(c) => c,
        Err(_) => {
            err_json("cannot connect to Wayland display");
            return;
        }
    };
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();
    let display = conn.display();

    let mut state = ActionState {
        toplevel_list: None,
        toplevel_info: None,
        manager: None,
        seat: None,
        target_id: window_id,
        ext_handles: HashMap::new(),
        cosmic_handle_ids: HashMap::new(),
        target_cosmic: None,
        got_globals: false,
    };

    let _registry = display.get_registry(&qh, ());
    event_queue.roundtrip(&mut state).expect("roundtrip failed");
    event_queue.roundtrip(&mut state).expect("roundtrip failed");
    event_queue.roundtrip(&mut state).expect("roundtrip failed");
    event_queue.roundtrip(&mut state).expect("roundtrip failed");

    if let Some(mgr) = &state.manager {
        if let Some(handle) = &state.target_cosmic {
            f(mgr, handle);
            event_queue.roundtrip(&mut state).ok();
            ok_json(None);
        } else {
            err_json(&format!("window {} not found", window_id));
        }
    } else {
        err_json("zcosmic_toplevel_manager_v1 not available");
    }
}

/// Like do_action, but also provides the wl_seat for activate() calls.
fn do_action_with_seat(window_id: u64) {
    let conn = match Connection::connect_to_env() {
        Ok(c) => c,
        Err(_) => {
            err_json("cannot connect to Wayland display");
            return;
        }
    };
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();
    let display = conn.display();

    let mut state = ActionState {
        toplevel_list: None,
        toplevel_info: None,
        manager: None,
        seat: None,
        target_id: window_id,
        ext_handles: HashMap::new(),
        cosmic_handle_ids: HashMap::new(),
        target_cosmic: None,
        got_globals: false,
    };

    let _registry = display.get_registry(&qh, ());
    event_queue.roundtrip(&mut state).expect("roundtrip failed");
    event_queue.roundtrip(&mut state).expect("roundtrip failed");
    event_queue.roundtrip(&mut state).expect("roundtrip failed");
    event_queue.roundtrip(&mut state).expect("roundtrip failed");

    if let (Some(mgr), Some(handle)) = (&state.manager, &state.target_cosmic) {
        if let Some(seat) = &state.seat {
            mgr.activate(handle, seat);
            event_queue.roundtrip(&mut state).ok();
            ok_json(None);
        } else {
            err_json("no wl_seat available for activate — compositor may not support it");
        }
    } else if state.manager.is_none() {
        err_json("zcosmic_toplevel_manager_v1 not available");
    } else {
        err_json(&format!("window {} not found", window_id));
    }
}

// ─── Workspace list state ────────────────────────────

struct WorkspaceListState {
    manager: Option<ExtWorkspaceManagerV1>,
    workspaces: Vec<WorkspaceInfo>,
    pending: HashMap<ObjectId, PendingWorkspace>,
    next_id: u32,
    finished: bool,
}

struct PendingWorkspace {
    id: u32,
    name: Option<String>,
    is_active: bool,
}

impl Dispatch<WlRegistry, ()> for WorkspaceListState {
    fn event(
        state: &mut Self,
        registry: &WlRegistry,
        event: <WlRegistry as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
            && interface.as_str() == "ext_workspace_manager_v1"
        {
            let mgr =
                registry.bind::<ExtWorkspaceManagerV1, (), Self>(name, version.min(1), qh, ());
            state.manager = Some(mgr);
        }
    }
}

impl Dispatch<ExtWorkspaceManagerV1, ()> for WorkspaceListState {
    event_created_child!(WorkspaceListState, ExtWorkspaceManagerV1, [
        0 => (ExtWorkspaceGroupHandleV1, ()),
        1 => (ExtWorkspaceHandleV1, ())
    ]);

    fn event(
        state: &mut Self,
        _proxy: &ExtWorkspaceManagerV1,
        event: <ExtWorkspaceManagerV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            ws_mgr::Event::Workspace { workspace } => {
                let id = state.next_id;
                state.next_id += 1;
                let obj_id = workspace.id();
                state.pending.insert(
                    obj_id,
                    PendingWorkspace {
                        id,
                        name: None,
                        is_active: false,
                    },
                );
            }
            ws_mgr::Event::WorkspaceGroup { .. } => {
                // We don't need group info for listing
            }
            ws_mgr::Event::Done => {
                // All workspace info for this batch has been sent
            }
            ws_mgr::Event::Finished => {
                state.finished = true;
            }
            _ => {}
        }
    }
}

impl Dispatch<ExtWorkspaceHandleV1, ()> for WorkspaceListState {
    fn event(
        state: &mut Self,
        proxy: &ExtWorkspaceHandleV1,
        event: <ExtWorkspaceHandleV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        let obj_id = proxy.id();
        match event {
            ws_handle::Event::Name { name } => {
                if let Some(p) = state.pending.get_mut(&obj_id) {
                    p.name = Some(name);
                }
            }
            ws_handle::Event::State { state: raw_state } => {
                // state is a bitfield: 1=active, 2=urgent, 4=hidden
                if let Some(p) = state.pending.get_mut(&obj_id) {
                    use wayland_client::WEnum;
                    let bits = match raw_state {
                        WEnum::Value(s) => u32::from(s),
                        WEnum::Unknown(v) => v,
                    };
                    p.is_active = (bits & 1) != 0;
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<ExtWorkspaceGroupHandleV1, ()> for WorkspaceListState {
    fn event(
        _state: &mut Self,
        _proxy: &ExtWorkspaceGroupHandleV1,
        _event: <ExtWorkspaceGroupHandleV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

// ─── Workspace action state (activate, move) ────────

struct WorkspaceActState {
    manager: Option<ExtWorkspaceManagerV1>,
    output: Option<WlOutput>,
    target_id: u32,
    target_workspace: Option<ExtWorkspaceHandleV1>,
    workspace_map: HashMap<ObjectId, u32>,
    next_id: u32,
}

impl Dispatch<WlRegistry, ()> for WorkspaceActState {
    fn event(
        state: &mut Self,
        registry: &WlRegistry,
        event: <WlRegistry as Proxy>::Event,
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
            match interface.as_str() {
                "ext_workspace_manager_v1" => {
                    let mgr = registry.bind::<ExtWorkspaceManagerV1, (), Self>(
                        name,
                        version.min(1),
                        qh,
                        (),
                    );
                    state.manager = Some(mgr);
                }
                "wl_output" => {
                    if state.output.is_none() {
                        let output =
                            registry.bind::<WlOutput, (), Self>(name, version.min(1), qh, ());
                        state.output = Some(output);
                    }
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<ExtWorkspaceManagerV1, ()> for WorkspaceActState {
    event_created_child!(WorkspaceActState, ExtWorkspaceManagerV1, [
        0 => (ExtWorkspaceGroupHandleV1, ()),
        1 => (ExtWorkspaceHandleV1, ())
    ]);

    fn event(
        state: &mut Self,
        _proxy: &ExtWorkspaceManagerV1,
        event: <ExtWorkspaceManagerV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            ws_mgr::Event::Workspace { workspace } => {
                let id = state.next_id;
                state.next_id += 1;
                let obj_id = workspace.id();
                state.workspace_map.insert(obj_id, id);
                if id == state.target_id {
                    state.target_workspace = Some(workspace);
                }
            }
            ws_mgr::Event::WorkspaceGroup { .. } => {}
            _ => {}
        }
    }
}

impl Dispatch<ExtWorkspaceHandleV1, ()> for WorkspaceActState {
    fn event(
        _state: &mut Self,
        _proxy: &ExtWorkspaceHandleV1,
        _event: <ExtWorkspaceHandleV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtWorkspaceGroupHandleV1, ()> for WorkspaceActState {
    fn event(
        _state: &mut Self,
        _proxy: &ExtWorkspaceGroupHandleV1,
        _event: <ExtWorkspaceGroupHandleV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlOutput, ()> for WorkspaceActState {
    fn event(
        _state: &mut Self,
        _proxy: &WlOutput,
        _event: <WlOutput as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}
// ─── Command implementations ─────────────────────────

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
    let conn = Connection::connect_to_env().expect("failed to connect to Wayland display");
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();
    let display = conn.display();

    let mut state = ListState {
        toplevel_list: None,
        toplevel_info: None,
        windows: Vec::new(),
        pending_ext: HashMap::new(),
        ext_id_map: HashMap::new(),
        cosmic_id_map: HashMap::new(),
        finished: false,
    };

    let _registry = display.get_registry(&qh, ());
    event_queue.roundtrip(&mut state).expect("roundtrip failed");
    for _ in 0..5 {
        event_queue.roundtrip(&mut state).expect("roundtrip failed");
        if state.finished && state.pending_ext.is_empty() {
            break;
        }
    }

    let mut fallback_id = 1;
    for win in state.windows.iter_mut() {
        if win.window_id == 0 {
            win.window_id = fallback_id;
            fallback_id += 1;
        }
    }

    println!("{}", serde_json::to_string(&state.windows).unwrap());
}

fn focused_window() {
    let conn = Connection::connect_to_env().expect("failed to connect to Wayland display");
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();
    let display = conn.display();

    let mut state = ListState {
        toplevel_list: None,
        toplevel_info: None,
        windows: Vec::new(),
        pending_ext: HashMap::new(),
        ext_id_map: HashMap::new(),
        cosmic_id_map: HashMap::new(),
        finished: false,
    };

    let _registry = display.get_registry(&qh, ());
    event_queue.roundtrip(&mut state).expect("roundtrip failed");
    for _ in 0..5 {
        event_queue.roundtrip(&mut state).expect("roundtrip failed");
        if state.finished && state.pending_ext.is_empty() {
            break;
        }
    }

    let focused = state.windows.iter().find(|w| w.focused);
    match focused {
        Some(win) => println!("{}", serde_json::to_string(win).unwrap()),
        None => println!("null"),
    }
}

fn close_helper(window_id: u64) {
    do_action(
        window_id,
        Box::new(|mgr, handle| {
            mgr.close(handle);
        }),
    );
}

fn activate_helper(window_id: u64) {
    // Activate with the first available seat
    // The do_action function will populate state.seat with a wl_seat if available
    do_action_with_seat(window_id);
}

fn set_maximized(window_id: u64, on: bool) {
    if on {
        do_action(window_id, Box::new(|mgr, handle| mgr.set_maximized(handle)));
    } else {
        do_action(
            window_id,
            Box::new(|mgr, handle| mgr.unset_maximized(handle)),
        );
    }
}

fn set_minimized(window_id: u64, on: bool) {
    if on {
        do_action(window_id, Box::new(|mgr, handle| mgr.set_minimized(handle)));
    } else {
        do_action(
            window_id,
            Box::new(|mgr, handle| mgr.unset_minimized(handle)),
        );
    }
}

fn set_fullscreen_act(window_id: u64, on: bool) {
    if on {
        do_action(
            window_id,
            Box::new(|mgr, handle| mgr.set_fullscreen(handle, None)),
        );
    } else {
        do_action(
            window_id,
            Box::new(|mgr, handle| mgr.unset_fullscreen(handle)),
        );
    }
}

fn workspace_list() {
    let conn = match Connection::connect_to_env() {
        Ok(c) => c,
        Err(_) => {
            err_json("cannot connect to Wayland display");
            return;
        }
    };
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();
    let display = conn.display();

    let mut state = WorkspaceListState {
        manager: None,
        workspaces: Vec::new(),
        pending: HashMap::new(),
        next_id: 1,
        finished: false,
    };

    let _registry = display.get_registry(&qh, ());
    event_queue.roundtrip(&mut state).expect("roundtrip failed");
    for _ in 0..5 {
        event_queue.roundtrip(&mut state).expect("roundtrip failed");
        if state.finished {
            break;
        }
    }

    // Move all pending workspaces into the final list
    for (_key, p) in state.pending.drain() {
        state.workspaces.push(WorkspaceInfo {
            id: p.id,
            name: p.name.unwrap_or_else(|| format!("Workspace {}", p.id)),
            is_active: p.is_active,
        });
    }

    println!("{}", serde_json::to_string(&state.workspaces).unwrap());
}

fn workspace_activate(id: u32) {
    let conn = match Connection::connect_to_env() {
        Ok(c) => c,
        Err(_) => {
            err_json("cannot connect to Wayland display");
            return;
        }
    };
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();
    let display = conn.display();

    let mut state = WorkspaceActState {
        manager: None,
        output: None,
        target_id: id,
        target_workspace: None,
        workspace_map: HashMap::new(),
        next_id: 1,
    };

    let _registry = display.get_registry(&qh, ());
    event_queue.roundtrip(&mut state).expect("roundtrip failed");
    event_queue.roundtrip(&mut state).expect("roundtrip failed");

    if let (Some(mgr), Some(ws)) = (&state.manager, &state.target_workspace) {
        ws.activate();
        mgr.commit();
        event_queue.roundtrip(&mut state).ok();
        ok_json(None);
    } else if state.manager.is_none() {
        err_json("ext_workspace_manager_v1 not available");
    } else {
        err_json(&format!("workspace {} not found", id));
    }
}

fn move_to_workspace(window_id: u64, workspace_id: u32) {
    let conn = match Connection::connect_to_env() {
        Ok(c) => c,
        Err(_) => {
            err_json("cannot connect to Wayland display");
            return;
        }
    };
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();
    let display = conn.display();

    // First discover workspaces to find the target
    let mut state = WorkspaceActState {
        manager: None,
        output: None,
        target_id: workspace_id,
        target_workspace: None,
        workspace_map: HashMap::new(),
        next_id: 1,
    };

    let _registry = display.get_registry(&qh, ());
    event_queue.roundtrip(&mut state).expect("roundtrip failed");
    event_queue.roundtrip(&mut state).expect("roundtrip failed");
    event_queue.roundtrip(&mut state).expect("roundtrip failed");

    let ws_handle = state.target_workspace.clone();

    if ws_handle.is_none() {
        err_json(&format!("workspace {} not found", workspace_id));
        return;
    }

    // Now find the toplevel and call move_to_ext_workspace
    let output = state.output.clone();
    do_action(
        window_id,
        Box::new(move |mgr, handle| {
            if let Some(ws) = &ws_handle
                && let Some(out) = &output
            {
                mgr.move_to_ext_workspace(handle, ws, out);
            }
        }),
    )
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
            activate_helper(wid);
        }
        "close" => {
            let wid = parse_u64_arg(&args, "--window-id").unwrap_or(0);
            close_helper(wid);
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
            set_fullscreen_act(wid, true);
        }
        "unfullscreen" => {
            let wid = parse_u64_arg(&args, "--window-id").unwrap_or(0);
            set_fullscreen_act(wid, false);
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
