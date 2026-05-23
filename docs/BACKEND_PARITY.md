# Backend Parity Matrix

This matrix tracks `DesktopBackend` coverage by compositor/backend. It is a code
audit of the current implementation, not a runtime certification on every distro.

Legend: `Y` = implemented, `D` = degraded or helper-dependent, `N` = explicitly
unsupported, `T` = implemented through common Linux tools such as `nmcli`,
`bluetoothctl`, `pactl`, `find`, or `notify`.

## High Priority Agent Actions

| Method | GNOME | KDE | Hyprland | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| windows_list | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| window_focus | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| window_get | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| window_close | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| window_minimize | Y | Y | N | Y | Y | N | Y | N | Y |
| window_maximize | Y | Y | Y | Y | Y | D | Y | Y | Y |
| window_move_resize | Y | Y | Y | N | Y | D | N | N | Y |
| workspaces_list | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| workspace_switch | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| workspace_move_window | D | Y | Y | Y | Y | Y | Y | Y | Y |
| keyboard_type/key/combo | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| mouse_move/click/scroll | D | Y | Y | Y | Y | Y | Y | Y | Y |
| clipboard_read/write | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| screenshot | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| notification_send | T | T | T | T | T | T | T | T | T |
| notification_close | Y | N | Y | N | Y | Y | Y | Y | N |

## System And Device Domains

| Method | GNOME | KDE | Hyprland | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| system_info | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| idle_seconds | Y | Y | Y | Y | Y | Y | Y | Y | T |
| power_action | T | T | T | T | T | T | T | T | T |
| battery_status | T | T | T | T | T | T | T | T | T |
| network_status | Y | T | T | T | T | T | T | T | T |
| network_interfaces | Y | T | T | T | T | T | T | T | T |
| wifi_scan | Y | T | T | T | T | T | T | T | T |
| wifi_connect | T | T | T | T | T | T | T | T | T |
| bluetooth_list | Y | T | T | T | T | T | T | T | T |
| bluetooth_scan | Y | T | T | T | T | T | T | T | T |
| bluetooth_stop_scan | Y | T | T | T | T | T | T | T | T |
| bluetooth_connect | Y | T | T | T | T | T | T | T | T |
| bluetooth_disconnect | Y | T | T | T | T | T | T | T | T |
| files_watch | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| files_unwatch | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| files_search | T | T | T | T | T | T | T | T | T |
| audio_list_sinks | T | T | T | T | T | T | T | T | T |
| audio_set_sink_volume | T | T | T | T | T | T | T | T | T |

## Monitor Control

| Method | GNOME | KDE | Hyprland | COSMIC | Sway | Niri | Wayfire | Labwc | X11 |
|---|---|---|---|---|---|---|---|---|---|
| monitor_set_primary | D | Y | N | D | Y | N | N | N | Y |
| monitor_set_resolution | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| monitor_set_scale | Y | Y | Y | Y | Y | Y | Y | Y | D |
| monitor_set_rotation | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| monitor_set_enabled | Y | Y | Y | Y | Y | Y | Y | Y | Y |

## Explicit Limitations

| Backend | Limitation | Capability reason |
|---|---|---|
| Hyprland | No native minimize dispatcher | `hyprland_has_no_native_minimize_dispatcher` |
| COSMIC | No window move/resize IPC yet | `cosmic_move_resize_not_available` |
| COSMIC | `notify-send` cannot close notifications by ID | `cosmic_notify_send_close_unsupported` |
| Niri | No minimize concept | `niri_has_no_minimize_concept` |
| Niri | Move/resize and tiling only set column width | `niri_only_sets_column_width` |
| Hyprland, Niri, Wayfire, Labwc | No primary monitor concept exposed | backend-specific `*_has_no_primary_monitor_setting` |
| Wayfire | `wf-ipc` does not expose move/resize | `wf_ipc_move_resize_not_available` |
| Labwc | `wlrctl` does not expose move/resize or minimize | `wlrctl_move_resize_not_available`, `wlrctl_minimize_not_available` |
| X11 | Notification close is unavailable through `notify-send` | `x11_unsupported` |
