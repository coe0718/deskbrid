// deskbrid@deskbrid — GNOME Shell extension for window management over DBus
// GNOME 46+ compatible (ES module import syntax)
// Provides: ListWindows, FocusedWindow, FocusWindow, window actions, workspaces, WindowStateChanged signal

import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';

const DBUS_SERVICE = 'org.deskbrid.WindowManager';
const DBUS_PATH = '/org/deskbrid/WindowManager';
const DBUS_IFACE = 'org.deskbrid.WindowManager';

const DeskbridIface = `
<node>
  <interface name="${DBUS_IFACE}">
    <method name="ListWindows">
      <arg type="s" name="result" direction="out"/>
    </method>
    <method name="FocusedWindow">
      <arg type="s" name="result" direction="out"/>
    </method>
    <method name="FocusWindow">
      <arg type="s" name="app_id" direction="in"/>
      <arg type="s" name="title" direction="in"/>
      <arg type="b" name="exact" direction="in"/>
      <arg type="b" name="success" direction="out"/>
    </method>
    <method name="CloseWindow">
      <arg type="s" name="window_id" direction="in"/>
      <arg type="b" name="success" direction="out"/>
    </method>
    <method name="MinimizeWindow">
      <arg type="s" name="window_id" direction="in"/>
      <arg type="b" name="success" direction="out"/>
    </method>
    <method name="MaximizeWindow">
      <arg type="s" name="window_id" direction="in"/>
      <arg type="b" name="success" direction="out"/>
    </method>
    <method name="MoveResizeWindow">
      <arg type="s" name="window_id" direction="in"/>
      <arg type="i" name="x" direction="in"/>
      <arg type="i" name="y" direction="in"/>
      <arg type="u" name="width" direction="in"/>
      <arg type="u" name="height" direction="in"/>
      <arg type="b" name="success" direction="out"/>
    </method>
    <method name="ListWorkspaces">
      <arg type="s" name="result" direction="out"/>
    </method>
    <method name="SwitchWorkspace">
      <arg type="u" name="index" direction="in"/>
    </method>
    <method name="MoveWindowToWorkspace">
      <arg type="s" name="window_id" direction="in"/>
      <arg type="u" name="workspace_index" direction="in"/>
      <arg type="b" name="success" direction="out"/>
    </method>
    <signal name="WindowStateChanged">
      <arg type="s" name="window_info"/>
    </signal>
  </interface>
</node>`;

let _debounceTimer = 0;
let _dbusImpl = null;
let _dbusId = 0;
let _focusSignalId = 0;
let _windowCreatedId = 0;
let _extensionInstance = null;  // GC root — prevent GJS from sweeping the instance

function serializeWindow(metaWindow) {
    if (!metaWindow) return 'null';
    const rect = metaWindow.get_frame_rect();
    return JSON.stringify({
        id: metaWindow.get_stable_sequence?.() ?? 0,
        title: metaWindow.get_title() || '',
        app_id: metaWindow.get_wm_class() || '',
        pid: metaWindow.get_pid() || 0,
        workspace_index: metaWindow.get_workspace()?.index() ?? 0,
        focused: global.display.focus_window === metaWindow,
        minimized: metaWindow.minimized,
        geometry: [rect.x, rect.y, rect.width, rect.height],
    });
}

function serializeWindows(windows) {
    return JSON.stringify(windows.map(w => {
        const m = w.meta_window;
        return JSON.parse(serializeWindow(m));
    }));
}

function serializeWorkspaces() {
    const wm = global.workspace_manager;
    const workspaces = [];
    for (let i = 0; i < wm.n_workspaces; i++) {
        const ws = wm.get_workspace_by_index(i);
        workspaces.push({
            index: i,
            name: ws?.name || `Workspace ${i + 1}`,
            active: i === wm.get_active_workspace_index(),
        });
    }
    return JSON.stringify(workspaces);
}

function findWindow(windowId) {
    const needle = String(windowId || '');
    if (needle.trim() === '') return null;

    const needleLower = needle.toLowerCase();
    const windows = global.get_window_actors().map(w => w.meta_window);

    return windows.find(w => String(w.get_stable_sequence?.() ?? 0) === needle)
        || windows.find(w => (w.get_wm_class() || '') === needle)
        || windows.find(w => (w.get_title() || '') === needle)
        || windows.find(w => {
            const wmClass = (w.get_wm_class() || '').toLowerCase();
            const title = (w.get_title() || '').toLowerCase();
            return wmClass.includes(needleLower) || title.includes(needleLower);
        });
}

