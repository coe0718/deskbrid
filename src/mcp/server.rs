//! MCP server — rmcp-based stdio server for `deskbrid mcp`.
//!
//! Each tool is a thin `#[tool]` wrapper delegating to async helpers.

#![allow(dead_code)]

use super::helpers::*;
use super::types::*;
use crate::DaemonState;
use crate::mcp::tools_agent::{BroadcastArgs, SendMessageArgs};
use crate::mcp::tools_confirmation::ConfirmActionArgs;
use crate::mcp::tools_search::SearchArgs;
use crate::mcp::tools_secrets::{SecretsGetArgs, SecretsStoreArgs};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::runtime::Handle;

#[derive(Clone)]
pub struct McpServer {
    state: Arc<DaemonState>,
    rt: Handle,
}

impl McpServer {
    pub fn new(state: Arc<DaemonState>) -> Self {
        Self {
            state,
            rt: Handle::current(),
        }
    }

    async fn call(&self, f: impl std::future::Future<Output = anyhow::Result<Value>>) -> String {
        f.await
            .unwrap_or_else(|e| json!({"error": e.to_string()}))
            .to_string()
    }

    async fn exec(&self, action: &str, args: Value) -> String {
        match do_execute_with(&self.state, action, args).await {
            Ok(v) => v.to_string(),
            Err(e) if e.to_string().contains("no backend") => {
                json!({"headless": true, "note": "Running in Docker/headless mode — no desktop backend available"}).to_string()
            }
            Err(e) => json!({"error": e.to_string()}).to_string(),
        }
    }
}

