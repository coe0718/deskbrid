// reason: exhaustive match on 90+ Action enum variants — cannot split without breaking exhaustiveness
use serde_json::json;
use uuid::Uuid;

use super::Action;

pub fn to_json(action: &Action) -> anyhow::Result<String> {
    let _msg_type = action.action_type();
    let id = Uuid::new_v4().to_string();
    let envelope = match action {
        Action::Ping => json!({"type": "ping", "id": id}),

        // Windows
        Action::WindowsList => json!({"type": "windows.list", "id": id}),
        Action::WindowsFocus(window_id) => {
            json!({"type": "windows.focus", "id": id, "window_id": window_id})
        }
        Action::WindowsGet(window_id) => {
            json!({"type": "windows.get", "id": id, "window_id": window_id})
        }
        Action::WindowsClose(window_id) => {
            json!({"type":"windows.close","id":id,"window_id":window_id})
        }
        Action::WindowsMinimize(window_id) => {
            json!({"type":"windows.minimize","id":id,"window_id":window_id})
        }
        Action::WindowsMaximize(window_id) => {
            json!({"type":"windows.maximize","id":id,"window_id":window_id})
        }
        Action::WindowsMoveResize {
            window_id,
            x,
            y,
            width,
            height,
        } => {
            json!({"type":"windows.move_resize","id":id,"window_id":window_id,"x":x,"y":y,"width":width,"height":height})
        }
        Action::WindowsActivateOrLaunch {
            app_id,
            command,
            workdir,
            env,
        } => {
            let mut obj = json!({"type":"windows.activate_or_launch","id":id,"app_id":app_id});
            if !command.is_empty() {
                obj["command"] = json!(command);
            }
            if let Some(wd) = workdir {
                obj["workdir"] = json!(wd);
            }
            if let Some(e) = env {
                obj["env"] = json!(e);
            }
            obj
        }

        // Workspaces
        Action::WorkspacesList => json!({"type": "workspaces.list", "id": id}),
        Action::WorkspaceSwitch(workspace_id) => {
            json!({"type": "workspaces.switch", "id": id, "workspace_id": workspace_id})
        }
        Action::WorkspaceMoveWindow {
            window_id,
            workspace_id,
            follow,
        } => {
            json!({"type": "workspaces.move_window", "id": id, "window_id": window_id, "workspace_id": workspace_id, "follow": follow})
        }

        // Layout profiles
        Action::LayoutProfilesList => json!({"type": "layout_profiles.list", "id": id}),
        Action::LayoutProfileGet { name } => {
            json!({"type": "layout_profiles.get", "id": id, "name": name})
        }
        Action::LayoutProfileSave { name, overwrite } => {
            json!({"type": "layout_profiles.save", "id": id, "name": name, "overwrite": overwrite})
        }
        Action::LayoutProfileDelete { name } => {
            json!({"type": "layout_profiles.delete", "id": id, "name": name})
        }
        Action::LayoutProfileRestore { name } => {
            json!({"type": "layout_profiles.restore", "id": id, "name": name})
        }

        // Input
        Action::InputKeyboardType { text } => {
            json!({"type": "input.keyboard", "id": id, "action": "type", "text": text})
        }
        Action::InputKeyboardKey { key } => {
            json!({"type": "input.keyboard", "id": id, "action": "key", "key": key})
        }
        Action::InputKeyboardCombo { keys } => {
            json!({"type": "input.keyboard", "id": id, "action": "combo", "keys": keys})
        }
        Action::InputMouse {
            action,
            x,
            y,
            button,
            dx,
            dy,
        } => {
            let mut obj = json!({"type": "input.mouse", "id": id, "action": action});
            if let Some(x) = x {
                obj["x"] = json!(x);
            }
            if let Some(y) = y {
                obj["y"] = json!(y);
            }
            if let Some(button) = button {
                obj["button"] = json!(button);
            }
            if let Some(dx) = dx {
                obj["dx"] = json!(dx);
            }
            if let Some(dy) = dy {
                obj["dy"] = json!(dy);
            }
            obj
        }

        // Clipboard
        Action::ClipboardRead => json!({"type": "clipboard.read", "id": id}),
        Action::ClipboardWrite { text } => {
            json!({"type": "clipboard.write", "id": id, "text": text})
        }

        // Screenshot
        Action::Screenshot {
            monitor,
            region,
            window_id,
        } => {
            let mut obj = json!({"type": "screenshot", "id": id});
            if let Some(m) = monitor {
                obj["monitor"] = json!(m);
            }
            if let Some(r) = region {
                obj["region"] = json!(r);
            }
            if let Some(w) = window_id {
                obj["window_id"] = json!(w);
            }
            obj
        }
        Action::ScreenshotOcr {
            path,
            language,
            psm,
            bounding_boxes,
            monitor,
            region,
            window_id,
        } => {
            let mut obj =
                json!({"type": "screenshot.ocr", "id": id, "bounding_boxes": bounding_boxes});
            if let Some(path) = path {
                obj["path"] = json!(path);
            }
            if let Some(language) = language {
                obj["language"] = json!(language);
            }
            if let Some(psm) = psm {
                obj["psm"] = json!(psm);
            }
            if let Some(monitor) = monitor {
                obj["monitor"] = json!(monitor);
            }
            if let Some(region) = region {
                obj["region"] = json!(region);
            }
            if let Some(window_id) = window_id {
                obj["window_id"] = json!(window_id);
            }
            obj
        }

        // Notifications
        Action::NotificationSend {
            app_name,
            title,
            body,
            urgency,
        } => {
            json!({"type": "notification.send", "id": id, "app_name": app_name, "title": title, "body": body, "urgency": urgency})
        }
        Action::NotificationClose { notification_id } => {
            json!({"type": "notification.close", "id": id, "notification_id": notification_id})
        }

        // System
        Action::SystemInfo => json!({"type": "system.info", "id": id}),
        Action::SystemCapabilities => json!({"type": "system.capabilities", "id": id}),
        Action::SystemHealth => json!({"type": "system.health", "id": id}),
        Action::SystemRemediate { check, apply } => {
            json!({"type": "system.remediate", "id": id, "check": check, "apply": apply})
        }
        Action::SystemNormalizeCoords { x, y, monitor } => {
            let mut obj = json!({"type":"system.normalize_coords","id":id,"x":x,"y":y});
            if let Some(m) = monitor {
                obj["monitor"] = json!(m);
            }
            obj
        }
        Action::WaitFor {
            condition,
            params,
            timeout_ms,
            interval_ms,
        } => {
            let mut obj = json!({"type": "wait.for", "id": id, "condition": condition, "params": params, "timeout_ms": timeout_ms});
            if let Some(interval_ms) = interval_ms {
                obj["interval_ms"] = json!(interval_ms);
            }
            obj
        }
        Action::SystemIdle => json!({"type": "system.idle", "id": id}),
        Action::SystemPower { action } => {
            json!({"type": "system.power", "id": id, "action": action})
        }
        Action::SystemBattery => json!({"type": "system.battery", "id": id}),
        Action::SystemInhibit {
            what,
            who,
            why,
            mode,
        } => {
            let mut obj = json!({"type": "system.inhibit", "id": id, "what": what, "who": who});
            if let Some(why) = why {
                obj["why"] = json!(why);
            }
            if let Some(mode) = mode {
                obj["mode"] = json!(mode);
            }
            obj
        }
        Action::SystemReleaseInhibit { inhibitor_id } => {
            json!({"type": "system.release_inhibit", "id": id, "inhibitor_id": inhibitor_id})
        }
        Action::SystemListSessions => json!({"type": "system.sessions", "id": id}),
        Action::SystemLockSession { session_id } => {
            let mut obj = json!({"type": "system.lock_session", "id": id});
            if let Some(session_id) = session_id {
                obj["session_id"] = json!(session_id);
            }
            obj
        }
        Action::SystemSwitchUser { username } => {
            json!({"type": "system.switch_user", "id": id, "username": username})
        }
        Action::SystemCheckAuth { action_id } => {
            json!({"type": "system.check_auth", "id": id, "action_id": action_id})
        }
        Action::SystemElevate { action_id, reason } => {
            let mut obj = json!({"type": "system.elevate", "id": id, "action_id": action_id});
            if let Some(reason) = reason {
                obj["reason"] = json!(reason);
            }
            obj
        }
        Action::ServiceStatus { name } => {
            json!({"type": "service.status", "id": id, "name": name})
        }
        Action::ServiceStart { name } => {
            json!({"type": "service.start", "id": id, "name": name})
        }
        Action::ServiceStop { name } => {
            json!({"type": "service.stop", "id": id, "name": name})
        }
        Action::ServiceRestart { name } => {
            json!({"type": "service.restart", "id": id, "name": name})
        }
        Action::ServiceEnable { name, runtime } => {
            json!({"type": "service.enable", "id": id, "name": name, "runtime": runtime})
        }
        Action::ServiceDisable { name, runtime } => {
            json!({"type": "service.disable", "id": id, "name": name, "runtime": runtime})
        }
        Action::ServiceList { unit_type } => {
            let mut obj = json!({"type": "service.list", "id": id});
            if let Some(unit_type) = unit_type {
                obj["unit_type"] = json!(unit_type);
            }
            obj
        }
        Action::JournalQuery {
            since,
            until,
            unit,
            priority,
            tail,
        } => {
            let mut obj = json!({"type": "journal.query", "id": id});
            if let Some(since) = since {
                obj["since"] = json!(since);
            }
            if let Some(until) = until {
                obj["until"] = json!(until);
            }
            if let Some(unit) = unit {
                obj["unit"] = json!(unit);
            }
            if let Some(priority) = priority {
                obj["priority"] = json!(priority);
            }
            if let Some(tail) = tail {
                obj["tail"] = json!(tail);
            }
            obj
        }
        Action::TimerList => json!({"type": "timer.list", "id": id}),
        Action::TimerStart { name } => json!({"type": "timer.start", "id": id, "name": name}),
        Action::TimerStop { name } => json!({"type": "timer.stop", "id": id, "name": name}),

        // Network
        Action::NetworkStatus => json!({"type": "network.status", "id": id}),
        Action::NetworkInterfaces => json!({"type": "network.interfaces", "id": id}),
        Action::NetworkWifiScan => json!({"type": "network.wifi.scan", "id": id}),
        Action::NetworkWifiConnect { ssid, password } => {
            let mut obj = json!({"type": "network.wifi.connect", "id": id, "ssid": ssid});
            if let Some(pw) = password {
                obj["password"] = json!(pw);
            }
            obj
        }

        // Bluetooth
        Action::BluetoothList => json!({"type": "bluetooth.list", "id": id}),
        Action::BluetoothScan { duration } => {
            let mut obj = json!({"type": "bluetooth.scan", "id": id});
            if let Some(d) = duration {
                obj["duration"] = json!(d);
            }
            obj
        }
        Action::BluetoothStopScan => json!({"type": "bluetooth.scan_stop", "id": id}),
        Action::BluetoothConnect { address } => {
            json!({"type": "bluetooth.connect", "id": id, "address": address})
        }
        Action::BluetoothDisconnect { address } => {
            json!({"type": "bluetooth.disconnect", "id": id, "address": address})
        }
        Action::BluetoothPair { address } => {
            json!({"type": "bluetooth.pair", "id": id, "address": address})
        }
        Action::BluetoothForget { address } => {
            json!({"type": "bluetooth.forget", "id": id, "address": address})
        }

        // Files
        Action::FilesWatch {
            path,
            recursive,
            patterns,
        } => {
            let mut obj =
                json!({"type": "files.watch", "id": id, "path": path, "recursive": recursive});
            if let Some(p) = patterns {
                obj["patterns"] = json!(p);
            }
            obj
        }
        Action::FilesUnwatch { path } => {
            json!({"type": "files.unwatch", "id": id, "path": path})
        }
        Action::FilesSearch {
            pattern,
            root,
            max_results,
        } => {
            let mut obj = json!({"type": "files.search", "id": id, "pattern": pattern, "max_results": max_results});
            if let Some(r) = root {
                obj["root"] = json!(r);
            }
            obj
        }
        Action::FilesRead {
            path,
            offset,
            limit,
        } => {
            let mut obj = json!({"type": "files.read", "id": id, "path": path});
            if let Some(o) = offset {
                obj["offset"] = json!(o);
            }
            if let Some(l) = limit {
                obj["limit"] = json!(l);
            }
            obj
        }
        Action::FilesWrite {
            path,
            content,
            append,
        } => {
            json!({"type": "files.write", "id": id, "path": path, "content": content, "append": append})
        }
        Action::FilesCopy {
            source,
            destination,
        } => {
            json!({"type": "files.copy", "id": id, "source": source, "destination": destination})
        }
        Action::FilesMove {
            source,
            destination,
        } => {
            json!({"type": "files.move", "id": id, "source": source, "destination": destination})
        }
        Action::FilesDelete { path, recursive } => {
            json!({"type": "files.delete", "id": id, "path": path, "recursive": recursive})
        }
        Action::FilesMkdir { path, parents } => {
            json!({"type": "files.mkdir", "id": id, "path": path, "parents": parents})
        }
        Action::FilesList { path } => {
            json!({"type": "files.list", "id": id, "path": path})
        }
        Action::BrowserListTabs => json!({"type": "browser.list_tabs", "id": id}),
        Action::BrowserNavigate { tab_index, url } => {
            let mut obj = json!({"type": "browser.navigate", "id": id, "url": url});
            if let Some(idx) = tab_index {
                obj["tab_index"] = json!(idx);
            }
            obj
        }
        Action::BrowserEvaluate {
            tab_index,
            expression,
            await_promise,
        } => {
            let mut obj = json!({"type": "browser.evaluate", "id": id, "expression": expression, "await_promise": await_promise});
            if let Some(idx) = tab_index {
                obj["tab_index"] = json!(idx);
            }
            obj
        }
        Action::BrowserScreenshotTab { tab_index } => {
            let mut obj = json!({"type": "browser.screenshot_tab", "id": id});
            if let Some(idx) = tab_index {
                obj["tab_index"] = json!(idx);
            }
            obj
        }
        Action::BrowserClick {
            tab_index,
            selector,
        } => {
            let mut obj = json!({"type": "browser.click", "id": id, "selector": selector});
            if let Some(idx) = tab_index {
                obj["tab_index"] = json!(idx);
            }
            obj
        }

        // Accessibility
        Action::A11yTree { depth } => {
            let mut obj = json!({"type": "a11y.tree", "id": id});
            if let Some(d) = depth {
                obj["depth"] = json!(d);
            }
            obj
        }
        Action::A11yGetElement { role, name, index } => {
            let mut obj = json!({"type": "a11y.get_element", "id": id});
            if let Some(r) = role {
                obj["role"] = json!(r);
            }
            if let Some(n) = name {
                obj["name"] = json!(n);
            }
            if let Some(i) = index {
                obj["index"] = json!(i);
            }
            obj
        }
        Action::A11yClickElement { role, name, index } => {
            let mut obj = json!({"type": "a11y.click_element", "id": id});
            if let Some(r) = role {
                obj["role"] = json!(r);
            }
            if let Some(n) = name {
                obj["name"] = json!(n);
            }
            if let Some(i) = index {
                obj["index"] = json!(i);
            }
            obj
        }
        Action::A11yGetText { role, name, index } => {
            let mut obj = json!({"type": "a11y.get_text", "id": id});
            if let Some(r) = role {
                obj["role"] = json!(r);
            }
            if let Some(n) = name {
                obj["name"] = json!(n);
            }
            if let Some(i) = index {
                obj["index"] = json!(i);
            }
            obj
        }

        // Process
        Action::ProcessList => json!({"type": "process.list", "id": id}),
        Action::ProcessStart {
            command,
            workdir,
            env,
        } => {
            let mut obj = json!({"type": "process.start", "id": id, "command": command});
            if let Some(wd) = workdir {
                obj["workdir"] = json!(wd);
            }
            if let Some(e) = env {
                obj["env"] = json!(e);
            }
            obj
        }
        Action::ProcessStop { pid, signal } => {
            let mut obj = json!({"type": "process.stop", "id": id, "pid": pid});
            if let Some(sig) = signal {
                obj["signal"] = json!(sig);
            }
            obj
        }
        Action::ProcessSignal { pid, signal } => {
            json!({"type": "process.signal", "id": id, "pid": pid, "signal": signal})
        }
        Action::ProcessExists { pid } => {
            json!({"type": "process.exists", "id": id, "pid": pid})
        }
        Action::ProcessWait { pid, timeout_ms } => {
            let mut obj = json!({"type": "process.wait", "id": id, "pid": pid});
            if let Some(ms) = timeout_ms {
                obj["timeout_ms"] = json!(ms);
            }
            obj
        }
        Action::TerminalCreate {
            shell,
            cwd,
            env,
            rows,
            cols,
        } => {
            let mut obj = json!({"type": "terminal.create", "id": id});
            if let Some(shell) = shell {
                obj["shell"] = json!(shell);
            }
            if let Some(cwd) = cwd {
                obj["cwd"] = json!(cwd);
            }
            if let Some(env) = env {
                obj["env"] = json!(env);
            }
            if let Some(rows) = rows {
                obj["rows"] = json!(rows);
            }
            if let Some(cols) = cols {
                obj["cols"] = json!(cols);
            }
            obj
        }
        Action::TerminalWrite { terminal_id, input } => {
            json!({"type": "terminal.write", "id": id, "terminal_id": terminal_id, "input": input})
        }
        Action::TerminalRead {
            terminal_id,
            max_bytes,
            flush,
        } => {
            let mut obj = json!({"type": "terminal.read", "id": id, "terminal_id": terminal_id, "flush": flush});
            if let Some(max_bytes) = max_bytes {
                obj["max_bytes"] = json!(max_bytes);
            }
            obj
        }
        Action::TerminalResize {
            terminal_id,
            rows,
            cols,
        } => {
            json!({"type": "terminal.resize", "id": id, "terminal_id": terminal_id, "rows": rows, "cols": cols})
        }
        Action::TerminalList => json!({"type": "terminal.list", "id": id}),
        Action::TerminalKill {
            terminal_id,
            signal,
        } => {
            let mut obj = json!({"type": "terminal.kill", "id": id, "terminal_id": terminal_id});
            if let Some(signal) = signal {
                obj["signal"] = json!(signal);
            }
            obj
        }
        Action::CapabilitiesList => json!({"type": "capabilities.list", "id": id}),

        // Hotkeys
        Action::HotkeysRegister { hotkey_id, keys } => {
            json!({"type": "hotkeys.register", "id": id, "hotkey_id": hotkey_id, "keys": keys})
        }
        Action::HotkeysUnregister { hotkey_id } => {
            json!({"type": "hotkeys.unregister", "id": id, "hotkey_id": hotkey_id})
        }

        // Audio
        Action::AudioListSinks => json!({"type": "audio.list_sinks", "id": id}),
        Action::AudioSetSinkVolume { sink_id, volume } => {
            json!({"type": "audio.set_sink_volume", "id": id, "sink_id": sink_id, "volume": volume})
        }

        // Monitor
        Action::MonitorList => json!({"type": "monitor.list", "id": id}),
        Action::MonitorSetPrimary { output } => {
            json!({"type": "monitor.set_primary", "id": id, "output": output})
        }
        Action::MonitorSetResolution {
            output,
            width,
            height,
            refresh_rate,
        } => {
            let mut obj = json!({"type": "monitor.set_resolution", "id": id, "output": output, "width": width, "height": height});
            if let Some(refresh) = refresh_rate {
                obj["refresh_rate"] = json!(refresh);
            }
            obj
        }
        Action::MonitorSetScale { output, scale } => {
            json!({"type": "monitor.set_scale", "id": id, "output": output, "scale": scale})
        }
        Action::MonitorSetRotation { output, rotation } => {
            json!({"type": "monitor.set_rotation", "id": id, "output": output, "rotation": rotation})
        }
        Action::MonitorEnable { output } => {
            json!({"type": "monitor.enable", "id": id, "output": output})
        }
        Action::MonitorDisable { output } => {
            json!({"type": "monitor.disable", "id": id, "output": output})
        }

        // Location
        Action::LocationGet => json!({"type": "location.get", "id": id}),
        Action::UiTreeGet => json!({"type":"ui.tree.get","id":id}),
        Action::UiElementClick { selector } => {
            json!({"type":"ui.element.click","id":id,"selector":selector})
        }
        Action::UiElementSetText { selector, text } => {
            json!({"type":"ui.element.set_text","id":id,"selector":selector,"text":text})
        }

        // Connection
        Action::Subscribe { events } => {
            json!({"type": "subscribe", "id": id, "events": events})
        }
        Action::Unsubscribe { events } => {
            json!({"type": "unsubscribe", "id": id, "events": events})
        }
        Action::Disconnect => json!({"type": "disconnect", "id": id}),
    };

    Ok(serde_json::to_string(&envelope)?)
}

