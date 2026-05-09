use crate::protocol::Action;
use crate::{ConnectionState, DaemonState};
use anyhow::Context;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tracing::{debug, error, info, warn};

fn socket_path() -> String {
    std::env::var("XDG_RUNTIME_DIR")
        .map(|d| format!("{}/deskbrid.sock", d))
        .unwrap_or_else(|_| "/run/user/1000/deskbrid.sock".into())
}

pub async fn run() -> anyhow::Result<()> {
    let sock = socket_path();
    let _ = tokio::fs::remove_file(&sock).await;

    if let Some(parent) = std::path::Path::new(&sock).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let listener = UnixListener::bind(&sock).context("failed to bind Unix socket")?;

    info!("Deskbrid daemon listening on {}", sock);

    let state = Arc::new(DaemonState::new());

    // Load the GNOME backend
    let backend_tx = state.event_tx.clone();
    match crate::backend::create_backend(backend_tx).await {
        Ok(backend) => {
            let mut guard = state.backend.write().await;
            *guard = Some(backend);
            info!("GNOME backend loaded successfully");
        }
        Err(e) => {
            warn!(
                "Failed to load GNOME backend (running without desktop features): {}",
                e
            );
        }
    }

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                debug!("New connection from {:?}", addr);
                let state = Arc::clone(&state);
                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream, &state).await {
                        error!("Client error: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("Accept error: {}", e);
            }
        }
    }
}

