//! COSMIC Wayland protocol dispatch implementations.

use super::CosmicState;
use cosmic_protocols::toplevel_info::v1::client::zcosmic_toplevel_handle_v1::ZcosmicToplevelHandleV1;
use cosmic_protocols::toplevel_management::v1::client::zcosmic_toplevel_manager_v1;
use cosmic_protocols::toplevel_management::v1::client::zcosmic_toplevel_manager_v1::ZcosmicToplevelManagerV1;
use wayland_client::{
    Connection, Dispatch, QueueHandle, protocol::wl_output, protocol::wl_seat, protocol::wl_surface,
};
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1;
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_list_v1;
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1;

impl Dispatch<ExtForeignToplevelListV1, ()> for CosmicState {
    fn event(
        state: &mut Self,
        _proxy: &ExtForeignToplevelListV1,
        event: <ExtForeignToplevelListV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let ext_foreign_toplevel_list_v1::Event::Finished = event {
            state.done = true;
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
        let proxy_id = proxy as *const _ as u64;

        match event {
            ext_foreign_toplevel_handle_v1::Event::Title { title } => {
                if let Some(&wid) = state.ext_handle_ids.get(&proxy_id)
                    && let Some(w) = state.windows.get_mut(&wid)
                {
                    w.title = Some(title);
                }
            }
            ext_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                if let Some(&wid) = state.ext_handle_ids.get(&proxy_id)
                    && let Some(w) = state.windows.get_mut(&wid)
                {
                    w.app_id = Some(app_id);
                }
            }
            ext_foreign_toplevel_handle_v1::Event::Identifier { identifier } => {
                let num_id: u64 = identifier.parse().unwrap_or(0);
                if num_id > 0 {
                    state.ext_handle_ids.insert(proxy_id, num_id);
                    state.windows.entry(num_id).or_insert(super::WindowInfo {
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
            ext_foreign_toplevel_handle_v1::Event::Done => {}
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
        if let zcosmic_toplevel_manager_v1::Event::Capabilities { .. } = event {}
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
