// reason: exhaustive match on 90+ Action enum variants — cannot split without breaking exhaustiveness
use crate::protocol::Action;
use anyhow::Context;
use tokio::io::AsyncWriteExt;

use super::layout::*;
use super::helpers::*;
use super::capabilities::{build_system_capabilities, build_system_health, run_system_remediation, normalize_coords};

pub async fn execute_action(
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

    FilesRead {
        ref path,
        offset,
        limit,
    } => {
        use tokio::io::{AsyncReadExt, AsyncSeekExt};
        let path = expand_path(path)?;
        let mut file = tokio::fs::File::open(&path)
            .await
            .with_context(|| format!("failed to open {}", path.display()))?;
        let metadata = file.metadata().await?;
        // Cap reads at 10 MB to avoid OOM on large files
        let max_read = 10 * 1024 * 1024u64;
        let limit = limit.unwrap_or(max_read).min(max_read);
        if let Some(off) = offset {
            file.seek(std::io::SeekFrom::Start(off)).await?;
        }
        let mut buf = vec![0u8; limit as usize];
        let n = file.read(&mut buf).await?;
        buf.truncate(n);
        // Try UTF-8; fall back to base64 for binary files
        match String::from_utf8(buf) {
            Ok(text) => serde_json::json!({
                "path": path.to_string_lossy(),
                "content": text,
                "bytes": n,
                "size": metadata.len(),
                "encoding": "utf-8",
            }),
            Err(e) => {
                let bytes = e.into_bytes();
                serde_json::json!({
                    "path": path.to_string_lossy(),
                    "content": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes),
                    "bytes": n,
                    "size": metadata.len(),
                    "encoding": "base64",
                })
            }
        }
    }
    FilesWrite {
        ref path,
        ref content,
        append,
    } => {
                let path = expand_path(path)?;
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let mut file = if append {
            tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .await?
        } else {
            tokio::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&path)
                .await?
        };
        file.write_all(content.as_bytes()).await?;
        file.flush().await?;
        serde_json::json!({"path": path.to_string_lossy(), "bytes_written": content.len()})
    }
    FilesCopy {
        ref source,
        ref destination,
    } => {
        let src = expand_path(source)?;
        let dst = expand_path(destination)?;
        if src.is_dir() {
            anyhow::bail!("directory copy not supported — use process.start with cp -r");
        }
        if let Some(parent) = dst.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::copy(&src, &dst).await.with_context(|| {
            format!("failed to copy {} -> {}", src.display(), dst.display())
        })?;
        serde_json::json!({"source": src.to_string_lossy(), "destination": dst.to_string_lossy()})
    }
    FilesMove {
        ref source,
        ref destination,
    } => {
        let src = expand_path(source)?;
        let dst = expand_path(destination)?;
        if let Some(parent) = dst.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::rename(&src, &dst).await.with_context(|| {
            format!("failed to move {} -> {}", src.display(), dst.display())
        })?;
        serde_json::json!({"source": src.to_string_lossy(), "destination": dst.to_string_lossy()})
    }
    FilesDelete {
        ref path,
        recursive,
    } => {
        let path = expand_path(path)?;
        if path.is_dir() {
            if recursive {
                tokio::fs::remove_dir_all(&path).await?;
            } else {
                tokio::fs::remove_dir(&path).await?;
            }
        } else {
            tokio::fs::remove_file(&path).await?;
        }
        serde_json::json!({"deleted": path.to_string_lossy()})
    }
    FilesMkdir { ref path, parents } => {
        let path = expand_path(path)?;
        if parents {
            tokio::fs::create_dir_all(&path).await?;
        } else {
            tokio::fs::create_dir(&path).await?;
        }
        serde_json::json!({"created": path.to_string_lossy()})
    }
    FilesList { ref path } => {
        let path = expand_path(path)?;
        let mut entries = Vec::new();
        let mut dir = tokio::fs::read_dir(&path)
            .await
            .with_context(|| format!("failed to list {}", path.display()))?;
        while let Some(entry) = dir.next_entry().await? {
            let metadata = entry.metadata().await?;
            entries.push(serde_json::json!({
                "name": entry.file_name().to_string_lossy(),
                "is_dir": metadata.is_dir(),
                "size": metadata.len(),
                "modified": metadata.modified().ok().map(|t| {
                    t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
                }),
            }));
        }
        entries.sort_by(|a, b| {
            let a_dir = a.get("is_dir").and_then(|v| v.as_bool()).unwrap_or(false);
            let b_dir = b.get("is_dir").and_then(|v| v.as_bool()).unwrap_or(false);
            b_dir.cmp(&a_dir).then_with(|| {
                a.get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .cmp(b.get("name").and_then(|v| v.as_str()).unwrap_or(""))
            })
        });
        serde_json::json!({"path": path.to_string_lossy(), "entries": entries})
    }

    // Browser (Chrome DevTools Protocol)
    BrowserListTabs => crate::browser::list_tabs().await?,
    BrowserNavigate { tab_index, ref url } => crate::browser::navigate(tab_index, url).await?,
    BrowserEvaluate {
        tab_index,
        ref expression,
        await_promise,
    } => crate::browser::evaluate(tab_index, expression, await_promise).await?,
    BrowserScreenshotTab { tab_index } => crate::browser::screenshot_tab(tab_index).await?,
    BrowserClick {
        tab_index,
        ref selector,
    } => crate::browser::click(tab_index, selector).await?,

    // Accessibility (AT-SPI2)
    A11yTree { depth } => crate::a11y::tree(depth).await?,
    A11yGetElement {
        role,
        ref name,
        index,
    } => crate::a11y::get_element(role.as_deref(), name.as_deref(), index).await?,
    A11yClickElement {
        role,
        ref name,
        index,
    } => crate::a11y::click_element(role.as_deref(), name.as_deref(), index).await?,
    A11yGetText {
        role,
        ref name,
        index,
    } => crate::a11y::get_text(role.as_deref(), name.as_deref(), index).await?,

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
    MonitorSetPrimary { ref output } => {
        backend.monitor_set_primary(output).await?;
        serde_json::json!({"output": output, "primary": true})
    }
    MonitorSetResolution {
        ref output,
        width,
        height,
        refresh_rate,
    } => {
        backend
            .monitor_set_resolution(output, width, height, refresh_rate)
            .await?;
        serde_json::json!({
            "output": output,
            "width": width,
            "height": height,
            "refresh_rate": refresh_rate
        })
    }
    MonitorSetScale { ref output, scale } => {
        backend.monitor_set_scale(output, scale).await?;
        serde_json::json!({"output": output, "scale": scale})
    }
    MonitorSetRotation {
        ref output,
        ref rotation,
    } => {
        backend.monitor_set_rotation(output, rotation).await?;
        serde_json::json!({"output": output, "rotation": rotation})
    }
    MonitorEnable { ref output } => {
        backend.monitor_set_enabled(output, true).await?;
        serde_json::json!({"output": output, "enabled": true})
    }
    MonitorDisable { ref output } => {
        backend.monitor_set_enabled(output, false).await?;
        serde_json::json!({"output": output, "enabled": false})
    }
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

