use crate::permissions::socket_peer_uid;
use crate::protocol::{Action, LayoutProfile, LayoutProfileSummary};
use crate::{ConnectionState, DaemonState};
use anyhow::Context;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
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
    let peer_uid = socket_peer_uid(&stream).unwrap_or(u32::MAX);
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut conn = ConnectionState::default();
    let mut line = String::new();
    let mut seq: u64 = 0;

    let connected = serde_json::json!({
        "type": "connected",
        "id": "server",
        "seq": 0,
        "data": { "version": "0.6.0", "protocol": "deskbrid-v2", "uid": peer_uid }
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
                        let resp = dispatch_action(action, state, peer_uid, seq).await;
                        writer
                            .write_all(format!("{}\n", serde_json::to_string(&resp)?).as_bytes())
                            .await?;
                    }
                    Action::FilesUnwatch { ref path } => {
                        conn.watched_paths.remove(path);
                        let resp = dispatch_action(action, state, peer_uid, seq).await;
                        writer
                            .write_all(format!("{}\n", serde_json::to_string(&resp)?).as_bytes())
                            .await?;
                    }

                    _ => {
                        let resp = dispatch_action(action, state, peer_uid, seq).await;
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
        if let Some(prefix) = sub.strip_suffix(".*")
            && event_type.starts_with(prefix)
            && event_type[prefix.len()..].starts_with('.')
        {
            return true;
        }
        // Glob-style: "*" matches everything
        if sub == "*" {
            return true;
        }
    }
    false
}

async fn dispatch_action(
    action: Action,
    state: &DaemonState,
    peer_uid: u32,
    seq: u64,
) -> serde_json::Value {
    // Check permissions first
    if !state.permissions.check(peer_uid, &action) {
        return permission_denied_response(seq);
    }
    for implied_action in implied_permission_actions(&action) {
        if !state.permissions.check(peer_uid, &implied_action) {
            return permission_denied_response(seq);
        }
    }
    if let Action::WindowsActivateOrLaunch {
        command,
        workdir,
        env,
        ..
    } = &action
    {
        let process_start = Action::ProcessStart {
            command: command.clone(),
            workdir: workdir.clone(),
            env: env.clone(),
        };
        if !state.permissions.check(peer_uid, &process_start) {
            return permission_denied_response(seq);
        }
    }

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

    let result = execute_action(action.clone(), backend.as_ref()).await;

    match result {
        Ok(data) => {
            emit_action_event(state, &action, &data);
            serde_json::json!({
                "type": "response", "id": "action", "seq": seq, "status": "ok", "data": data
            })
        }
        Err(e) => {
            warn!("Action failed: {}", e);
            serde_json::json!({
                "type": "response", "id": "action", "seq": seq, "status": "error",
                "error": { "code": "INTERNAL_ERROR", "message": format!("{}", e) }
            })
        }
    }
}

fn implied_permission_actions(action: &Action) -> Vec<Action> {
    match action {
        Action::LayoutProfileSave { .. } => {
            vec![
                Action::WindowsList,
                Action::WorkspacesList,
                Action::SystemInfo,
            ]
        }
        Action::LayoutProfileRestore { .. } => vec![
            Action::WindowsMoveResize {
                window_id: "profile".into(),
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            },
            Action::WindowsMinimize("profile".into()),
            Action::WorkspaceSwitch(0),
            Action::WorkspaceMoveWindow {
                window_id: "profile".into(),
                workspace_id: 0,
                follow: false,
            },
        ],
        _ => Vec::new(),
    }
}