async fn handle_client(stream: UnixStream, state: &DaemonState) -> anyhow::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut conn = ConnectionState::default();
    let mut line = String::new();
    let mut seq: u64 = 0;

    let connected = serde_json::json!({
        "type": "connected",
        "id": "server",
        "seq": 0,
        "data": { "version": "2.0.0", "protocol": "deskbrid-v2" }
    });
    writer
        .write_all(format!("{}\n", serde_json::to_string(&connected)?).as_bytes())
        .await?;

    // Spawn event forwarder: reads from broadcast and pushes matching events to this client
    let mut event_rx = state.event_tx.subscribe();
    let (event_tx, mut event_rx_forward) = tokio::sync::mpsc::unbounded_channel::<String>();

    tokio::spawn(async move {
        loop {
            match event_rx.recv().await {
                Ok(evt) => {
                    if let Ok(json) = serde_json::to_string(&evt) {
                        let _ = event_tx.send(json);
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    loop {
        tokio::select! {
            // Check for pending events to forward
            event_msg = event_rx_forward.recv() => {
                if let Some(msg) = event_msg {
                    // Parse the event to get its type for subscription matching
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&msg) {
                        let event_type = parsed["event"].as_str().unwrap_or("");
                        if event_matches_any(&conn.subscriptions, event_type) {
                            let envelope = serde_json::json!({
                                "type": "event",
                                "id": event_type,
                                "data": parsed
                            });
                            if let Ok(out) = serde_json::to_string(&envelope) {
                                let _ = writer.write_all(format!("{}\n", out).as_bytes()).await;
                            }
                        }
                    }
                }
            }

            // Read next client command
            result = reader.read_line(&mut line) => {
                let n = result?;
                if n == 0 {
                    break;
                }

                seq += 1;
                if line.trim().is_empty() {
                    line.clear();
                    continue;
                }

                let action = match Action::from_json(&line) {
                    Ok((_, action)) => action,
                    Err(e) => {
                        warn!("Failed to parse message: {}", e);
                        let err = serde_json::json!({
                            "type": "response", "id": "?", "seq": seq, "status": "error",
                            "error": { "code": "INVALID_PARAMS", "message": format!("{}", e) }
                        });
                        writer
                            .write_all(format!("{}\n", serde_json::to_string(&err)?).as_bytes())
                            .await?;
                        line.clear();
                        continue;
                    }
                };
                line.clear();

                match action {
                    Action::Disconnect => {
                        let resp = serde_json::json!({"type": "disconnected", "id": "dc", "seq": seq});
                        writer
                            .write_all(format!("{}\n", serde_json::to_string(&resp)?).as_bytes())
                            .await?;
                        break;
                    }

                    Action::Ping => {
                        let resp = serde_json::json!({"type": "pong", "id": "ping", "seq": seq});
                        writer
                            .write_all(format!("{}\n", serde_json::to_string(&resp)?).as_bytes())
                            .await?;
                    }

                    Action::Subscribe { events } => {
                        for evt in &events {
                            conn.subscriptions.insert(evt.clone());
                        }
                        let resp = ok_response("subscribe", seq);
                        writer
                            .write_all(format!("{}\n", serde_json::to_string(&resp)?).as_bytes())
                            .await?;
                    }

                    Action::Unsubscribe { events } => {
                        for evt in &events {
                            conn.subscriptions.remove(evt);
                        }
                        let resp = ok_response("unsubscribe", seq);
                        writer
                            .write_all(format!("{}\n", serde_json::to_string(&resp)?).as_bytes())
                            .await?;
                    }

                    // Files — track watched paths locally
                    Action::FilesWatch { ref path, .. } => {
                        conn.watched_paths.insert(path.clone());
                        let resp = dispatch_action(action, state, seq).await;
                        writer
                            .write_all(format!("{}\n", serde_json::to_string(&resp)?).as_bytes())
                            .await?;
                    }
                    Action::FilesUnwatch { ref path } => {
                        conn.watched_paths.remove(path);
                        let resp = dispatch_action(action, state, seq).await;
                        writer
                            .write_all(format!("{}\n", serde_json::to_string(&resp)?).as_bytes())
                            .await?;
                    }

                    _ => {
                        let resp = dispatch_action(action, state, seq).await;
                        writer
                            .write_all(format!("{}\n", serde_json::to_string(&resp)?).as_bytes())
                            .await?;
                    }
                }
            }
        }
    }

    info!("Client disconnected");
    Ok(())
}

/// Check if an event type matches any subscription glob pattern.
/// Simple prefix/wildcard matching: "file.*" matches "file.created", "file.*" matches "file.*", etc.
fn event_matches_any(subscriptions: &std::collections::HashSet<String>, event_type: &str) -> bool {
    for sub in subscriptions {
        if sub == event_type {
            return true;
        }
        // Glob-style: "file.*" matches "file.created"
        if let Some(prefix) = sub.strip_suffix(".*") {
            if event_type.starts_with(prefix) && event_type[prefix.len()..].starts_with('.') {
                return true;
            }
        }
        // Glob-style: "*" matches everything
        if sub == "*" {
            return true;
        }
    }
    false
}

async fn dispatch_action(action: Action, state: &DaemonState, seq: u64) -> serde_json::Value {
    let backend = state.backend.read().await;
    let backend = match backend.as_ref() {
        Some(b) => b,
        None => {
            return not_supported_response(
                "no backend loaded (start daemon in a GNOME 46+ session)",
                seq,
            );
        }
    };

    let result = execute_action(action, backend.as_ref()).await;

    match result {
        Ok(data) => serde_json::json!({
            "type": "response", "id": "action", "seq": seq, "status": "ok", "data": data
        }),
        Err(e) => {
            warn!("Action failed: {}", e);
            serde_json::json!({
                "type": "response", "id": "action", "seq": seq, "status": "error",
                "error": { "code": "INTERNAL_ERROR", "message": format!("{}", e) }
            })
        }
    }
}

async fn execute_action(
    action: Action,
    backend: &dyn crate::backend::DesktopBackend,
) -> anyhow::Result<serde_json::Value> {
    use Action::*;

    Ok(match action {
        WindowsList => serde_json::json!(backend.windows_list().await?),
        WindowsFocus(ref id) => {
            backend.window_focus(id).await?;
            serde_json::json!({"focused": id})
        }
        WindowsGet(ref id) => serde_json::json!(backend.window_get(id).await?),

        WorkspacesList => serde_json::json!(backend.workspaces_list().await?),
        WorkspaceSwitch(id) => {
            backend.workspace_switch(id).await?;
            serde_json::json!({"workspace": id})
        }
        WorkspaceMoveWindow {
            ref window_id,
            workspace_id,
            follow,
        } => {
            backend
                .workspace_move_window(window_id, workspace_id, follow)
                .await?;
            serde_json::json!({"moved": true})
        }

        InputKeyboardType { ref text } => {
            backend.keyboard_type(text).await?;
            serde_json::json!({"typed": text.len()})
        }
        InputKeyboardKey { ref key } => {
            backend.keyboard_key(key).await?;
            serde_json::json!({"key": key})
        }
        InputKeyboardCombo { ref keys } => {
            backend.keyboard_combo(keys).await?;
            serde_json::json!({"combo": keys})
        }
        InputMouse {
            ref action,
            x,
            y,
            ref button,
            dx,
            dy,
        } => {
            match action.as_str() {
                "move" => {
                    backend
                        .mouse_move(x.unwrap_or(0.0), y.unwrap_or(0.0))
                        .await?
                }
                "click" => {
                    backend
                        .mouse_click(button.as_deref().unwrap_or("left"))
                        .await?
                }
                "scroll" => {
                    backend
                        .mouse_scroll(dx.unwrap_or(0.0), dy.unwrap_or(0.0))
                        .await?
                }
                _ => anyhow::bail!("unknown mouse action: {}", action),
            }
            serde_json::json!({"mouse": action})
        }

        ClipboardRead => serde_json::json!({"text": backend.clipboard_read().await?}),
        ClipboardWrite { ref text } => {
            backend.clipboard_write(text).await?;
            serde_json::json!({"written": true})
        }

        Screenshot {
            monitor,
            ref region,
            ref window_id,
        } => {
            serde_json::json!(
                backend
                    .screenshot(monitor, region.clone(), window_id.clone())
                    .await?
            )
        }

        NotificationSend {
            ref app_name,
            ref title,
            ref body,
            ref urgency,
        } => {
            let id = backend
                .notification_send(app_name, title, body, urgency)
                .await?;
            serde_json::json!({"notification_id": id})
        }
        NotificationClose { notification_id } => {
            backend.notification_close(notification_id).await?;
            serde_json::json!({"closed": notification_id})
        }

        SystemInfo => serde_json::json!(backend.system_info().await?),
        SystemIdle => serde_json::json!({"idle_seconds": backend.idle_seconds().await?}),
        SystemPower { ref action } => {
            backend.power_action(action).await?;
            serde_json::json!({"power": action})
        }
        SystemBattery => serde_json::json!(backend.battery_status().await?),

        NetworkStatus => serde_json::json!(backend.network_status().await?),
        NetworkInterfaces => serde_json::json!(backend.network_interfaces().await?),
        NetworkWifiScan => serde_json::json!(backend.wifi_scan().await?),
        NetworkWifiConnect {
            ref ssid,
            ref password,
        } => {
            backend.wifi_connect(ssid, password.as_deref()).await?;
            serde_json::json!({"connected": ssid})
        }

        BluetoothList => serde_json::json!(backend.bluetooth_list().await?),
        BluetoothScan { duration } => {
            backend.bluetooth_scan(duration).await?;
            serde_json::json!({"scanning": true})
        }
        BluetoothStopScan => {
            backend.bluetooth_stop_scan().await?;
            serde_json::json!({"scanning": false})
        }
        BluetoothConnect { ref address } => {
            backend.bluetooth_connect(address).await?;
            serde_json::json!({"connected": address})
        }
        BluetoothDisconnect { ref address } => {
            backend.bluetooth_disconnect(address).await?;
            serde_json::json!({"disconnected": address})
        }

        // BT pair/forget not in trait yet — stub
        BluetoothPair { ref address } => {
            serde_json::json!({"paired": address, "note": "not yet supported"})
        }
        BluetoothForget { ref address } => {
            serde_json::json!({"forgotten": address, "note": "not yet supported"})
        }

        FilesWatch {
            ref path,
            recursive,
            ref patterns,
        } => {
            backend
                .files_watch(path, recursive, patterns.as_deref())
                .await?;
            serde_json::json!({"watching": path})
        }
        FilesUnwatch { ref path } => {
            backend.files_unwatch(path).await?;
            serde_json::json!({"unwatched": path})
        }
        FilesSearch {
            ref pattern,
            ref root,
            max_results,
        } => {
            serde_json::json!({"matches": backend.files_search(pattern, root.as_deref(), max_results).await?})
        }

        ProcessList => {
            let output = tokio::process::Command::new("ps")
                .args(["aux", "--no-headers"])
                .output()
                .await?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            let processes: Vec<serde_json::Value> = stdout
                .lines()
                .take(200)
                .filter_map(|line| {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() < 11 {
                        return None;
                    }
                    Some(serde_json::json!({
                        "user": parts[0],
                        "pid": parts[1].parse::<u32>().unwrap_or(0),
                        "cpu": parts[2],
                        "mem": parts[3],
                        "command": parts[10..].join(" ")
                    }))
                })
                .collect();
            serde_json::json!({"processes": processes})
        }
        ProcessStart {
            ref command,
            ref workdir,
            ref env,
        } => {
            let mut cmd = std::process::Command::new(&command[0]);
            cmd.args(&command[1..]);
            if let Some(wd) = workdir {
                cmd.current_dir(wd);
            }
            if let Some(env_vars) = env {
                for (k, v) in env_vars {
                    cmd.env(k, v);
                }
            }
            let child = cmd
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()?;
            serde_json::json!({"pid": child.id(), "command": command})
        }

        HotkeysRegister {
            ref hotkey_id,
            ref keys,
        } => serde_json::json!({"registered": hotkey_id, "keys": keys}),
        HotkeysUnregister { ref hotkey_id } => serde_json::json!({"unregistered": hotkey_id}),

        AudioListSinks => serde_json::json!(backend.audio_list_sinks().await?),
        AudioSetSinkVolume { sink_id, volume } => {
            backend.audio_set_sink_volume(sink_id, volume).await?;
            serde_json::json!({"sink": sink_id, "volume": volume})
        }

        MonitorList => serde_json::json!(backend.system_info().await?.monitors),
        LocationGet => serde_json::json!({"location": "not yet implemented"}),

        // Handled before dispatch
        Ping | Subscribe { .. } | Unsubscribe { .. } | Disconnect => unreachable!(),
    })
}

fn ok_response(id: &str, seq: u64) -> serde_json::Value {
    serde_json::json!({"type": "response", "id": id, "seq": seq, "status": "ok", "data": {}})
}

fn not_supported_response(msg: &str, seq: u64) -> serde_json::Value {
    serde_json::json!({
        "type": "response", "id": "?", "seq": seq, "status": "error",
        "error": { "code": "NOT_SUPPORTED", "message": msg }
    })
}