function emitWindowStateChanged() {
    if (_debounceTimer) {
        GLib.source_remove(_debounceTimer);
        _debounceTimer = 0;
    }
    _debounceTimer = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 150, () => {
        _debounceTimer = 0;
        try {
            const info = serializeWindow(global.display.focus_window);
            if (info !== 'null' && _dbusImpl) {
                _dbusImpl.emit_signal('WindowStateChanged',
                    GLib.Variant.new('(s)', [info]));
            }
        } catch (e) {
            log('[deskbrid] signal error: ' + e);
        }
        return GLib.SOURCE_REMOVE;
    });
}

export default class Extension {
    enable() {
        _dbusImpl = Gio.DBusExportedObject.wrapJSObject(DeskbridIface, {
            ListWindows() {
                return serializeWindows(global.get_window_actors());
            },
            FocusedWindow() {
                return serializeWindow(global.display.focus_window);
            },
            FocusWindow(app_id, title, exact) {
                const windows = global.get_window_actors().map(w => w.meta_window);
                const matches = (needle, value) =>
                    exact ? value === needle : value.toLowerCase().includes(needle.toLowerCase());

                const found = windows.find(w => {
                    const wmClass = w.get_wm_class() || '';
                    const windowTitle = w.get_title() || '';
                    if (app_id && matches(app_id, wmClass)) return true;
                    if (title && matches(title, windowTitle)) return true;
                    return false;
                });

                if (!found) return false;
                found.activate(global.get_current_time());
                return true;
            },
            CloseWindow(window_id) {
                const found = findWindow(window_id);
                if (!found) return false;
                found.delete(global.get_current_time());
                emitWindowStateChanged();
                return true;
            },
            MinimizeWindow(window_id) {
                const found = findWindow(window_id);
                if (!found) return false;
                found.minimize();
                emitWindowStateChanged();
                return true;
            },
            MaximizeWindow(window_id) {
                const found = findWindow(window_id);
                if (!found) return false;
                const flags = Meta.MaximizeFlags.BOTH
                    ?? (Meta.MaximizeFlags.HORIZONTAL | Meta.MaximizeFlags.VERTICAL);
                found.maximize(flags);
                emitWindowStateChanged();
                return true;
            },
            MoveResizeWindow(window_id, x, y, width, height) {
                const found = findWindow(window_id);
                if (!found) return false;
                found.move_resize_frame(true, x, y, width, height);
                emitWindowStateChanged();
                return true;
            },
            ListWorkspaces() {
                return serializeWorkspaces();
            },
            SwitchWorkspace(index) {
                const wm = global.workspace_manager;
                if (index < 0 || index >= wm.n_workspaces) return;
                const ws = wm.get_workspace_by_index(index);
                if (ws) ws.activate(global.get_current_time());
            },
            MoveWindowToWorkspace(window_id, workspace_index) {
                const wm = global.workspace_manager;
                if (workspace_index < 0 || workspace_index >= wm.n_workspaces)
                    return false;
                const ws = wm.get_workspace_by_index(workspace_index);
                const found = findWindow(window_id);
                if (!found || !ws) return false;
                found.change_workspace(ws);
                emitWindowStateChanged();
                return true;
            }
        });

        _dbusId = Gio.DBus.session.own_name(
            DBUS_SERVICE,
            Gio.BusNameOwnerFlags.NONE,
            null, null, null
        );

        _dbusImpl.export(Gio.DBus.session, DBUS_PATH);

        // Watch for window focus changes
        _focusSignalId = global.display.connect('notify::focus-window', () => {
            emitWindowStateChanged();
        });

        // Watch window creation
        _windowCreatedId = global.display.connect('window-created', () => {
            emitWindowStateChanged();
        });

        log('[deskbrid] extension enabled — DBus service: ' + DBUS_SERVICE);
        _extensionInstance = this;  // GC root
    }

    disable() {
        _extensionInstance = null;
        if (_focusSignalId) {
            global.display.disconnect(_focusSignalId);
            _focusSignalId = 0;
        }
        if (_windowCreatedId) {
            global.display.disconnect(_windowCreatedId);
            _windowCreatedId = 0;
        }
        if (_dbusImpl) {
            _dbusImpl.unexport();
            _dbusImpl = null;
        }
        if (_dbusId) {
            Gio.DBus.session.unown_name(_dbusId);
            _dbusId = 0;
        }
        if (_debounceTimer) {
            GLib.source_remove(_debounceTimer);
            _debounceTimer = 0;
        }
        log('[deskbrid] extension disabled');
    }
}