fn emit_action_event(state: &DaemonState, action: &Action, data: &serde_json::Value) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let event = match action {
        // Use the resolved window ID from the response data when available,
        // so subscribers get the canonical ID, not the caller-provided selector.
        Action::WindowsFocus(_) => {
            let window_id = data
                .get("focused")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            Some(crate::protocol::DeskbridEvent::WindowFocused {
                window_id,
                timestamp: now,
            })
        }
        Action::WorkspaceSwitch(id) => Some(crate::protocol::DeskbridEvent::WorkspaceChanged {
            workspace_id: *id,
            timestamp: now,
        }),
        Action::WorkspaceMoveWindow {
            window_id,
            workspace_id,
            ..
        } => Some(crate::protocol::DeskbridEvent::WorkspaceWindowMoved {
            window_id: window_id.clone(),
            workspace_id: *workspace_id,
            timestamp: now,
        }),
        _ => None,
    };
    if let Some(evt) = event {
        let _ = state.event_tx.send(evt);
    }
    let _ = data;
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
            // Try to get the resolved window so events publish the canonical ID,
            // not the caller-provided selector. Falls back to the raw selector.
            let resolved = backend
                .window_get(id)
                .await
                .map(|w| w.id)
                .unwrap_or_else(|_| id.clone());
            serde_json::json!({"focused": resolved, "id": id})
        }
        WindowsGet(ref id) => serde_json::json!(backend.window_get(id).await?),
        WindowsClose(ref id) => {
            backend.window_close(id).await?;
            serde_json::json!({"closed": id})
        }
        WindowsMinimize(ref id) => {
            backend.window_minimize(id).await?;
            serde_json::json!({"minimized": id})
        }
        WindowsMaximize(ref id) => {
            backend.window_maximize(id).await?;
            serde_json::json!({"maximized": id})
        }
        WindowsMoveResize {
            ref window_id,
            x,
            y,
            width,
            height,
        } => {
            backend
                .window_move_resize(window_id, x, y, width, height)
                .await?;
            serde_json::json!({
                "window_id": window_id, "x": x, "y": y, "width": width, "height": height
            })
        }
        WindowsActivateOrLaunch {
            ref app_id,
            ref command,
            ref workdir,
            ref env,
        } => {
            if let Some(window) = find_app_window(backend, app_id).await? {
                backend.window_focus(&window.id).await?;
                serde_json::json!({
                    "app_id": app_id,
                    "activated": true,
                    "launched": false,
                    "window_id": window.id
                })
            } else {
                let launch_command = if command.is_empty() {
                    vec![app_id.clone()]
                } else {
                    command.clone()
                };
                let pid = spawn_detached_process(&launch_command, workdir.as_deref(), env.as_ref())
                    .await?;
                serde_json::json!({
                    "app_id": app_id,
                    "activated": false,
                    "launched": true,
                    "pid": pid,
                    "command": launch_command
                })
            }
        }

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
        LayoutProfilesList => serde_json::json!(list_layout_profiles().await?),
        LayoutProfileGet { ref name } => serde_json::json!(load_layout_profile(name).await?),
        LayoutProfileSave {
            ref name,
            overwrite,
        } => {
            let profile = capture_layout_profile(name, backend).await?;
            let path = save_layout_profile(&profile, overwrite).await?;
            serde_json::json!({
                "profile": profile,
                "path": path.to_string_lossy()
            })
        }
        LayoutProfileDelete { ref name } => {
            let path = layout_profile_path(name)?;
            tokio::fs::remove_file(&path)
                .await
                .with_context(|| format!("failed to delete layout profile '{}'", name))?;
            serde_json::json!({"deleted": name})
        }
        LayoutProfileRestore { ref name } => {
            let profile = load_layout_profile(name).await?;
            serde_json::json!(restore_layout_profile(&profile, backend).await?)
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
        SystemCapabilities => serde_json::json!(build_system_capabilities(backend).await?),
        SystemHealth => serde_json::json!(build_system_health(backend).await?),

        SystemIdle => serde_json::json!({"idle_seconds": backend.idle_seconds().await?}),
        SystemRemediate { ref check, apply } => {
            serde_json::json!(run_system_remediation(check, apply).await?)
        }
        SystemNormalizeCoords { x, y, monitor } => {
            let info = backend.system_info().await?;
            serde_json::json!(normalize_coords(&info, x, y, monitor))
        }
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
            let pid = spawn_detached_process(command, workdir.as_deref(), env.as_ref()).await?;
            serde_json::json!({"pid": pid, "command": command})
        }
        ProcessStop { pid, ref signal } => {
            ensure_safe_pid(pid)?;
            let sig = parse_signal(signal.as_deref().unwrap_or("TERM"))?;
            let rc = unsafe { libc::kill(pid as i32, sig) };
            if rc != 0 {
                let err = std::io::Error::last_os_error();
                anyhow::bail!("failed to stop pid {}: {}", pid, err);
            }
            serde_json::json!({"stopped": pid, "signal": sig})
        }
        ProcessSignal { pid, ref signal } => {
            ensure_safe_pid(pid)?;
            let sig = parse_signal(signal)?;
            let rc = unsafe { libc::kill(pid as i32, sig) };
            if rc != 0 {
                let err = std::io::Error::last_os_error();
                anyhow::bail!("failed to signal pid {}: {}", pid, err);
            }
            serde_json::json!({"signaled": pid, "signal": sig})
        }
        ProcessExists { pid } => {
            ensure_safe_pid(pid)?;
            let rc = unsafe { libc::kill(pid as i32, 0) };
            if rc == 0 {
                serde_json::json!({"pid": pid, "exists": true})
            } else {
                let errno = std::io::Error::last_os_error()
                    .raw_os_error()
                    .unwrap_or_default();
                if errno == libc::ESRCH {
                    serde_json::json!({"pid": pid, "exists": false})
                } else {
                    anyhow::bail!(
                        "failed to check pid {}: {}",
                        pid,
                        std::io::Error::last_os_error()
                    )
                }
            }
        }
        ProcessWait { pid, timeout_ms } => {
            ensure_safe_pid(pid)?;
            let timeout = std::time::Duration::from_millis(timeout_ms.unwrap_or(30_000));
            let started = std::time::Instant::now();
            loop {
                let rc = unsafe { libc::kill(pid as i32, 0) };
                if rc != 0 {
                    let errno = std::io::Error::last_os_error()
                        .raw_os_error()
                        .unwrap_or_default();
                    if errno == libc::ESRCH {
                        break;
                    }
                    anyhow::bail!(
                        "failed to wait on pid {}: {}",
                        pid,
                        std::io::Error::last_os_error()
                    );
                }
                if started.elapsed() >= timeout {
                    return Ok(
                        serde_json::json!({"pid": pid, "exited": false, "timeout_ms": timeout.as_millis()}),
                    );
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            serde_json::json!({"pid": pid, "exited": true, "elapsed_ms": started.elapsed().as_millis()})
        }
        CapabilitiesList => {
            let actions = crate::protocol::Action::public_action_types();
            let desktop = backend.system_info().await?.desktop;
            let desktop_l = desktop.to_lowercase();
            let mut unsupported = vec![
                serde_json::json!({"action":"ui.tree.get","reason":"AT-SPI not integrated yet"}),
                serde_json::json!({"action":"ui.element.click","reason":"AT-SPI not integrated yet"}),
                serde_json::json!({"action":"ui.element.set_text","reason":"AT-SPI not integrated yet"}),
            ];
            if desktop_l.contains("hyprland") {
                unsupported.push(serde_json::json!({
                    "action":"windows.minimize",
                    "reason":"Hyprland does not expose a native minimize dispatcher"
                }));
            }
            // Keep `supported` and `unsupported` mutually exclusive for clients.
            let unsupported_actions: std::collections::HashSet<&str> = unsupported
                .iter()
                .filter_map(|entry| entry.get("action").and_then(|value| value.as_str()))
                .collect();
            let supported: Vec<&'static str> = actions
                .iter()
                .copied()
                .filter(|name| !unsupported_actions.contains(name))
                .collect();

            serde_json::json!({
                "desktop": desktop,
                "actions": actions,
                "supported": supported,
                "unsupported": unsupported
            })
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
        UiTreeGet => {
            serde_json::json!({"supported": false, "reason":"AT-SPI not integrated yet", "nodes":[]})
        }
        UiElementClick { ref selector } => {
            serde_json::json!({"supported": false, "reason":"AT-SPI not integrated yet", "selector": selector})
        }
        UiElementSetText {
            ref selector,
            ref text,
        } => {
            serde_json::json!({"supported": false, "reason":"AT-SPI not integrated yet", "selector": selector, "text": text})
        }

        // Handled before dispatch
        Ping | Subscribe { .. } | Unsubscribe { .. } | Disconnect => unreachable!(),
    })
}

