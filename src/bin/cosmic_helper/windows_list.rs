use std::collections::HashMap;

use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle,
    backend::ObjectId,
    event_created_child,
    protocol::wl_registry::{self, WlRegistry},
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

use crate::{id_from_identifier, types::WindowInfo};

// ─── Window listing state ────────────────────────────

pub(crate) struct ListState {
    pub(crate) toplevel_list: Option<ExtForeignToplevelListV1>,
    pub(crate) toplevel_info: Option<ZcosmicToplevelInfoV1>,
    pub(crate) windows: Vec<WindowInfo>,
    pub(crate) pending_ext: HashMap<ObjectId, PendingExt>,
    pub(crate) ext_id_map: HashMap<ObjectId, usize>,
    pub(crate) cosmic_id_map: HashMap<ObjectId, usize>,
    pub(crate) finished: bool,
}

pub(crate) struct PendingExt {
    pub(crate) window_idx: usize,
    pub(crate) title: Option<String>,
    pub(crate) app_id: Option<String>,
    pub(crate) identifier: Option<String>,
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