#[tool_router(server_handler)]
impl McpServer {
    #[tool(
        name = "list_apps",
        description = "List AT-SPI application roots running on the desktop.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn list_apps(&self) -> String {
        self.call(do_list_apps(&self.state)).await
    }

    #[tool(
        name = "get_accessibility_tree",
        description = "Full AT-SPI tree for an app or window with bounds, roles, states, actions, and text.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_accessibility_tree(
        &self,
        Parameters(A11yTree {
            app_name,
            pid,
            max_nodes,
            max_depth,
        }): Parameters<A11yTree>,
    ) -> String {
        self.call(do_get_accessibility_tree(
            &self.state,
            app_name.as_deref(),
            pid,
            max_nodes,
            max_depth,
        ))
        .await
    }

    #[tool(
        name = "perform_action",
        description = "Perform an AT-SPI action on an accessibility element (click, activate, toggle).",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn perform_action(
        &self,
        Parameters(A11yAction {
            object_ref,
            action_name,
        }): Parameters<A11yAction>,
    ) -> String {
        self.call(do_perform_action(
            &self.state,
            &object_ref,
            action_name.as_deref(),
        ))
        .await
    }

    #[tool(
        name = "set_element_value",
        description = "Set the text value of an AT-SPI editable element.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn set_element_value(
        &self,
        Parameters(SetValue { object_ref, value }): Parameters<SetValue>,
    ) -> String {
        self.call(do_set_element_value(&self.state, &object_ref, &value))
            .await
    }

    #[tool(
        name = "get_element_text",
        description = "Get the text content from an AT-SPI element.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_element_text(
        &self,
        Parameters(GetText {
            object_ref,
            max_chars,
        }): Parameters<GetText>,
    ) -> String {
        self.call(do_get_element_text(&self.state, &object_ref, max_chars))
            .await
    }

    #[tool(
        name = "click_element",
        description = "Click an AT-SPI element using its bounds, falling back to coordinate click.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn click_element(
        &self,
        Parameters(ClickElement { object_ref }): Parameters<ClickElement>,
    ) -> String {
        self.call(do_click_element(&self.state, &object_ref)).await
    }

    #[tool(
        name = "send_message",
        description = "Send a message to another agent session's mailbox. Messages persist until retrieved via check_mailbox.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn send_message(&self, Parameters(args): Parameters<SendMessageArgs>) -> String {
        self.exec(
            "agent.message",
            json!({
                "to_session": args.to_session,
                "subject": args.subject,
                "body": serde_json::to_value(&args.body).unwrap_or(serde_json::Value::Null),
                "ttl_ms": args.ttl_ms,
                "reply_to": args.reply_to,
            }),
        )
        .await
    }

    #[tool(
        name = "broadcast",
        description = "Broadcast a message to all connected agent sessions.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn broadcast(&self, Parameters(args): Parameters<BroadcastArgs>) -> String {
        self.exec(
            "agent.broadcast",
            json!({
                "subject": args.subject,
                "body": serde_json::to_value(&args.body).unwrap_or(serde_json::Value::Null),
                "exclude_self": args.exclude_self,
            }),
        )
        .await
    }

    #[tool(
        name = "check_mailbox",
        description = "Check the agent mailbox for received messages.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn check_mailbox(&self) -> String {
        self.exec("agent.mailbox", json!({})).await
    }

    #[tool(
        name = "list_audio_sinks",
        description = "List audio output devices with volume and mute state.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn list_audio_sinks(&self) -> String {
        self.call(do_execute(&self.state, "audio.list_sinks", json!({})))
            .await
    }

    #[tool(
        name = "list_audio_sources",
        description = "List audio input devices (microphones) with volume and mute state.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn list_audio_sources(&self) -> String {
        self.call(do_execute(&self.state, "audio.list_sources", json!({})))
            .await
    }

    #[tool(
        name = "get_audio_volume",
        description = "Get volume level for a sink or source.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_audio_volume(
        &self,
        Parameters(AudioTargetParams { target, id }): Parameters<AudioTargetParams>,
    ) -> String {
        self.call(do_execute(
            &self.state,
            "audio.get_volume",
            json!({"target": target, "id": id}),
        ))
        .await
    }

    #[tool(
        name = "set_audio_volume",
        description = "Set volume for a sink or source. Volume is 0.0-1.0.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn set_audio_volume(
        &self,
        Parameters(SetVolume { sink_id, volume }): Parameters<SetVolume>,
    ) -> String {
        self.exec(
            "audio.set_sink_volume",
            json!({"sink_id": sink_id, "volume": volume}),
        )
        .await
    }

    #[tool(
        name = "set_audio_node_volume",
        description = "Set volume for any audio node (sink or source) by ID. Volume is 0.0-1.0.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn set_audio_node_volume(
        &self,
        Parameters(AudioVolumeParams { target, id, volume }): Parameters<AudioVolumeParams>,
    ) -> String {
        self.exec(
            "audio.set_volume",
            json!({"target": target, "id": id, "volume": volume}),
        )
        .await
    }

    #[tool(
        name = "mute_audio",
        description = "Mute or unmute a sink or source.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn mute_audio(
        &self,
        Parameters(AudioMuteParams { target, id, mute }): Parameters<AudioMuteParams>,
    ) -> String {
        self.exec(
            "audio.mute",
            json!({"target": target, "id": id, "mute": mute}),
        )
        .await
    }

    #[tool(
        name = "set_default_audio",
        description = "Set the default sink or source by name.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn set_default_audio(
        &self,
        Parameters(AudioDefaultParams { target, name }): Parameters<AudioDefaultParams>,
    ) -> String {
        self.exec("audio.set_default", json!({"target": target, "name": name}))
            .await
    }

    #[tool(
        name = "bluetooth_list",
        description = "List paired Bluetooth devices.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn bluetooth_list(&self) -> String {
        self.call(do_execute(&self.state, "bluetooth.list", json!({})))
            .await
    }

    #[tool(
        name = "bluetooth_scan",
        description = "Scan for nearby Bluetooth devices.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn bluetooth_scan(
        &self,
        Parameters(BluetoothScan { duration }): Parameters<BluetoothScan>,
    ) -> String {
        let mut args = json!({});
        if let Some(d) = duration {
            args["duration"] = json!(d);
        }
        self.exec("bluetooth.scan", args).await
    }

    #[tool(
        name = "list_browser_tabs",
        description = "List open browser tabs via Chrome DevTools Protocol. Requires Chrome/Chromium with remote debugging enabled.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn list_browser_tabs(&self) -> String {
        self.call(do_execute(&self.state, "browser.list_tabs", json!({})))
            .await
    }

    #[tool(
        name = "browser_navigate",
        description = "Navigate a browser tab to a URL.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn browser_navigate(
        &self,
        Parameters(BrowserNavigate { tab_index, url }): Parameters<BrowserNavigate>,
    ) -> String {
        let mut args = json!({"url": url});
        if let Some(t) = tab_index {
            args["tab_index"] = json!(t);
        }
        self.exec("browser.navigate", args).await
    }

    #[tool(
        name = "browser_evaluate",
        description = "Evaluate JavaScript in a browser tab and return the result.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn browser_evaluate(
        &self,
        Parameters(BrowserEvaluate {
            tab_index,
            expression,
            await_promise,
        }): Parameters<BrowserEvaluate>,
    ) -> String {
        let mut args = json!({"expression": expression, "await_promise": await_promise});
        if let Some(t) = tab_index {
            args["tab_index"] = json!(t);
        }
        self.exec("browser.evaluate", args).await
    }

    #[tool(
        name = "browser_screenshot",
        description = "Take a screenshot of a browser tab.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn browser_screenshot(
        &self,
        Parameters(TabIndex { tab_index }): Parameters<TabIndex>,
    ) -> String {
        let mut args = json!({});
        if let Some(t) = tab_index {
            args["tab_index"] = json!(t);
        }
        self.exec("browser.screenshot_tab", args).await
    }

    #[tool(
        name = "browser_click",
        description = "Click an element in a browser tab by CSS selector.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn browser_click(
        &self,
        Parameters(BrowserClick {
            tab_index,
            selector,
        }): Parameters<BrowserClick>,
    ) -> String {
        let mut args = json!({"selector": selector});
        if let Some(t) = tab_index {
            args["tab_index"] = json!(t);
        }
        self.exec("browser.click", args).await
    }

    #[tool(
        name = "clipboard_read",
        description = "Read the current clipboard contents.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn clipboard_read(&self) -> String {
        self.call(do_execute(&self.state, "clipboard.read", json!({})))
            .await
    }

    #[tool(
        name = "clipboard_write",
        description = "Write text to the system clipboard.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn clipboard_write(
        &self,
        Parameters(ClipboardWrite { text }): Parameters<ClipboardWrite>,
    ) -> String {
        self.call(do_clipboard_write(&self.state, &text)).await
    }

    #[tool(
        name = "confirm_action",
        description = "Confirm a pending destructive action. Requires the confirmation ID from the pending list.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn confirm_action(
        &self,
        Parameters(ConfirmActionArgs { id }): Parameters<ConfirmActionArgs>,
    ) -> String {
        self.exec("confirmation.confirm", json!({"id": id})).await
    }

    #[tool(
        name = "deny_action",
        description = "Deny/reject a pending destructive action. Requires the confirmation ID.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn deny_action(
        &self,
        Parameters(ConfirmActionArgs { id }): Parameters<ConfirmActionArgs>,
    ) -> String {
        self.exec("confirmation.deny", json!({"id": id})).await
    }

    #[tool(
        name = "list_confirmations",
        description = "List all pending action confirmations waiting for approval.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_confirmations(&self) -> String {
        self.exec("confirmation.list", json!({})).await
    }

    #[tool(
        name = "list_schemas",
        description = "List all available gsettings schemas on the system.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn list_schemas(&self) -> String {
        self.call(do_execute(&self.state, "desktop.list_schemas", json!({})))
            .await
    }

    #[tool(
        name = "get_setting",
        description = "Read a desktop setting value by schema and key (e.g. 'org.gnome.desktop.interface', 'gtk-theme').",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_setting(
        &self,
        Parameters(DesktopSettingKey { schema, key }): Parameters<DesktopSettingKey>,
    ) -> String {
        self.exec("desktop.get_setting", json!({"schema": schema, "key": key}))
            .await
    }

    #[tool(
        name = "set_setting",
        description = "Write a desktop setting value by schema, key, and value.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn set_setting(
        &self,
        Parameters(DesktopSettingValue { schema, key, value }): Parameters<DesktopSettingValue>,
    ) -> String {
        self.exec(
            "desktop.set_setting",
            json!({"schema": schema, "key": key, "value": value}),
        )
        .await
    }

    #[tool(
        name = "file_list",
        description = "List files and directories at a path.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn file_list(&self, Parameters(FilePath { path }): Parameters<FilePath>) -> String {
        self.exec("files.list", json!({"path": path})).await
    }

    #[tool(
        name = "file_read",
        description = "Read contents of a file.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn file_read(
        &self,
        Parameters(FileRead {
            path,
            offset,
            limit,
        }): Parameters<FileRead>,
    ) -> String {
        let mut args = json!({"path": path});
        if let Some(o) = offset {
            args["offset"] = json!(o);
        }
        if let Some(l) = limit {
            args["limit"] = json!(l);
        }
        self.exec("files.read", args).await
    }

    #[tool(
        name = "file_write",
        description = "Write content to a file (create or overwrite).",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn file_write(
        &self,
        Parameters(FileWrite {
            path,
            content,
            append,
        }): Parameters<FileWrite>,
    ) -> String {
        self.exec(
            "files.write",
            json!({"path": path, "content": content, "append": append}),
        )
        .await
    }

    #[tool(
        name = "file_search",
        description = "Search filesystem by glob or regex pattern.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn file_search(
        &self,
        Parameters(FileSearch {
            pattern,
            root,
            max_results,
        }): Parameters<FileSearch>,
    ) -> String {
        let mut args = json!({"pattern": pattern, "max_results": max_results});
        if let Some(r) = root {
            args["root"] = json!(r);
        }
        self.exec("files.search", args).await
    }

    #[tool(
        name = "file_copy",
        description = "Copy a file or directory.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn file_copy(
        &self,
        Parameters(FileCopy {
            source,
            destination,
        }): Parameters<FileCopy>,
    ) -> String {
        self.exec(
            "files.copy",
            json!({"source": source, "destination": destination}),
        )
        .await
    }

    #[tool(
        name = "file_watch",
        description = "Watch a path for file changes. Returns a watch ID.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn file_watch(
        &self,
        Parameters(FileWatch {
            path,
            recursive,
            patterns,
        }): Parameters<FileWatch>,
    ) -> String {
        let mut args = json!({"path": path, "recursive": recursive});
        if let Some(p) = patterns {
            args["patterns"] = json!(p);
        }
        self.exec("files.watch", args).await
    }

    #[tool(
        name = "type_text",
        description = "Type a string via keyboard input.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn type_text(&self, Parameters(TypeText { text }): Parameters<TypeText>) -> String {
        self.call(do_type_text(&self.state, &text)).await
    }

    #[tool(
        name = "press_key",
        description = "Press a single key (e.g. 'Return', 'Escape', 'Tab').",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn press_key(&self, Parameters(PressKey { key }): Parameters<PressKey>) -> String {
        self.exec("input.keyboard", json!({"key": key})).await
    }

    #[tool(
        name = "press_keys",
        description = "Press a key combination (e.g. ['Control_L', 'c'] for Ctrl+C).",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn press_keys(&self, Parameters(PressKeys { keys }): Parameters<PressKeys>) -> String {
        self.call(do_press_keys(&self.state, &keys)).await
    }

    #[tool(
        name = "mouse_move",
        description = "Move the mouse cursor to absolute coordinates.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn mouse_move(&self, Parameters(MouseMove { x, y }): Parameters<MouseMove>) -> String {
        self.call(do_mouse_move(&self.state, x, y)).await
    }

    #[tool(
        name = "mouse_click",
        description = "Click a mouse button at the current position.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn mouse_click(
        &self,
        Parameters(MouseClick { button }): Parameters<MouseClick>,
    ) -> String {
        self.call(do_mouse_click(&self.state, &button)).await
    }

    #[tool(
        name = "mouse_scroll",
        description = "Scroll the mouse wheel. Negative dy scrolls down.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn mouse_scroll(
        &self,
        Parameters(MouseScroll { dx, dy }): Parameters<MouseScroll>,
    ) -> String {
        self.exec(
            "input.mouse",
            json!({"action": "scroll", "dx": dx, "dy": dy}),
        )
        .await
    }

    #[tool(
        name = "click_coordinate",
        description = "Move to pixel coordinates and click.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn click_coordinate(
        &self,
        Parameters(ClickCoord { x, y, button }): Parameters<ClickCoord>,
    ) -> String {
        self.call(do_click_coordinate(&self.state, x, y, &button))
            .await
    }

    #[tool(
        name = "drag",
        description = "Click-and-drag between two pixel coordinates.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn drag(
        &self,
        Parameters(Drag {
            from_x,
            from_y,
            to_x,
            to_y,
            button,
        }): Parameters<Drag>,
    ) -> String {
        self.call(do_drag(&self.state, from_x, from_y, to_x, to_y, &button))
            .await
    }

    #[tool(
        name = "list_media_players",
        description = "List MPRIS media players on the D-Bus session bus.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn list_media_players(&self) -> String {
        self.call(do_execute(&self.state, "mpris.list", json!({})))
            .await
    }

    #[tool(
        name = "media_player_info",
        description = "Get detailed info about an MPRIS media player (track, artist, album, position, playback status).",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn media_player_info(
        &self,
        Parameters(MprisPlayer { player }): Parameters<MprisPlayer>,
    ) -> String {
        let mut args = json!({});
        if let Some(p) = player {
            args["player"] = json!(p);
        }
        self.exec("mpris.get", args).await
    }

    #[tool(
        name = "media_player_control",
        description = "Control an MPRIS media player (play, pause, next, previous, stop).",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn media_player_control(
        &self,
        Parameters(MprisControl { player, action }): Parameters<MprisControl>,
    ) -> String {
        let mut args = json!({"action": action});
        if let Some(p) = player {
            args["player"] = json!(p);
        }
        self.exec("mpris.control", args).await
    }

    #[tool(
        name = "doctor",
        description = "Check AT-SPI accessibility readiness. Returns dependency status and fixes needed.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn doctor(&self) -> String {
        self.call(do_doctor(&self.state)).await
    }

    #[tool(
        name = "setup_accessibility",
        description = "Enable AT-SPI accessibility via gsettings. Requires user session.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn setup_accessibility(&self) -> String {
        self.call(do_setup_accessibility(&self.state)).await
    }

    #[tool(
        name = "capabilities",
        description = "List all available Deskbrid capabilities and tool types.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn capabilities(&self) -> String {
        self.call(do_capabilities(&self.state)).await
    }

    #[tool(
        name = "layout_list",
        description = "List saved window layout profiles.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn layout_list(&self) -> String {
        self.call(do_execute(&self.state, "layout_profiles.list", json!({})))
            .await
    }

    #[tool(
        name = "layout_save",
        description = "Save current window layout as a named profile.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn layout_save(
        &self,
        Parameters(LayoutSave { name, overwrite }): Parameters<LayoutSave>,
    ) -> String {
        self.exec(
            "layout_profiles.save",
            json!({"name": name, "overwrite": overwrite}),
        )
        .await
    }

    #[tool(
        name = "layout_restore",
        description = "Restore a saved window layout profile.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn layout_restore(
        &self,
        Parameters(LayoutName { name }): Parameters<LayoutName>,
    ) -> String {
        self.exec("layout_profiles.restore", json!({"name": name}))
            .await
    }

    #[tool(
        name = "layout_delete",
        description = "Delete a saved layout profile.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn layout_delete(
        &self,
        Parameters(LayoutName { name }): Parameters<LayoutName>,
    ) -> String {
        self.exec("layout_profiles.delete", json!({"name": name}))
            .await
    }

    #[tool(
        name = "register_hotkey",
        description = "Register a global hotkey combination.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn register_hotkey(
        &self,
        Parameters(HotkeyRegister { hotkey_id, keys }): Parameters<HotkeyRegister>,
    ) -> String {
        self.exec(
            "hotkeys.register",
            json!({"hotkey_id": hotkey_id, "keys": keys}),
        )
        .await
    }

    #[tool(
        name = "unregister_hotkey",
        description = "Unregister a previously registered hotkey.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn unregister_hotkey(
        &self,
        Parameters(HotkeyUnregister { hotkey_id }): Parameters<HotkeyUnregister>,
    ) -> String {
        self.exec("hotkeys.unregister", json!({"hotkey_id": hotkey_id}))
            .await
    }

    #[tool(
        name = "list_monitors",
        description = "List all connected monitors/displays with resolution, position, scale, and refresh rate.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn list_monitors(&self) -> String {
        self.call(do_execute(&self.state, "monitor.list", json!({})))
            .await
    }

    #[tool(
        name = "set_primary_monitor",
        description = "Set a monitor as the primary display.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn set_primary_monitor(
        &self,
        Parameters(MonitorOutput { output }): Parameters<MonitorOutput>,
    ) -> String {
        self.exec("monitor.set_primary", json!({"output": output}))
            .await
    }

    #[tool(
        name = "set_monitor_resolution",
        description = "Change a monitor's resolution and optionally refresh rate.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn set_monitor_resolution(
        &self,
        Parameters(SetResolution {
            output,
            width,
            height,
            refresh_rate,
        }): Parameters<SetResolution>,
    ) -> String {
        let mut args = json!({"output": output, "width": width, "height": height});
        if let Some(r) = refresh_rate {
            args["refresh_rate"] = json!(r);
        }
        self.exec("monitor.set_resolution", args).await
    }

    #[tool(
        name = "set_monitor_scale",
        description = "Set a monitor's display scale factor.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn set_monitor_scale(
        &self,
        Parameters(SetScale { output, scale }): Parameters<SetScale>,
    ) -> String {
        self.exec(
            "monitor.set_scale",
            json!({"output": output, "scale": scale}),
        )
        .await
    }

    #[tool(
        name = "set_monitor_rotation",
        description = "Rotate a monitor's display output.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn set_monitor_rotation(
        &self,
        Parameters(SetRotation { output, rotation }): Parameters<SetRotation>,
    ) -> String {
        self.exec(
            "monitor.set_rotation",
            json!({"output": output, "rotation": rotation}),
        )
        .await
    }

    #[tool(
        name = "enable_monitor",
        description = "Enable a previously disabled monitor.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn enable_monitor(
        &self,
        Parameters(MonitorOutput { output }): Parameters<MonitorOutput>,
    ) -> String {
        self.exec("monitor.enable", json!({"output": output})).await
    }

    #[tool(
        name = "disable_monitor",
        description = "Disable a monitor output.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn disable_monitor(
        &self,
        Parameters(MonitorOutput { output }): Parameters<MonitorOutput>,
    ) -> String {
        self.exec("monitor.disable", json!({"output": output}))
            .await
    }

    #[tool(
        name = "network_status",
        description = "Network interfaces, IP addresses, and connectivity state.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn network_status(&self) -> String {
        self.call(do_execute(&self.state, "network.status", json!({})))
            .await
    }

    #[tool(
        name = "send_notification",
        description = "Send a desktop notification via D-Bus.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn send_notification(
        &self,
        Parameters(NotificationSend {
            app_name,
            title,
            body,
            urgency,
        }): Parameters<NotificationSend>,
    ) -> String {
        self.exec(
            "notification.send",
            json!({"app_name": app_name, "title": title, "body": body, "urgency": urgency}),
        )
        .await
    }

    #[tool(
        name = "close_notification",
        description = "Close a desktop notification by ID.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn close_notification(
        &self,
        Parameters(NotificationClose { notification_id }): Parameters<NotificationClose>,
    ) -> String {
        self.exec(
            "notification.close",
            json!({"notification_id": notification_id}),
        )
        .await
    }

    #[tool(
        name = "portal_screenshot",
        description = "Take a screenshot via the XDG Desktop Portal (cross-Wayland compatible).",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn portal_screenshot(
        &self,
        Parameters(PortalScreenshotParams { interactive }): Parameters<PortalScreenshotParams>,
    ) -> String {
        self.exec("portal.screenshot", json!({"interactive": interactive}))
            .await
    }

    #[tool(
        name = "portal_screencast_start",
        description = "Start a screencast session via XDG Desktop Portal (requires PipeWire).",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn portal_screencast_start(
        &self,
        Parameters(ScreencastStartParams { output_path }): Parameters<ScreencastStartParams>,
    ) -> String {
        self.exec(
            "portal.screencast_start",
            json!({"output_path": output_path}),
        )
        .await
    }

    #[tool(
        name = "portal_screencast_stop",
        description = "Stop the running portal screencast session.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn portal_screencast_stop(&self) -> String {
        self.exec("portal.screencast_stop", json!({})).await
    }

    #[tool(
        name = "screencast_start",
        description = "Start recording the desktop to a video file via PipeWire/gst-launch.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn screencast_start(
        &self,
        Parameters(ScreencastStartParams { output_path }): Parameters<ScreencastStartParams>,
    ) -> String {
        self.exec("screencast.start", json!({ "output_path": output_path }))
            .await
    }

    #[tool(
        name = "screencast_stop",
        description = "Stop the running screencast recording.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn screencast_stop(&self) -> String {
        self.exec("screencast.stop", json!({})).await
    }

    #[tool(
        name = "screenshot",
        description = "Take a screenshot. Returns base64-encoded PNG.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn screenshot(&self) -> String {
        self.call(do_execute(&self.state, "screenshot", json!({})))
            .await
    }

    #[tool(
        name = "screenshot_region",
        description = "Capture a region of the screen or a specific window.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn screenshot_region(
        &self,
        Parameters(ScreenshotOptions {
            monitor,
            window_id,
            region_x,
            region_y,
            region_w,
            region_h,
        }): Parameters<ScreenshotOptions>,
    ) -> String {
        let mut args = json!({});
        if let Some(m) = monitor {
            args["monitor"] = json!(m);
        }
        if let Some(w) = window_id {
            args["window_id"] = json!(w);
        }
        if let (Some(x), Some(y), Some(w), Some(h)) = (region_x, region_y, region_w, region_h) {
            args["region"] = json!({"x": x, "y": y, "width": w, "height": h});
        }
        self.exec("screenshot", args).await
    }

    #[tool(
        name = "screenshot_diff",
        description = "Pixel diff between two screenshots. Useful for detecting UI changes.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn screenshot_diff(
        &self,
        Parameters(ScreenshotDiff {
            before_path,
            after_path,
            tolerance,
            diff_path,
            monitor,
        }): Parameters<ScreenshotDiff>,
    ) -> String {
        let mut args = json!({"before_path": before_path});
        if let Some(a) = after_path {
            args["after_path"] = json!(a);
        }
        if let Some(t) = tolerance {
            args["tolerance"] = json!(t);
        }
        if let Some(d) = diff_path {
            args["diff_path"] = json!(d);
            args["save_diff"] = json!(true);
        }
        if let Some(m) = monitor {
            args["monitor"] = json!(m);
        }
        self.exec("screenshot.diff", args).await
    }

    #[tool(
        name = "unified_search",
        description = "Search across windows, apps, files, clipboard history, and audit log in one query. Returns scored results ranked by relevance.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn unified_search(&self, Parameters(args): Parameters<SearchArgs>) -> String {
        self.exec(
            "search.query",
            serde_json::json!({
                "query": args.query,
                "categories": args.categories,
                "limit": args.limit,
            }),
        )
        .await
    }

    #[tool(
        name = "search_index_status",
        description = "Get search index statistics — indexed file count and last index time.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn search_index_status(&self) -> String {
        self.exec("search.index", serde_json::json!({})).await
    }

    #[tool(
        name = "secrets_list_collections",
        description = "List all keyring collections. Returns available secret collections from the Secret Service.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn secrets_list_collections(&self) -> String {
        self.exec("secrets.list_collections", json!({})).await
    }

    #[tool(
        name = "secrets_get_secret",
        description = "Look up a secret by its attributes (key=value pairs). Requires confirmation approval before returning the secret value.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn secrets_get_secret(
        &self,
        Parameters(SecretsGetArgs { attributes }): Parameters<SecretsGetArgs>,
    ) -> String {
        self.exec("secrets.get_secret", json!({"attributes": attributes}))
            .await
    }

    #[tool(
        name = "secrets_store_secret",
        description = "Store a secret in the keyring. Requires confirmation approval.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn secrets_store_secret(
        &self,
        Parameters(SecretsStoreArgs {
            attributes,
            secret,
            label,
            collection,
        }): Parameters<SecretsStoreArgs>,
    ) -> String {
        self.exec(
            "secrets.store_secret",
            json!({
                "attributes": attributes,
                "secret": secret,
                "label": label,
                "collection": collection,
            }),
        )
        .await
    }

    #[tool(
        name = "service_status",
        description = "Check a systemd service's status.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn service_status(
        &self,
        Parameters(ServiceName { name }): Parameters<ServiceName>,
    ) -> String {
        self.exec("service.status", json!({"name": name})).await
    }

    #[tool(
        name = "service_start",
        description = "Start a systemd service.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn service_start(
        &self,
        Parameters(ServiceName { name }): Parameters<ServiceName>,
    ) -> String {
        self.exec("service.start", json!({"name": name})).await
    }

    #[tool(
        name = "service_stop",
        description = "Stop a systemd service.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn service_stop(
        &self,
        Parameters(ServiceName { name }): Parameters<ServiceName>,
    ) -> String {
        self.exec("service.stop", json!({"name": name})).await
    }

    #[tool(
        name = "journal_query",
        description = "Query the systemd journal.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn journal_query(
        &self,
        Parameters(JournalQuery {
            since,
            until,
            unit,
            priority,
            tail,
        }): Parameters<JournalQuery>,
    ) -> String {
        let mut args = json!({});
        if let Some(s) = since {
            args["since"] = json!(s);
        }
        if let Some(u) = until {
            args["until"] = json!(u);
        }
        if let Some(n) = unit {
            args["unit"] = json!(n);
        }
        if let Some(p) = priority {
            args["priority"] = json!(p);
        }
        if let Some(t) = tail {
            args["tail"] = json!(t);
        }
        self.exec("journal.query", args).await
    }

    #[tool(
        name = "system_info",
        description = "System information — hostname, OS, kernel, uptime, memory, CPU.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn system_info(&self) -> String {
        self.call(do_execute(&self.state, "system.info", json!({})))
            .await
    }

    #[tool(
        name = "battery_status",
        description = "Battery percentage and charging state.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn battery_status(&self) -> String {
        self.call(do_execute(&self.state, "system.battery", json!({})))
            .await
    }

    #[tool(
        name = "idle_seconds",
        description = "User idle time in seconds (time since last input).",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn idle_seconds(&self) -> String {
        self.call(do_execute(&self.state, "system.idle", json!({})))
            .await
    }

    #[tool(
        name = "check_update",
        description = "Check the latest GitHub release without installing it.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn check_update(&self) -> String {
        self.call(do_execute(
            &self.state,
            "system.update",
            json!({"check": true}),
        ))
        .await
    }

    #[tool(
        name = "self_update",
        description = "Download the latest GitHub release, replace the deskbrid binary, and restart the user service if active. High-risk action: requires explicit system.update permission.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn self_update(&self) -> String {
        self.call(do_execute(
            &self.state,
            "system.update",
            json!({"force": false}),
        ))
        .await
    }

    #[tool(
        name = "dbus_call",
        description = "Raw D-Bus method call. Escape hatch for direct D-Bus access. Requires explicit dbus.call permission.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn dbus_call(
        &self,
        Parameters(DbusCallArgs {
            bus,
            service,
            path,
            interface,
            method,
            args,
        }): Parameters<DbusCallArgs>,
    ) -> String {
        let mut req = json!({
            "service": service,
            "path": path,
            "interface": interface,
            "method": method,
        });
        if let Some(b) = bus {
            req["bus"] = json!(b);
        }
        if let Some(a) = args {
            req["args"] = a;
        }
        self.exec("dbus.call", req).await
    }

    #[tool(
        name = "list_processes",
        description = "List running processes with PID, name, CPU, and memory.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn list_processes(&self) -> String {
        self.call(do_execute(&self.state, "process.list", json!({})))
            .await
    }

    #[tool(
        name = "start_process",
        description = "Start a new background process. Returns the PID.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn start_process(
        &self,
        Parameters(ProcessStart { command, workdir }): Parameters<ProcessStart>,
    ) -> String {
        let mut args = json!({"command": command});
        if let Some(w) = workdir {
            args["workdir"] = json!(w);
        }
        self.exec("process.start", args).await
    }

    #[tool(
        name = "stop_process",
        description = "Stop a running process by PID.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn stop_process(
        &self,
        Parameters(ProcessSignal { pid, signal }): Parameters<ProcessSignal>,
    ) -> String {
        self.exec("process.stop", json!({"pid": pid, "signal": signal}))
            .await
    }

    #[tool(
        name = "signal_process",
        description = "Send a signal to a running process.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn signal_process(
        &self,
        Parameters(ProcessSignal { pid, signal }): Parameters<ProcessSignal>,
    ) -> String {
        self.exec("process.signal", json!({"pid": pid, "signal": signal}))
            .await
    }

    #[tool(
        name = "process_exists",
        description = "Check if a process with the given PID exists.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn process_exists(
        &self,
        Parameters(ProcessPid { pid }): Parameters<ProcessPid>,
    ) -> String {
        self.exec("process.exists", json!({"pid": pid})).await
    }

    #[tool(
        name = "wait_for_process",
        description = "Wait for a process to exit.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn wait_for_process(
        &self,
        Parameters(ProcessWait { pid, timeout_ms }): Parameters<ProcessWait>,
    ) -> String {
        let mut args = json!({"pid": pid});
        if let Some(t) = timeout_ms {
            args["timeout_ms"] = json!(t);
        }
        self.exec("process.wait", args).await
    }

    #[tool(
        name = "backlight_list",
        description = "List all backlight devices with max and current brightness.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn backlight_list(&self) -> String {
        self.call(do_execute(&self.state, "system.backlight_list", json!({})))
            .await
    }

    #[tool(
        name = "backlight_get",
        description = "Get current brightness of a backlight device (or default).",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn backlight_get(
        &self,
        Parameters(BacklightDevice { device }): Parameters<BacklightDevice>,
    ) -> String {
        let mut args = json!({});
        if let Some(d) = device {
            args["device"] = json!(d);
        }
        self.exec("system.backlight_get", args).await
    }

    #[tool(
        name = "backlight_set",
        description = "Set backlight brightness by percentage ('50%') or raw value ('469').",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn backlight_set(
        &self,
        Parameters(BacklightSetArgs { device, value }): Parameters<BacklightSetArgs>,
    ) -> String {
        let mut args = json!({"value": value});
        if let Some(d) = device {
            args["device"] = json!(d);
        }
        self.exec("system.backlight_set", args).await
    }

    #[tool(
        name = "print_list",
        description = "List all configured printers with name, status, and default.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn print_list(&self) -> String {
        self.call(do_execute(&self.state, "system.print_list", json!({})))
            .await
    }

    #[tool(
        name = "print_default",
        description = "Get or set the default printer. Omit printer to read; provide printer to set.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn print_default(
        &self,
        Parameters(PrintDefaultArgs { printer }): Parameters<PrintDefaultArgs>,
    ) -> String {
        let mut args = json!({});
        if let Some(p) = printer {
            args["printer"] = json!(p);
        }
        self.exec("system.print_default", args).await
    }

    #[tool(
        name = "print_file",
        description = "Send a file to a printer. printer: printer name, path: absolute path to file.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn print_file(
        &self,
        Parameters(PrintFileArgs { printer, path }): Parameters<PrintFileArgs>,
    ) -> String {
        let args = json!({"printer": printer, "path": path});
        self.exec("system.print_file", args).await
    }

    #[tool(
        name = "print_jobs",
        description = "List active print jobs in the queue.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn print_jobs(&self) -> String {
        self.call(do_execute(&self.state, "system.print_jobs", json!({})))
            .await
    }

    #[tool(
        name = "print_job_cancel",
        description = "Cancel a print job by ID.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn print_job_cancel(
        &self,
        Parameters(PrintJobAction { job_id }): Parameters<PrintJobAction>,
    ) -> String {
        self.exec("system.print_job_cancel", json!({"job_id": job_id}))
            .await
    }

    #[tool(
        name = "print_job_pause",
        description = "Pause a print job by ID.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn print_job_pause(
        &self,
        Parameters(PrintJobAction { job_id }): Parameters<PrintJobAction>,
    ) -> String {
        self.exec("system.print_job_pause", json!({"job_id": job_id}))
            .await
    }

    #[tool(
        name = "print_job_resume",
        description = "Resume a paused print job by ID.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn print_job_resume(
        &self,
        Parameters(PrintJobAction { job_id }): Parameters<PrintJobAction>,
    ) -> String {
        self.exec("system.print_job_resume", json!({"job_id": job_id}))
            .await
    }

    #[tool(
        name = "pressure",
        description = "Read Linux Pressure Stall Information (PSI) — CPU, memory, and IO pressure stats. Agents use this to decide whether to proceed, back off, or retry.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn pressure(&self) -> String {
        self.call(do_execute(&self.state, "system.pressure", json!({})))
            .await
    }

    #[tool(
        name = "terminal_create",
        description = "Create a PTY terminal. Returns a terminal_id for subsequent operations.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn terminal_create(
        &self,
        Parameters(TerminalCreate {
            shell,
            cwd,
            rows,
            cols,
        }): Parameters<TerminalCreate>,
    ) -> String {
        let mut args = json!({});
        if let Some(s) = shell {
            args["shell"] = json!(s);
        }
        if let Some(c) = cwd {
            args["cwd"] = json!(c);
        }
        if let Some(r) = rows {
            args["rows"] = json!(r);
        }
        if let Some(c) = cols {
            args["cols"] = json!(c);
        }
        self.exec("terminal.create", args).await
    }

    #[tool(
        name = "terminal_write",
        description = "Send input to a terminal (supports ANSI escape sequences).",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn terminal_write(
        &self,
        Parameters(TerminalWrite { terminal_id, input }): Parameters<TerminalWrite>,
    ) -> String {
        self.exec(
            "terminal.write",
            json!({"terminal_id": terminal_id, "input": input}),
        )
        .await
    }

    #[tool(
        name = "terminal_read",
        description = "Read output from a terminal.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn terminal_read(
        &self,
        Parameters(TerminalRead {
            terminal_id,
            max_bytes,
            flush,
        }): Parameters<TerminalRead>,
    ) -> String {
        let mut args = json!({"terminal_id": terminal_id});
        if let Some(m) = max_bytes {
            args["max_bytes"] = json!(m);
        }
        if flush {
            args["flush"] = json!(true);
        }
        self.exec("terminal.read", args).await
    }

    #[tool(
        name = "terminal_resize",
        description = "Resize a terminal's rows and columns.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn terminal_resize(
        &self,
        Parameters(TerminalResize {
            terminal_id,
            rows,
            cols,
        }): Parameters<TerminalResize>,
    ) -> String {
        self.exec(
            "terminal.resize",
            json!({"terminal_id": terminal_id, "rows": rows, "cols": cols}),
        )
        .await
    }

    #[tool(
        name = "list_windows",
        description = "List all open windows with IDs, titles, classes, workspace, and geometry.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn list_windows(&self) -> String {
        self.call(do_execute(&self.state, "windows.list", json!({})))
            .await
    }

    #[tool(
        name = "focused_window",
        description = "Get the currently focused/active window.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn focused_window(&self) -> String {
        self.call(do_focused_window(&self.state)).await
    }

    #[tool(
        name = "list_workspaces",
        description = "List all virtual desktops/workspaces with current state.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn list_workspaces(&self) -> String {
        self.call(do_execute(&self.state, "workspaces.list", json!({})))
            .await
    }

    #[tool(
        name = "focus_window",
        description = "Focus (activate) a window by its ID.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn focus_window(
        &self,
        Parameters(WindowId { window_id }): Parameters<WindowId>,
    ) -> String {
        self.call(do_focus_window(&self.state, &window_id)).await
    }

    #[tool(
        name = "close_window",
        description = "Close a window by its ID.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn close_window(
        &self,
        Parameters(WindowId { window_id }): Parameters<WindowId>,
    ) -> String {
        self.exec("windows.close", json!({"window_id": window_id}))
            .await
    }

    #[tool(
        name = "minimize_window",
        description = "Minimize a window by its ID.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn minimize_window(
        &self,
        Parameters(WindowId { window_id }): Parameters<WindowId>,
    ) -> String {
        self.exec("windows.minimize", json!({"window_id": window_id}))
            .await
    }

    #[tool(
        name = "maximize_window",
        description = "Maximize a window by its ID.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn maximize_window(
        &self,
        Parameters(WindowId { window_id }): Parameters<WindowId>,
    ) -> String {
        self.exec("windows.maximize", json!({"window_id": window_id}))
            .await
    }

    #[tool(
        name = "move_resize_window",
        description = "Move and/or resize a window.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn move_resize_window(
        &self,
        Parameters(MoveResize {
            window_id,
            x,
            y,
            width,
            height,
        }): Parameters<MoveResize>,
    ) -> String {
        self.exec(
            "windows.move_resize",
            json!({"window_id": window_id, "x": x, "y": y, "width": width, "height": height}),
        )
        .await
    }

    #[tool(
        name = "tile_window",
        description = "Tile a window to a preset position (left, right, maximize, fullscreen).",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn tile_window(
        &self,
        Parameters(TileWindow {
            window_id,
            preset,
            monitor,
            padding,
        }): Parameters<TileWindow>,
    ) -> String {
        let mut args = json!({"window_id": window_id, "preset": preset});
        if let Some(m) = monitor {
            args["monitor"] = json!(m);
        }
        if let Some(p) = padding {
            args["padding"] = json!(p);
        }
        self.exec("windows.tile", args).await
    }

    #[tool(
        name = "activate_or_launch",
        description = "Focus an existing app window or launch it if not running.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn activate_or_launch(
        &self,
        Parameters(ActivateOrLaunch {
            app_id,
            command,
            workdir,
        }): Parameters<ActivateOrLaunch>,
    ) -> String {
        let mut args = json!({"app_id": app_id, "command": command});
        if let Some(wd) = workdir {
            args["workdir"] = json!(wd);
        }
        self.exec("windows.activate_or_launch", args).await
    }

    #[tool(
        name = "switch_workspace",
        description = "Switch to a specific workspace by index.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn switch_workspace(
        &self,
        Parameters(SwitchWorkspace { workspace_id }): Parameters<SwitchWorkspace>,
    ) -> String {
        self.exec("workspaces.switch", json!({"workspace_id": workspace_id}))
            .await
    }

    #[tool(
        name = "move_window_to_workspace",
        description = "Move a window to another workspace, optionally following it.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn move_window_to_workspace(
        &self,
        Parameters(MoveWindowToWorkspace {
            window_id,
            workspace_id,
            follow,
        }): Parameters<MoveWindowToWorkspace>,
    ) -> String {
        self.exec(
            "workspaces.move_window",
            json!({"window_id": window_id, "workspace_id": workspace_id, "follow": follow}),
        )
        .await
    }
}