async fn capture_layout_profile(
    name: &str,
    backend: &dyn crate::backend::DesktopBackend,
) -> anyhow::Result<LayoutProfile> {
    let name = validate_layout_profile_name(name)?.to_string();
    let info = backend.system_info().await?;
    Ok(LayoutProfile {
        schema_version: 1,
        name,
        saved_at: unix_timestamp(),
        desktop: info.desktop,
        session_type: info.session_type,
        current_workspace: info.current_workspace,
        monitors: info.monitors,
        workspaces: backend.workspaces_list().await?,
        windows: backend.windows_list().await?,
    })
}

async fn save_layout_profile(profile: &LayoutProfile, overwrite: bool) -> anyhow::Result<PathBuf> {
    let path = layout_profile_path(&profile.name)?;
    if !overwrite && tokio::fs::metadata(&path).await.is_ok() {
        anyhow::bail!("layout profile '{}' already exists", profile.name);
    }

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let data = serde_json::to_vec_pretty(profile)?;
    tokio::fs::write(&path, data).await?;
    Ok(path)
}

async fn load_layout_profile(name: &str) -> anyhow::Result<LayoutProfile> {
    let path = layout_profile_path(name)?;
    let data = tokio::fs::read(&path)
        .await
        .with_context(|| format!("failed to read layout profile '{}'", name))?;
    serde_json::from_slice(&data)
        .with_context(|| format!("failed to parse layout profile '{}'", name))
}

