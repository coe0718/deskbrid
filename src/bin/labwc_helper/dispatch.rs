//! Labwc Wayland protocol dispatch implementations.
//!
//! Handles wlr-foreign-toplevel-management and core Wayland protocol dispatching.

use super::LabwcState;
use wayland_client::{
    Connection, Dispatch, QueueHandle, protocol::wl_output, protocol::wl_seat, protocol::wl_surface,
};
use wayland_protocols::ext::foreign_toplevel_list::v1::client::{
    ext_foreign_toplevel_handle_v1::{self, ExtForeignToplevelHandleV1},
    ext_foreign_toplevel_list_v1::{self, ExtForeignToplevelListV1},
};

impl Dispatch<ExtForeignToplevelListV1, ()> for LabwcState {
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

impl Dispatch<ExtForeignToplevelHandleV1, ()> for LabwcState {
    fn event(
        state: &mut Self,
        proxy: &ExtForeignToplevelHandleV1,
        event: <ExtForeignToplevelHandleV1 as wayland_client::Proxy>::Event,
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
                        focused: false,
                        minimized: false,
                        maximized: false,
                        fullscreen: false,
                    });
                }
            }
            ext_foreign_toplevel_handle_v1::Event::Closed => {
                if let Some(&wid) = state.ext_handle_ids.get(&proxy_id) {
                    state.windows.remove(&wid);
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_output::WlOutput, ()> for LabwcState {
    fn event(
        _s: &mut Self,
        _p: &wl_output::WlOutput,
        _e: <wl_output::WlOutput as wayland_client::Proxy>::Event,
        _d: &(),
        _c: &Connection,
        _q: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<wl_seat::WlSeat, ()> for LabwcState {
    fn event(
        _s: &mut Self,
        _p: &wl_seat::WlSeat,
        _e: <wl_seat::WlSeat as wayland_client::Proxy>::Event,
        _d: &(),
        _c: &Connection,
        _q: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<wl_surface::WlSurface, ()> for LabwcState {
    fn event(
        _s: &mut Self,
        _p: &wl_surface::WlSurface,
        _e: <wl_surface::WlSurface as wayland_client::Proxy>::Event,
        _d: &(),
        _c: &Connection,
        _q: &QueueHandle<Self>,
    ) {
    }
}
