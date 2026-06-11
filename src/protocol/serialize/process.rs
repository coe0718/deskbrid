use super::Action;
use serde_json::json;

pub(super) fn serialize_process(action: &Action, id: &str) -> serde_json::Value {
    match action {
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
        _ => serde_json::json!({"error": "not a process action"}),
    }
}