async fn list_layout_profiles() -> anyhow::Result<Vec<LayoutProfileSummary>> {
    let dir = layout_profiles_dir();
    let mut reader = match tokio::fs::read_dir(&dir).await {
        Ok(reader) => reader,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };

    let mut profiles = Vec::new();
    while let Some(entry) = reader.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let Ok(data) = tokio::fs::read(&path).await else {
            continue;
        };
        let Ok(profile) = serde_json::from_slice::<LayoutProfile>(&data) else {
            continue;
        };
        profiles.push(layout_profile_summary(&profile));
    }
    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(profiles)
}

async fn restore_layout_profile(
    profile: &LayoutProfile,
    backend: &dyn crate::backend::DesktopBackend,
) -> anyhow::Result<serde_json::Value> {
    let current_info = backend.system_info().await?;
    let current_windows = backend.windows_list().await?;
    let mut restored = Vec::new();
    let mut missing = Vec::new();
    let mut errors = Vec::new();

    for saved in &profile.windows {
        let Some(target) = match_profile_window(saved, &current_windows) else {
            missing.push(serde_json::json!({
                "id": saved.id,
                "app_id": saved.app_id,
                "title": saved.title
            }));
            continue;
        };

        let mut window_errors = Vec::new();
        if target.workspace_id != saved.workspace_id
            && let Err(e) = backend
                .workspace_move_window(&target.id, saved.workspace_id, false)
                .await
        {
            window_errors.push(format!("workspace: {}", e));
        }
        if let Some(ref geometry) = saved.geometry
            && geometry.width > 0
            && geometry.height > 0
            && let Err(e) = backend
                .window_move_resize(
                    &target.id,
                    geometry.x,
                    geometry.y,
                    geometry.width,
                    geometry.height,
                )
                .await
        {
            window_errors.push(format!("geometry: {}", e));
        }
        if saved.is_minimized
            && let Err(e) = backend.window_minimize(&target.id).await
        {
            window_errors.push(format!("minimize: {}", e));
        }

        if window_errors.is_empty() {
            restored.push(serde_json::json!({
                "profile_window_id": saved.id,
                "window_id": target.id,
                "app_id": saved.app_id,
                "title": saved.title,
                "workspace_id": saved.workspace_id
            }));
        } else {
            errors.push(serde_json::json!({
                "profile_window_id": saved.id,
                "window_id": target.id,
                "app_id": saved.app_id,
                "title": saved.title,
                "errors": window_errors
            }));
        }
    }

    let workspace_switched = match backend.workspace_switch(profile.current_workspace).await {
        Ok(()) => true,
        Err(e) => {
            errors.push(serde_json::json!({
                "workspace_id": profile.current_workspace,
                "errors": [format!("switch: {}", e)]
            }));
            false
        }
    };

    Ok(serde_json::json!({
        "profile": profile.name,
        "restored": restored,
        "missing": missing,
        "errors": errors,
        "workspace_switched": workspace_switched,
        "current_workspace": profile.current_workspace,
        "monitor_topology_matches": monitors_match(&profile.monitors, &current_info.monitors),
        "saved_monitor_count": profile.monitors.len(),
        "current_monitor_count": current_info.monitors.len()
    }))
}

fn layout_profile_summary(profile: &LayoutProfile) -> LayoutProfileSummary {
    LayoutProfileSummary {
        name: profile.name.clone(),
        saved_at: profile.saved_at,
        desktop: profile.desktop.clone(),
        session_type: profile.session_type.clone(),
        current_workspace: profile.current_workspace,
        monitor_count: profile.monitors.len(),
        workspace_count: profile.workspaces.len(),
        window_count: profile.windows.len(),
    }
}