pub fn action_type(action: &Action) -> &'static str {
    match action {
        Action::Ping => "ping",
        Action::WindowsList => "windows.list",
        Action::WindowsFocus(_) => "windows.focus",
        Action::WindowsGet(_) => "windows.get",
        Action::WindowsClose(_) => "windows.close",
        Action::WindowsMinimize(_) => "windows.minimize",
        Action::WindowsMaximize(_) => "windows.maximize",
        Action::WindowsMoveResize { .. } => "windows.move_resize",
        Action::WindowsActivateOrLaunch { .. } => "windows.activate_or_launch",
        Action::WorkspacesList => "workspaces.list",
        Action::WorkspaceSwitch(_) => "workspaces.switch",
        Action::WorkspaceMoveWindow { .. } => "workspaces.move_window",
        Action::LayoutProfilesList => "layout_profiles.list",
        Action::LayoutProfileGet { .. } => "layout_profiles.get",
        Action::LayoutProfileSave { .. } => "layout_profiles.save",
        Action::LayoutProfileDelete { .. } => "layout_profiles.delete",
        Action::LayoutProfileRestore { .. } => "layout_profiles.restore",
        Action::InputKeyboardType { .. } => "input.keyboard",
        Action::InputKeyboardKey { .. } => "input.keyboard",
        Action::InputKeyboardCombo { .. } => "input.keyboard",
        Action::InputMouse { .. } => "input.mouse",
        Action::ClipboardRead => "clipboard.read",
        Action::ClipboardWrite { .. } => "clipboard.write",
        Action::Screenshot { .. } => "screenshot",
        Action::ScreenshotOcr { .. } => "screenshot.ocr",
        Action::NotificationSend { .. } => "notification.send",
        Action::NotificationClose { .. } => "notification.close",
        Action::SystemInfo => "system.info",
        Action::SystemCapabilities => "system.capabilities",
        Action::SystemHealth => "system.health",
        Action::SystemRemediate { .. } => "system.remediate",
        Action::SystemNormalizeCoords { .. } => "system.normalize_coords",
        Action::WaitFor { .. } => "wait.for",
        Action::SystemIdle => "system.idle",
        Action::SystemPower { .. } => "system.power",
        Action::SystemBattery => "system.battery",
        Action::SystemInhibit { .. } => "system.inhibit",
        Action::SystemReleaseInhibit { .. } => "system.release_inhibit",
        Action::SystemListSessions => "system.sessions",
        Action::SystemLockSession { .. } => "system.lock_session",
        Action::SystemSwitchUser { .. } => "system.switch_user",
        Action::SystemCheckAuth { .. } => "system.check_auth",
        Action::SystemElevate { .. } => "system.elevate",
        Action::ServiceStatus { .. } => "service.status",
        Action::ServiceStart { .. } => "service.start",
        Action::ServiceStop { .. } => "service.stop",
        Action::ServiceRestart { .. } => "service.restart",
        Action::ServiceEnable { .. } => "service.enable",
        Action::ServiceDisable { .. } => "service.disable",
        Action::ServiceList { .. } => "service.list",
        Action::JournalQuery { .. } => "journal.query",
        Action::TimerList => "timer.list",
        Action::TimerStart { .. } => "timer.start",
        Action::TimerStop { .. } => "timer.stop",
        Action::NetworkStatus => "network.status",
        Action::NetworkInterfaces => "network.interfaces",
        Action::NetworkWifiScan => "network.wifi.scan",
        Action::NetworkWifiConnect { .. } => "network.wifi.connect",
        Action::BluetoothList => "bluetooth.list",
        Action::BluetoothScan { .. } => "bluetooth.scan",
        Action::BluetoothStopScan => "bluetooth.scan_stop",
        Action::BluetoothConnect { .. } => "bluetooth.connect",
        Action::BluetoothDisconnect { .. } => "bluetooth.disconnect",
        Action::BluetoothPair { .. } => "bluetooth.pair",
        Action::BluetoothForget { .. } => "bluetooth.forget",
        Action::FilesWatch { .. } => "files.watch",
        Action::FilesUnwatch { .. } => "files.unwatch",
        Action::FilesSearch { .. } => "files.search",
        Action::FilesRead { .. } => "files.read",
        Action::FilesWrite { .. } => "files.write",
        Action::FilesCopy { .. } => "files.copy",
        Action::FilesMove { .. } => "files.move",
        Action::FilesDelete { .. } => "files.delete",
        Action::FilesMkdir { .. } => "files.mkdir",
        Action::FilesList { .. } => "files.list",
        Action::BrowserListTabs => "browser.list_tabs",
        Action::BrowserNavigate { .. } => "browser.navigate",
        Action::BrowserEvaluate { .. } => "browser.evaluate",
        Action::BrowserScreenshotTab { .. } => "browser.screenshot_tab",
        Action::BrowserClick { .. } => "browser.click",
        Action::A11yTree { .. } => "a11y.tree",
        Action::A11yGetElement { .. } => "a11y.get_element",
        Action::A11yClickElement { .. } => "a11y.click_element",
        Action::A11yGetText { .. } => "a11y.get_text",
        Action::ProcessList => "process.list",
        Action::ProcessStart { .. } => "process.start",
        Action::ProcessStop { .. } => "process.stop",
        Action::ProcessSignal { .. } => "process.signal",
        Action::ProcessExists { .. } => "process.exists",
        Action::ProcessWait { .. } => "process.wait",
        Action::TerminalCreate { .. } => "terminal.create",
        Action::TerminalWrite { .. } => "terminal.write",
        Action::TerminalRead { .. } => "terminal.read",
        Action::TerminalResize { .. } => "terminal.resize",
        Action::TerminalList => "terminal.list",
        Action::TerminalKill { .. } => "terminal.kill",
        Action::CapabilitiesList => "capabilities.list",
        Action::HotkeysRegister { .. } => "hotkeys.register",
        Action::HotkeysUnregister { .. } => "hotkeys.unregister",
        Action::AudioListSinks => "audio.list_sinks",
        Action::AudioSetSinkVolume { .. } => "audio.set_sink_volume",
        Action::MonitorList => "monitor.list",
        Action::MonitorSetPrimary { .. } => "monitor.set_primary",
        Action::MonitorSetResolution { .. } => "monitor.set_resolution",
        Action::MonitorSetScale { .. } => "monitor.set_scale",
        Action::MonitorSetRotation { .. } => "monitor.set_rotation",
        Action::MonitorEnable { .. } => "monitor.enable",
        Action::MonitorDisable { .. } => "monitor.disable",
        Action::LocationGet => "location.get",
        Action::UiTreeGet => "ui.tree.get",
        Action::UiElementClick { .. } => "ui.element.click",
        Action::UiElementSetText { .. } => "ui.element.set_text",
        Action::Subscribe { .. } => "subscribe",
        Action::Unsubscribe { .. } => "unsubscribe",
        Action::Disconnect => "disconnect",
    }
}
