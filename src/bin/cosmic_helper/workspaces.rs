use std::collections::HashMap;

use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle,
    backend::ObjectId,
    event_created_child,
    protocol::{
        wl_output::WlOutput,
        wl_registry::{self, WlRegistry},
    },
};

use wayland_protocols::ext::workspace::v1::client::{
    ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1,
    ext_workspace_handle_v1::{self as ws_handle, ExtWorkspaceHandleV1},
    ext_workspace_manager_v1::{self as ws_mgr, ExtWorkspaceManagerV1},
};

use crate::types::WorkspaceInfo;

// ─── Workspace list state ────────────────────────────

pub(crate) struct WorkspaceListState {
    pub(crate) manager: Option<ExtWorkspaceManagerV1>,
    pub(crate) workspaces: Vec<WorkspaceInfo>,
    pub(crate) pending: HashMap<ObjectId, PendingWorkspace>,
    pub(crate) next_id: u32,
    pub(crate) finished: bool,
}

pub(crate) struct PendingWorkspace {
    pub(crate) id: u32,
    pub(crate) name: Option<String>,
    pub(crate) is_active: bool,
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

pub(crate) struct WorkspaceActState {
    pub(crate) manager: Option<ExtWorkspaceManagerV1>,
    pub(crate) output: Option<WlOutput>,
    pub(crate) target_id: u32,
    pub(crate) target_workspace: Option<ExtWorkspaceHandleV1>,
    pub(crate) workspace_map: HashMap<ObjectId, u32>,
    pub(crate) next_id: u32,
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
                "wl_output" if state.output.is_none() => {
                    let output = registry.bind::<WlOutput, (), Self>(name, version.min(1), qh, ());
                    state.output = Some(output);
                }
                "wl_output" => {}
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
