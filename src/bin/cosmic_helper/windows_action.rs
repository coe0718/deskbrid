use std::collections::HashMap;

use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle,
    backend::ObjectId,
    event_created_child,
    protocol::{
        wl_registry::{self, WlRegistry},
        wl_seat::WlSeat,
    },
};

use wayland_protocols::ext::foreign_toplevel_list::v1::client::{
    ext_foreign_toplevel_handle_v1::{self as ext_handle, ExtForeignToplevelHandleV1},
    ext_foreign_toplevel_list_v1::{self as ext_list, ExtForeignToplevelListV1},
};

use cosmic_protocols::toplevel_info::v1::client::{
    zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1,
    zcosmic_toplevel_info_v1::ZcosmicToplevelInfoV1,
};

use cosmic_protocols::toplevel_management::v1::client::zcosmic_toplevel_manager_v1::ZcosmicToplevelManagerV1;

use crate::{err_json, id_from_identifier, ok_json};

// ─── Action state (close, activate, etc.) ────────────

/// State for performing a single window action.
/// Tracks both ext handles (for discovery) and cosmic handles (for control).
pub(crate) struct ActionState {
    pub(crate) toplevel_list: Option<ExtForeignToplevelListV1>,
    pub(crate) toplevel_info: Option<ZcosmicToplevelInfoV1>,
    pub(crate) manager: Option<ZcosmicToplevelManagerV1>,
    pub(crate) seat: Option<WlSeat>,
    pub(crate) target_id: u64,
    // ext_foreign_toplevel_handle_v1 -> its identifier hash
    pub(crate) ext_handles: HashMap<ObjectId, u64>,
    // ext_foreign_toplevel_handle_v1 -> zcosmic_toplevel_handle_v1
    pub(crate) ext_cosmic_map: HashMap<ObjectId, ZcosmicToplevelHandleV1>,
    pub(crate) target_cosmic: Option<ZcosmicToplevelHandleV1>,
    pub(crate) got_globals: bool,
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
            state.ext_handles.insert(obj_id.clone(), 0);
            if let Some(info) = &state.toplevel_info {
                let cosmic_h = info.get_cosmic_toplevel(&toplevel, qh, ());
                state.ext_cosmic_map.insert(obj_id, cosmic_h);
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
            state.ext_handles.insert(obj_id.clone(), nid);
            if nid != 0 && nid == state.target_id {
                state.got_globals = true;
                if let Some(cosmic) = state.ext_cosmic_map.get(&obj_id) {
                    state.target_cosmic = Some(cosmic.clone());
                }
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
        _state: &mut Self,
        _proxy: &ZcosmicToplevelHandleV1,
        _event: <ZcosmicToplevelHandleV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // COSMIC handle matching is done via ext_cosmic_map in the Identifier handler
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
pub(crate) fn do_action(
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
        ext_cosmic_map: HashMap::new(),
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
pub(crate) fn do_action_with_seat(window_id: u64) {
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
        ext_cosmic_map: HashMap::new(),
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
