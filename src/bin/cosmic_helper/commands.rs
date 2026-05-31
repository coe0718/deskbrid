use std::collections::HashMap;

use wayland_client::Connection;

use crate::{
    err_json, ok_json,
    types::WorkspaceInfo,
    windows_action::{do_action, do_action_with_seat},
    windows_list::ListState,
    workspaces::{WorkspaceActState, WorkspaceListState},
};

// ─── Command implementations ─────────────────────────

pub(crate) fn probe() {
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

pub(crate) fn list_windows() {
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

pub(crate) fn focused_window() {
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

pub(crate) fn close_window(window_id: u64) {
    do_action(
        window_id,
        Box::new(|mgr, handle| {
            mgr.close(handle);
        }),
    );
}

pub(crate) fn activate_window(window_id: u64) {
    do_action_with_seat(window_id);
}

pub(crate) fn set_maximized(window_id: u64, on: bool) {
    if on {
        do_action(window_id, Box::new(|mgr, handle| mgr.set_maximized(handle)));
    } else {
        do_action(
            window_id,
            Box::new(|mgr, handle| {
                mgr.unset_maximized(handle);
            }),
        );
    }
}

pub(crate) fn set_minimized(window_id: u64, on: bool) {
    if on {
        do_action(window_id, Box::new(|mgr, handle| mgr.set_minimized(handle)));
    } else {
        do_action(
            window_id,
            Box::new(|mgr, handle| {
                mgr.unset_minimized(handle);
            }),
        );
    }
}

pub(crate) fn set_fullscreen(window_id: u64, on: bool) {
    if on {
        do_action(
            window_id,
            Box::new(|mgr, handle| {
                mgr.set_fullscreen(handle, None);
            }),
        );
    } else {
        do_action(
            window_id,
            Box::new(|mgr, handle| {
                mgr.unset_fullscreen(handle);
            }),
        );
    }
}

pub(crate) fn workspace_list() {
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

pub(crate) fn workspace_activate(id: u32) {
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

pub(crate) fn move_to_workspace(window_id: u64, workspace_id: u32) {
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