fn match_profile_window(
    saved: &crate::protocol::WindowInfo,
    current: &[crate::protocol::WindowInfo],
) -> Option<crate::protocol::WindowInfo> {
    current
        .iter()
        .find(|w| w.id == saved.id)
        .cloned()
        .or_else(|| {
            current
                .iter()
                .find(|w| {
                    !saved.app_id.is_empty()
                        && !saved.title.is_empty()
                        && w.app_id == saved.app_id
                        && w.title == saved.title
                })
                .cloned()
        })
        .or_else(|| {
            current
                .iter()
                .find(|w| !saved.app_id.is_empty() && w.app_id == saved.app_id)
                .cloned()
        })
        .or_else(|| {
            current
                .iter()
                .find(|w| !saved.title.is_empty() && w.title == saved.title)
                .cloned()
        })
}

fn monitors_match(
    saved: &[crate::protocol::MonitorInfo],
    current: &[crate::protocol::MonitorInfo],
) -> bool {
    if saved.len() != current.len() {
        return false;
    }
    saved.iter().zip(current).all(|(a, b)| {
        a.name == b.name
            && a.width == b.width
            && a.height == b.height
            && (a.scale - b.scale).abs() < f64::EPSILON
            && a.primary == b.primary
    })
}

fn layout_profile_path(name: &str) -> anyhow::Result<PathBuf> {
    let name = validate_layout_profile_name(name)?;
    Ok(layout_profiles_dir().join(format!("{}.json", name)))
}

fn layout_profiles_dir() -> PathBuf {
    if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(config_home)
            .join("deskbrid")
            .join("layout_profiles");
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("deskbrid")
        .join("layout_profiles")
}

fn validate_layout_profile_name(name: &str) -> anyhow::Result<&str> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        anyhow::bail!("layout profile name must not be empty");
    }
    if name.len() != trimmed.len() {
        anyhow::bail!("layout profile name must not start or end with whitespace");
    }
    if name == "." || name == ".." {
        anyhow::bail!("invalid layout profile name: {}", name);
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        anyhow::bail!("layout profile name may only contain letters, numbers, '.', '-' and '_'");
    }
    Ok(name)
}

fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

async fn find_app_window(
    backend: &dyn crate::backend::DesktopBackend,
    app_id: &str,
) -> anyhow::Result<Option<crate::protocol::WindowInfo>> {
    if app_id.trim().is_empty() {
        anyhow::bail!("app_id must not be empty");
    }

    let windows = backend.windows_list().await?;
    let app_l = app_id.to_lowercase();
    Ok(windows
        .iter()
        .find(|w| w.app_id.eq_ignore_ascii_case(app_id))
        .cloned()
        .or_else(|| {
            windows
                .iter()
                .find(|w| w.title.eq_ignore_ascii_case(app_id))
                .cloned()
        })
        .or_else(|| {
            windows
                .iter()
                .find(|w| {
                    w.app_id.to_lowercase().contains(&app_l)
                        || w.title.to_lowercase().contains(&app_l)
                })
                .cloned()
        }))
}

async fn spawn_detached_process(
    command: &[String],
    workdir: Option<&str>,
    env: Option<&HashMap<String, String>>,
) -> anyhow::Result<u32> {
    let program = command
        .first()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("command must not be empty"))?;

    let mut cmd = tokio::process::Command::new(program);
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
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(child.id().unwrap_or(0))
}

fn ensure_safe_pid(pid: u32) -> anyhow::Result<()> {
    if pid <= 1 {
        anyhow::bail!("refusing to target reserved pid {}", pid);
    }
    if pid > i32::MAX as u32 {
        anyhow::bail!(
            "refusing to target out-of-range pid {} (exceeds i32::MAX)",
            pid
        );
    }
    let self_pid = std::process::id();
    if pid == self_pid {
        anyhow::bail!("refusing to target deskbrid daemon pid {}", pid);
    }
    Ok(())
}