/// Run the MCP server over stdio transport (for `deskbrid mcp`).
pub async fn run_mcp(state: Arc<DaemonState>) -> anyhow::Result<()> {
    use rmcp::{service::serve_server, transport::stdio};
    serve_server(McpServer::new(state), stdio())
        .await?
        .waiting()
        .await?;
    Ok(())
}

/// Run the MCP server over TCP transport.
pub async fn run_mcp_tcp(state: Arc<DaemonState>, port: u16, token: String) -> anyhow::Result<()> {
    use crate::daemon::tcp::{constant_time_eq, read_limited_line};
    use rmcp::service::serve_server;
    use tokio::net::TcpListener;
    const MAX_AUTH_LINE: u64 = 4096;
    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("Deskbrid MCP (rmcp) TCP server listening on {addr} (token auth)");
    loop {
        let (stream, peer) = listener.accept().await?;
        let state = state.clone();
        let token = token.clone();
        tokio::spawn(async move {
            let (mut reader, writer) = tokio::io::split(stream);
            let auth_line = match read_limited_line(&mut reader, MAX_AUTH_LINE as usize).await {
                Ok(line) => line,
                Err(e) => {
                    tracing::error!("MCP auth read error from {peer}: {e}");
                    return;
                }
            };
            if auth_line.len() > MAX_AUTH_LINE as usize {
                tracing::warn!("MCP auth message too large from {peer} — rejecting");
                return;
            }
            if auth_line.trim().is_empty() {
                tracing::warn!("MCP client {peer} sent empty auth");
                return;
            }
            let auth: serde_json::Value = match serde_json::from_str(&auth_line) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("MCP client {peer} sent invalid JSON auth: {e}");
                    return;
                }
            };
            if auth.get("type") != Some(&serde_json::Value::String("auth".into())) {
                tracing::warn!("MCP client {peer} sent non-auth first message");
                return;
            }
            let provided = auth.get("token").and_then(|v| v.as_str()).unwrap_or("");
            if !constant_time_eq(provided, &token) {
                tracing::warn!("MCP client {peer} sent invalid token");
                return;
            }
            let stream = reader.unsplit(writer);
            let server = McpServer::new(state);
            if let Err(e) = serve_server(server, stream).await {
                tracing::error!("MCP connection error from {peer}: {e}");
            }
        });
    }
}