fn parse_signal(sig: &str) -> anyhow::Result<i32> {
    let normalized = sig.trim().to_ascii_uppercase();
    let normalized = normalized.strip_prefix("SIG").unwrap_or(&normalized);
    let value = match normalized {
        "HUP" => libc::SIGHUP,
        "INT" => libc::SIGINT,
        "QUIT" => libc::SIGQUIT,
        "KILL" => libc::SIGKILL,
        "TERM" => libc::SIGTERM,
        "USR1" => libc::SIGUSR1,
        "USR2" => libc::SIGUSR2,
        "CONT" => libc::SIGCONT,
        "STOP" => libc::SIGSTOP,
        _ => anyhow::bail!("unsupported signal: {}", sig),
    };
    Ok(value)
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

fn permission_denied_response(seq: u64) -> serde_json::Value {
    serde_json::json!({
        "type": "response", "id": "?", "seq": seq, "status": "error",
        "error": { "code": "PERMISSION_DENIED", "message": "action not permitted" }
    })
}

async fn build_system_capabilities(
    backend: &dyn crate::backend::DesktopBackend,
) -> anyhow::Result<serde_json::Value> {
    let desktop = backend.system_info().await?.desktop.to_lowercase();
    let mut actions = serde_json::Map::new();
    for action in crate::protocol::Action::public_action_types() {
        actions.insert(
            (*action).to_string(),
            serde_json::json!({
                "supported": true,
                "degraded": false,
                "reason": serde_json::Value::Null,
                "requires": [],
                "session": "any",
                "degraded_modes": []
            }),
        );
    }

    if desktop.contains("gnome") {
        set_degraded(
            &mut actions,
            "input.mouse",
            "absolute_move_may_be_unavailable_without_screencast",
        );
        set_requires(&mut actions, "windows.list", &["gnome-extension"]);
        set_requires(&mut actions, "windows.focus", &["gnome-extension"]);
        set_requires(&mut actions, "windows.close", &["gnome-extension"]);
        set_requires(&mut actions, "windows.minimize", &["gnome-extension"]);
        set_requires(&mut actions, "windows.maximize", &["gnome-extension"]);
        set_requires(&mut actions, "windows.move_resize", &["gnome-extension"]);
        set_requires(
            &mut actions,
            "windows.activate_or_launch",
            &["gnome-extension"],
        );
        set_requires(&mut actions, "workspaces.list", &["gnome-extension"]);
        set_requires(&mut actions, "workspaces.switch", &["gnome-extension"]);
        set_session(&mut actions, "input.mouse", "wayland");
    }

    if desktop.contains("kde") || desktop.contains("hyprland") {
        set_degraded(
            &mut actions,
            "input.keyboard",
            "depends_on_ydotoold_and_uinput_permissions",
        );
        set_degraded(
            &mut actions,
            "input.mouse",
            "depends_on_ydotoold_and_uinput_permissions",
        );
        set_requires(&mut actions, "input.keyboard", &["ydotoold", "/dev/uinput"]);
        set_requires(&mut actions, "input.mouse", &["ydotoold", "/dev/uinput"]);
        set_session(&mut actions, "input.keyboard", "wayland");
        set_session(&mut actions, "input.mouse", "wayland");
    }

    if desktop.contains("x11") {
        set_degraded(
            &mut actions,
            "windows.activate_or_launch",
            "x11_window_enumeration_unavailable_launch_only",
        );
        set_degraded(
            &mut actions,
            "layout_profiles.save",
            "x11_window_enumeration_unavailable",
        );
        set_degraded(
            &mut actions,
            "layout_profiles.restore",
            "x11_window_enumeration_unavailable",
        );
        set_requires(&mut actions, "windows.maximize", &["wmctrl"]);
        // X11 backend doesn't support notification actions via GNOME/KDE APIs
        set_unsupported(&mut actions, "notification.send", "x11_unsupported");
        set_unsupported(&mut actions, "notification.close", "x11_unsupported");
        set_unsupported(&mut actions, "screencast.start", "x11_unsupported");
        set_unsupported(&mut actions, "screencast.stop", "x11_unsupported");
    }

    for action in [
        "ui.tree.get",
        "ui.element.click",
        "ui.element.set_text",
        "bluetooth.pair",
        "bluetooth.forget",
    ] {
        set_unsupported(&mut actions, action, "not_implemented");
    }

    if desktop.contains("hyprland") {
        set_unsupported(
            &mut actions,
            "windows.minimize",
            "hyprland_has_no_native_minimize_dispatcher",
        );
    }

    Ok(serde_json::json!({
        "schema_version": 1,
        "backend": desktop,
        "actions": actions,
        "backend_notes": {
            "gnome": "window control via Shell extension + Mutter DBus",
            "kde": "window control via KWin scripting/DBus",
            "hyprland": "window control via hyprctl dispatch"
        }
    }))
}

async fn build_system_health(
    backend: &dyn crate::backend::DesktopBackend,
) -> anyhow::Result<serde_json::Value> {
    let desktop = backend.system_info().await?.desktop.to_lowercase();
    let mut deps = serde_json::Map::new();

    if desktop.contains("gnome") {
        deps.insert(
            "gnome-extension".to_string(),
            check_cmd(
                "gdbus",
                &[
                    "introspect",
                    "--session",
                    "--dest",
                    "org.deskbrid.WindowManager",
                    "--object-path",
                    "/org/deskbrid/WindowManager",
                ],
            ),
        );
        deps.insert("grim".to_string(), check_in_path("grim"));
        deps.insert("wl_clipboard".to_string(), check_clipboard_tools());
    } else if desktop.contains("kde") {
        deps.insert("qdbus6".to_string(), check_in_path("qdbus6"));
        deps.insert("spectacle".to_string(), check_in_path("spectacle"));
        deps.insert("imagemagick_convert".to_string(), check_in_path("convert"));
        deps.insert("ydotoold".to_string(), check_process("ydotoold").await);
        deps.insert("ydotool".to_string(), check_in_path("ydotool"));

        deps.insert("uinput".to_string(), check_uinput());
    } else if desktop.contains("hyprland") {
        deps.insert("hyprctl".to_string(), check_in_path("hyprctl"));
        deps.insert("ydotoold".to_string(), check_process("ydotoold").await);
        deps.insert("ydotool".to_string(), check_in_path("ydotool"));

        deps.insert("uinput".to_string(), check_uinput());
        deps.insert("grim".to_string(), check_in_path("grim"));
    }

    Ok(serde_json::json!({
        "schema_version": 1,
        "backend": desktop,
        "deps": deps,
        "remediation": health_remediation()
    }))
}

fn set_degraded(
    actions: &mut serde_json::Map<String, serde_json::Value>,
    action: &str,
    reason: &str,
) {
    actions.insert(
        action.to_string(),
        serde_json::json!({"supported": true, "degraded": true, "reason": reason, "requires": [], "session": "any", "degraded_modes": [reason]}),
    );
}

fn set_unsupported(
    actions: &mut serde_json::Map<String, serde_json::Value>,
    action: &str,
    reason: &str,
) {
    actions.insert(
        action.to_string(),
        serde_json::json!({"supported": false, "degraded": false, "reason": reason, "requires": [], "session": "any", "degraded_modes": []}),
    );
}

fn set_requires(
    actions: &mut serde_json::Map<String, serde_json::Value>,
    action: &str,
    requires: &[&str],
) {
    if let Some(v) = actions.get_mut(action) {
        v["requires"] = serde_json::json!(requires);
    }
}

fn set_session(
    actions: &mut serde_json::Map<String, serde_json::Value>,
    action: &str,
    session: &str,
) {
    if let Some(v) = actions.get_mut(action) {
        v["session"] = serde_json::json!(session);
    }
}
fn check_in_path(cmd: &str) -> serde_json::Value {
    match std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {} >/dev/null 2>&1", cmd))
        .status()
    {
        Ok(status) if status.success() => serde_json::json!({"ok": true, "details": "present"}),
        Ok(_) => serde_json::json!({"ok": false, "details": "missing"}),
        Err(e) => serde_json::json!({"ok": false, "details": format!("check failed: {}", e)}),
    }
}

fn health_remediation() -> serde_json::Value {
    serde_json::json!({
        "ydotoold": "Start ydotoold in your user session (e.g. autostart entry).",
        "uinput": "Configure udev: KERNEL==\"uinput\", GROUP=\"input\", MODE=\"0660\" and add your user to input group.",
        "gnome-extension": "Install/enable deskbrid GNOME extension, then restart shell/session.",
        "grim": "Install grim package for screenshots.",
        "spectacle": "Install spectacle package for KDE screenshots."
    })
}

async fn run_system_remediation(check: &str, apply: bool) -> anyhow::Result<serde_json::Value> {
    match check {
        "ydotoold" => {
            if !apply {
                return Ok(serde_json::json!({
                    "check":"ydotoold",
                    "applied": false,
                    "command":"ydotoold &",
                    "note":"Set apply=true to start ydotoold in current user session"
                }));
            }
            tokio::process::Command::new("sh")
                .arg("-c")
                .arg("pgrep -x ydotoold >/dev/null 2>&1 || (nohup ydotoold >/tmp/deskbrid-ydotoold.log 2>&1 &)")
                .output()
                .await?;
            // Don't trust nohup's exit code — it exits 0 even if ydotoold crashes immediately.
            // Verify the process actually started.
            let running = tokio::process::Command::new("pgrep")
                .args(["-x", "ydotoold"])
                .output()
                .await
                .map(|o| o.status.success())
                .unwrap_or(false);
            Ok(serde_json::json!({
                "check":"ydotoold",
                "applied": running,
                "details": if running { "started_or_already_running" } else { "failed_to_start" }
            }))
        }
        "kde_ydotoold_autostart" => {
            let home = std::env::var("HOME").unwrap_or_default();
            let path = format!("{}/.config/autostart/ydotoold.desktop", home);
            if !apply {
                return Ok(
                    serde_json::json!({"check":"kde_ydotoold_autostart","applied":false,"path":path}),
                );
            }
            tokio::fs::create_dir_all(format!("{}/.config/autostart", home)).await?;
            let desktop = "[Desktop Entry]\nType=Application\nExec=ydotoold\nHidden=false\nNoDisplay=false\nX-GNOME-Autostart-enabled=true\nName=Deskbrid ydotool Daemon\nComment=Auto-start ydotoold for input injection\n";
            tokio::fs::write(&path, desktop).await?;
            Ok(serde_json::json!({"check":"kde_ydotoold_autostart","applied":true,"path":path}))
        }
        _ => Ok(serde_json::json!({"check": check,"applied": false,"error": "unknown check"})),
    }
}

fn normalize_coords(
    info: &crate::protocol::SystemInfo,
    x: f64,
    y: f64,
    monitor: Option<u32>,
) -> serde_json::Value {
    let target = monitor
        .and_then(|m| info.monitors.iter().find(|mon| mon.id == m))
        .or_else(|| info.monitors.iter().find(|m| m.primary))
        .or_else(|| info.monitors.first());
    if let Some(mon) = target {
        let px = (x * mon.scale).round();
        let py = (y * mon.scale).round();
        serde_json::json!({
            "input": {"x": x, "y": y, "monitor": monitor},
            "monitor": {"id": mon.id, "name": mon.name, "scale": mon.scale, "width": mon.width, "height": mon.height},
            "backend_coords": {"x": px, "y": py}
        })
    } else {
        serde_json::json!({
            "input": {"x": x, "y": y, "monitor": monitor},
            "backend_coords": {"x": x, "y": y},
            "note": "no monitor metadata available"
        })
    }
}

async fn check_process(proc_name: &str) -> serde_json::Value {
    match tokio::process::Command::new("pgrep")
        .args(["-x", proc_name])
        .output()
        .await
    {
        Ok(out) if out.status.success() => serde_json::json!({"ok": true, "details": "running"}),
        Ok(_) => serde_json::json!({"ok": false, "details": "not running"}),
        Err(e) => serde_json::json!({"ok": false, "details": format!("check failed: {}", e)}),
    }
}

fn check_cmd(cmd: &str, args: &[&str]) -> serde_json::Value {
    match std::process::Command::new(cmd).args(args).output() {
        Ok(out) if out.status.success() => serde_json::json!({"ok": true, "details": "reachable"}),
        Ok(out) => {
            serde_json::json!({"ok": false, "details": format!("failed (code {:?})", out.status.code())})
        }
        Err(e) => serde_json::json!({"ok": false, "details": format!("check failed: {}", e)}),
    }
}

fn check_uinput() -> serde_json::Value {
    let path = std::path::Path::new("/dev/uinput");
    if !path.exists() {
        return serde_json::json!({"ok": false, "details": "missing /dev/uinput"});
    }
    match std::fs::OpenOptions::new().write(true).open(path) {
        Ok(_) => serde_json::json!({"ok": true, "details": "write access"}),
        Err(e) => serde_json::json!({"ok": false, "details": format!("no write access: {}", e)}),
    }
}
fn check_clipboard_tools() -> serde_json::Value {
    let copy = std::process::Command::new("sh")
        .arg("-c")
        .arg("command -v wl-copy >/dev/null 2>&1")
        .status();
    let paste = std::process::Command::new("sh")
        .arg("-c")
        .arg("command -v wl-paste >/dev/null 2>&1")
        .status();

    let copy_ok = copy.map(|s| s.success()).unwrap_or(false);
    let paste_ok = paste.map(|s| s.success()).unwrap_or(false);

    if copy_ok && paste_ok {
        serde_json::json!({"ok": true, "details": "wl-copy and wl-paste present"})
    } else {
        let mut missing = Vec::new();
        if !copy_ok {
            missing.push("wl-copy");
        }
        if !paste_ok {
            missing.push("wl-paste");
        }
        serde_json::json!({"ok": false, "details": format!("missing: {}", missing.join(", "))})
    }
}
