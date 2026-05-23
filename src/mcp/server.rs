//! MCP server — rmcp-based stdio server for `deskbrid mcp`.
//!
//! Each tool is a thin `#[tool]` wrapper delegating to async helpers.
//! Safety annotations follow MCP_INTEGRATION.md Part 5.
//! Parameter types live in `types.rs`.

use super::helpers::*;
use super::types::*;
use crate::DaemonState;
use anyhow::Context;
use rmcp::{
    handler::server::wrapper::{Json, Parameters},
    tool, tool_router,
};
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
}

fn block<F: std::future::Future<Output = anyhow::Result<Value>>>(rt: &Handle, f: F) -> Json<Value> {
    Json(rt.block_on(async { f.await.unwrap_or_else(|e| json!({"error": e.to_string()})) }))
}

fn execute(state: Arc<DaemonState>, rt: &Handle, action: &str, args: Value) -> Json<Value> {
    let action = action.to_string();
    let rt = rt.clone();
    Json(rt.block_on(async {
        do_execute_with(&state, &action, args)
            .await
            .unwrap_or_else(|e| json!({"error": e.to_string()}))
    }))
}

// ── Tool implementations ─────────────────────────────────────

#[tool_router(server_handler)]
impl McpServer {
    // ═══════════════════════════════════════════════════════════
    //  DISCOVERY
    // ═══════════════════════════════════════════════════════════

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
    fn list_windows(&self) -> Json<Value> {
        block(&self.rt, do_execute(&self.state, "windows.list", json!({})))
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
    fn focused_window(&self) -> Json<Value> {
        block(&self.rt, do_focused_window(&self.state))
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
    fn list_workspaces(&self) -> Json<Value> {
        block(
            &self.rt,
            do_execute(&self.state, "workspaces.list", json!({})),
        )
    }

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
    fn list_apps(&self) -> Json<Value> {
        block(&self.rt, do_list_apps(&self.state))
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
    fn get_accessibility_tree(
        &self,
        Parameters(A11yTree {
            app_name,
            pid,
            max_nodes,
            max_depth,
        }): Parameters<A11yTree>,
    ) -> Json<Value> {
        block(
            &self.rt,
            do_get_accessibility_tree(&self.state, app_name.as_deref(), pid, max_nodes, max_depth),
        )
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
    fn screenshot(&self) -> Json<Value> {
        block(&self.rt, do_execute(&self.state, "screenshot", json!({})))
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
    fn screenshot_region(
        &self,
        Parameters(ScreenshotOptions {
            monitor,
            window_id,
            region_x,
            region_y,
            region_w,
            region_h,
        }): Parameters<ScreenshotOptions>,
    ) -> Json<Value> {
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
        execute(self.state.clone(), &self.rt, "screenshot", args)
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
    fn screenshot_diff(
        &self,
        Parameters(ScreenshotDiff {
            before_path,
            after_path,
            tolerance,
            diff_path,
            monitor,
        }): Parameters<ScreenshotDiff>,
    ) -> Json<Value> {
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
        execute(self.state.clone(), &self.rt, "screenshot.diff", args)
    }

    // ═══════════════════════════════════════════════════════════
    //  WINDOW CONTROL
    // ═══════════════════════════════════════════════════════════

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
    fn focus_window(
        &self,
        Parameters(WindowId { window_id }): Parameters<WindowId>,
    ) -> Json<Value> {
        block(&self.rt, do_focus_window(&self.state, &window_id))
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
    fn close_window(
        &self,
        Parameters(WindowId { window_id }): Parameters<WindowId>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "windows.close",
            json!({"window_id": window_id}),
        )
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
    fn minimize_window(
        &self,
        Parameters(WindowId { window_id }): Parameters<WindowId>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "windows.minimize",
            json!({"window_id": window_id}),
        )
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
    fn maximize_window(
        &self,
        Parameters(WindowId { window_id }): Parameters<WindowId>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "windows.maximize",
            json!({"window_id": window_id}),
        )
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
    fn move_resize_window(
        &self,
        Parameters(MoveResize {
            window_id,
            x,
            y,
            width,
            height,
        }): Parameters<MoveResize>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "windows.move_resize",
            json!({"window_id": window_id, "x": x, "y": y, "width": width, "height": height}),
        )
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
    fn tile_window(
        &self,
        Parameters(TileWindow {
            window_id,
            preset,
            monitor,
            padding,
        }): Parameters<TileWindow>,
    ) -> Json<Value> {
        let mut args = json!({"window_id": window_id, "preset": preset});
        if let Some(m) = monitor {
            args["monitor"] = json!(m);
        }
        if let Some(p) = padding {
            args["padding"] = json!(p);
        }
        execute(self.state.clone(), &self.rt, "windows.tile", args)
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
    fn activate_or_launch(
        &self,
        Parameters(ActivateOrLaunch {
            app_id,
            command,
            workdir,
        }): Parameters<ActivateOrLaunch>,
    ) -> Json<Value> {
        let mut args = json!({"app_id": app_id, "command": command});
        if let Some(wd) = workdir {
            args["workdir"] = json!(wd);
        }
        execute(
            self.state.clone(),
            &self.rt,
            "windows.activate_or_launch",
            args,
        )
    }

    // ═══════════════════════════════════════════════════════════
    //  WORKSPACES
    // ═══════════════════════════════════════════════════════════

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
    fn switch_workspace(
        &self,
        Parameters(SwitchWorkspace { workspace_id }): Parameters<SwitchWorkspace>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "workspaces.switch",
            json!({"workspace_id": workspace_id}),
        )
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
    fn move_window_to_workspace(
        &self,
        Parameters(MoveWindowToWorkspace {
            window_id,
            workspace_id,
            follow,
        }): Parameters<MoveWindowToWorkspace>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "workspaces.move_window",
            json!({"window_id": window_id, "workspace_id": workspace_id, "follow": follow}),
        )
    }

    // ═══════════════════════════════════════════════════════════
    //  INPUT
    // ═══════════════════════════════════════════════════════════

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
    fn type_text(&self, Parameters(TypeText { text }): Parameters<TypeText>) -> Json<Value> {
        block(&self.rt, do_type_text(&self.state, &text))
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
    fn press_key(&self, Parameters(PressKey { key }): Parameters<PressKey>) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "input.keyboard",
            json!({"key": key}),
        )
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
    fn press_keys(&self, Parameters(PressKeys { keys }): Parameters<PressKeys>) -> Json<Value> {
        block(&self.rt, do_press_keys(&self.state, &keys))
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
    fn mouse_move(&self, Parameters(MouseMove { x, y }): Parameters<MouseMove>) -> Json<Value> {
        block(&self.rt, do_mouse_move(&self.state, x, y))
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
    fn mouse_click(
        &self,
        Parameters(MouseClick { button }): Parameters<MouseClick>,
    ) -> Json<Value> {
        block(&self.rt, do_mouse_click(&self.state, &button))
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
    fn mouse_scroll(
        &self,
        Parameters(MouseScroll { dx, dy }): Parameters<MouseScroll>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "input.mouse",
            json!({"action": "scroll", "dx": dx, "dy": dy}),
        )
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
    fn click_coordinate(
        &self,
        Parameters(ClickCoord { x, y, button }): Parameters<ClickCoord>,
    ) -> Json<Value> {
        block(&self.rt, do_click_coordinate(x, y, &button))
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
    fn drag(
        &self,
        Parameters(Drag {
            from_x,
            from_y,
            to_x,
            to_y,
            button,
        }): Parameters<Drag>,
    ) -> Json<Value> {
        block(&self.rt, do_drag(from_x, from_y, to_x, to_y, &button))
    }

    // ═══════════════════════════════════════════════════════════
    //  CLIPBOARD
    // ═══════════════════════════════════════════════════════════

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
    fn clipboard_read(&self) -> Json<Value> {
        block(
            &self.rt,
            do_execute(&self.state, "clipboard.read", json!({})),
        )
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
    fn clipboard_write(
        &self,
        Parameters(ClipboardWrite { text }): Parameters<ClipboardWrite>,
    ) -> Json<Value> {
        block(&self.rt, do_clipboard_write(&self.state, &text))
    }

    // ═══════════════════════════════════════════════════════════
    //  AT-SPI
    // ═══════════════════════════════════════════════════════════

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
    fn perform_action(
        &self,
        Parameters(A11yAction {
            object_ref,
            action_name,
        }): Parameters<A11yAction>,
    ) -> Json<Value> {
        block(
            &self.rt,
            do_perform_action(&self.state, &object_ref, action_name.as_deref()),
        )
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
    fn set_element_value(
        &self,
        Parameters(SetValue { object_ref, value }): Parameters<SetValue>,
    ) -> Json<Value> {
        block(
            &self.rt,
            do_set_element_value(&self.state, &object_ref, &value),
        )
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
    fn get_element_text(
        &self,
        Parameters(GetText {
            object_ref,
            max_chars,
        }): Parameters<GetText>,
    ) -> Json<Value> {
        block(
            &self.rt,
            do_get_element_text(&self.state, &object_ref, max_chars),
        )
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
    fn click_element(
        &self,
        Parameters(ClickElement { object_ref }): Parameters<ClickElement>,
    ) -> Json<Value> {
        block(&self.rt, do_click_element(&self.state, &object_ref))
    }

    // ═══════════════════════════════════════════════════════════
    //  DIAGNOSTICS
    // ═══════════════════════════════════════════════════════════

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
    fn doctor(&self) -> Json<Value> {
        block(&self.rt, do_doctor(&self.state))
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
    fn setup_accessibility(&self) -> Json<Value> {
        block(&self.rt, do_setup_accessibility(&self.state))
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
    fn capabilities(&self) -> Json<Value> {
        block(&self.rt, do_capabilities(&self.state))
    }

    // ═══════════════════════════════════════════════════════════
    //  SYSTEM
    // ═══════════════════════════════════════════════════════════

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
    fn system_info(&self) -> Json<Value> {
        block(&self.rt, do_execute(&self.state, "system.info", json!({})))
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
    fn battery_status(&self) -> Json<Value> {
        block(
            &self.rt,
            do_execute(&self.state, "system.battery", json!({})),
        )
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
    fn idle_seconds(&self) -> Json<Value> {
        block(&self.rt, do_execute(&self.state, "system.idle", json!({})))
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
    fn network_status(&self) -> Json<Value> {
        block(
            &self.rt,
            do_execute(&self.state, "network.status", json!({})),
        )
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
    fn bluetooth_list(&self) -> Json<Value> {
        block(
            &self.rt,
            do_execute(&self.state, "bluetooth.list", json!({})),
        )
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
    fn bluetooth_scan(
        &self,
        Parameters(BluetoothScan { duration }): Parameters<BluetoothScan>,
    ) -> Json<Value> {
        let mut args = json!({});
        if let Some(d) = duration {
            args["duration"] = json!(d);
        }
        execute(self.state.clone(), &self.rt, "bluetooth.scan", args)
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
    fn service_status(
        &self,
        Parameters(ServiceName { name }): Parameters<ServiceName>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "service.status",
            json!({"name": name}),
        )
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
    fn service_start(
        &self,
        Parameters(ServiceName { name }): Parameters<ServiceName>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "service.start",
            json!({"name": name}),
        )
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
    fn service_stop(
        &self,
        Parameters(ServiceName { name }): Parameters<ServiceName>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "service.stop",
            json!({"name": name}),
        )
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
    fn journal_query(
        &self,
        Parameters(JournalQuery {
            since,
            until,
            unit,
            priority,
            tail,
        }): Parameters<JournalQuery>,
    ) -> Json<Value> {
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
        execute(self.state.clone(), &self.rt, "journal.query", args)
    }

    // ═══════════════════════════════════════════════════════════
    //  AUDIO
    // ═══════════════════════════════════════════════════════════

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
    fn list_audio_sinks(&self) -> Json<Value> {
        block(
            &self.rt,
            do_execute(&self.state, "audio.list_sinks", json!({})),
        )
    }

    #[tool(
        name = "set_volume",
        description = "Set audio sink volume.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    fn set_volume(
        &self,
        Parameters(SetVolume { sink_id, volume }): Parameters<SetVolume>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "audio.set_sink_volume",
            json!({"sink_id": sink_id, "volume": volume}),
        )
    }

    // ═══════════════════════════════════════════════════════════
    //  FILE OPERATIONS
    // ═══════════════════════════════════════════════════════════

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
    fn file_list(&self, Parameters(FilePath { path }): Parameters<FilePath>) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "files.list",
            json!({"path": path}),
        )
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
    fn file_read(
        &self,
        Parameters(FileRead {
            path,
            offset,
            limit,
        }): Parameters<FileRead>,
    ) -> Json<Value> {
        let mut args = json!({"path": path});
        if let Some(o) = offset {
            args["offset"] = json!(o);
        }
        if let Some(l) = limit {
            args["limit"] = json!(l);
        }
        execute(self.state.clone(), &self.rt, "files.read", args)
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
    fn file_write(
        &self,
        Parameters(FileWrite {
            path,
            content,
            append,
        }): Parameters<FileWrite>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "files.write",
            json!({"path": path, "content": content, "append": append}),
        )
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
    fn file_search(
        &self,
        Parameters(FileSearch {
            pattern,
            root,
            max_results,
        }): Parameters<FileSearch>,
    ) -> Json<Value> {
        let mut args = json!({"pattern": pattern, "max_results": max_results});
        if let Some(r) = root {
            args["root"] = json!(r);
        }
        execute(self.state.clone(), &self.rt, "files.search", args)
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
    fn file_copy(
        &self,
        Parameters(FileCopy {
            source,
            destination,
        }): Parameters<FileCopy>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "files.copy",
            json!({"source": source, "destination": destination}),
        )
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
    fn file_watch(
        &self,
        Parameters(FileWatch {
            path,
            recursive,
            patterns,
        }): Parameters<FileWatch>,
    ) -> Json<Value> {
        let mut args = json!({"path": path, "recursive": recursive});
        if let Some(p) = patterns {
            args["patterns"] = json!(p);
        }
        execute(self.state.clone(), &self.rt, "files.watch", args)
    }

    // ═══════════════════════════════════════════════════════════
    //  TERMINAL
    // ═══════════════════════════════════════════════════════════

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
    fn terminal_create(
        &self,
        Parameters(TerminalCreate {
            shell,
            cwd,
            rows,
            cols,
        }): Parameters<TerminalCreate>,
    ) -> Json<Value> {
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
        execute(self.state.clone(), &self.rt, "terminal.create", args)
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
    fn terminal_write(
        &self,
        Parameters(TerminalWrite { terminal_id, input }): Parameters<TerminalWrite>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "terminal.write",
            json!({"terminal_id": terminal_id, "input": input}),
        )
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
    fn terminal_read(
        &self,
        Parameters(TerminalRead {
            terminal_id,
            max_bytes,
            flush,
        }): Parameters<TerminalRead>,
    ) -> Json<Value> {
        let mut args = json!({"terminal_id": terminal_id});
        if let Some(m) = max_bytes {
            args["max_bytes"] = json!(m);
        }
        if flush {
            args["flush"] = json!(true);
        }
        execute(self.state.clone(), &self.rt, "terminal.read", args)
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
    fn terminal_resize(
        &self,
        Parameters(TerminalResize {
            terminal_id,
            rows,
            cols,
        }): Parameters<TerminalResize>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "terminal.resize",
            json!({"terminal_id": terminal_id, "rows": rows, "cols": cols}),
        )
    }

    // ═══════════════════════════════════════════════════════════
    //  LAYOUT PROFILES
    // ═══════════════════════════════════════════════════════════

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
    fn layout_list(&self) -> Json<Value> {
        block(
            &self.rt,
            do_execute(&self.state, "layout_profiles.list", json!({})),
        )
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
    fn layout_save(
        &self,
        Parameters(LayoutSave { name, overwrite }): Parameters<LayoutSave>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "layout_profiles.save",
            json!({"name": name, "overwrite": overwrite}),
        )
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
    fn layout_restore(
        &self,
        Parameters(LayoutName { name }): Parameters<LayoutName>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "layout_profiles.restore",
            json!({"name": name}),
        )
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
    fn layout_delete(
        &self,
        Parameters(LayoutName { name }): Parameters<LayoutName>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "layout_profiles.delete",
            json!({"name": name}),
        )
    }

    // ═══════════════════════════════════════════════════════════
    //  MONITOR
    // ═══════════════════════════════════════════════════════════

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
    fn list_monitors(&self) -> Json<Value> {
        block(&self.rt, do_execute(&self.state, "monitor.list", json!({})))
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
    fn set_primary_monitor(
        &self,
        Parameters(MonitorOutput { output }): Parameters<MonitorOutput>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "monitor.set_primary",
            json!({"output": output}),
        )
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
    fn set_monitor_resolution(
        &self,
        Parameters(SetResolution {
            output,
            width,
            height,
            refresh_rate,
        }): Parameters<SetResolution>,
    ) -> Json<Value> {
        let mut args = json!({"output": output, "width": width, "height": height});
        if let Some(r) = refresh_rate {
            args["refresh_rate"] = json!(r);
        }
        execute(self.state.clone(), &self.rt, "monitor.set_resolution", args)
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
    fn set_monitor_scale(
        &self,
        Parameters(SetScale { output, scale }): Parameters<SetScale>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "monitor.set_scale",
            json!({"output": output, "scale": scale}),
        )
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
    fn set_monitor_rotation(
        &self,
        Parameters(SetRotation { output, rotation }): Parameters<SetRotation>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "monitor.set_rotation",
            json!({"output": output, "rotation": rotation}),
        )
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
    fn enable_monitor(
        &self,
        Parameters(MonitorOutput { output }): Parameters<MonitorOutput>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "monitor.enable",
            json!({"output": output}),
        )
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
    fn disable_monitor(
        &self,
        Parameters(MonitorOutput { output }): Parameters<MonitorOutput>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "monitor.disable",
            json!({"output": output}),
        )
    }

    // ═══════════════════════════════════════════════════════════
    //  BROWSER (Chrome DevTools Protocol)
    // ═══════════════════════════════════════════════════════════

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
    fn list_browser_tabs(&self) -> Json<Value> {
        block(
            &self.rt,
            do_execute(&self.state, "browser.list_tabs", json!({})),
        )
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
    fn browser_navigate(
        &self,
        Parameters(BrowserNavigate { tab_index, url }): Parameters<BrowserNavigate>,
    ) -> Json<Value> {
        let mut args = json!({"url": url});
        if let Some(t) = tab_index {
            args["tab_index"] = json!(t);
        }
        execute(self.state.clone(), &self.rt, "browser.navigate", args)
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
    fn browser_evaluate(
        &self,
        Parameters(BrowserEvaluate {
            tab_index,
            expression,
            await_promise,
        }): Parameters<BrowserEvaluate>,
    ) -> Json<Value> {
        let mut args = json!({"expression": expression, "await_promise": await_promise});
        if let Some(t) = tab_index {
            args["tab_index"] = json!(t);
        }
        execute(self.state.clone(), &self.rt, "browser.evaluate", args)
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
    fn browser_screenshot(
        &self,
        Parameters(TabIndex { tab_index }): Parameters<TabIndex>,
    ) -> Json<Value> {
        let mut args = json!({});
        if let Some(t) = tab_index {
            args["tab_index"] = json!(t);
        }
        execute(self.state.clone(), &self.rt, "browser.screenshot_tab", args)
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
    fn browser_click(
        &self,
        Parameters(BrowserClick {
            tab_index,
            selector,
        }): Parameters<BrowserClick>,
    ) -> Json<Value> {
        let mut args = json!({"selector": selector});
        if let Some(t) = tab_index {
            args["tab_index"] = json!(t);
        }
        execute(self.state.clone(), &self.rt, "browser.click", args)
    }

    // ═══════════════════════════════════════════════════════════
    //  MPRIS (Media Player Remote Interfacing Specification)
    // ═══════════════════════════════════════════════════════════

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
    fn list_media_players(&self) -> Json<Value> {
        block(&self.rt, do_execute(&self.state, "mpris.list", json!({})))
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
    fn media_player_info(
        &self,
        Parameters(MprisPlayer { player }): Parameters<MprisPlayer>,
    ) -> Json<Value> {
        let mut args = json!({});
        if let Some(p) = player {
            args["player"] = json!(p);
        }
        execute(self.state.clone(), &self.rt, "mpris.get", args)
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
    fn media_player_control(
        &self,
        Parameters(MprisControl { player, action }): Parameters<MprisControl>,
    ) -> Json<Value> {
        let mut args = json!({"action": action});
        if let Some(p) = player {
            args["player"] = json!(p);
        }
        execute(self.state.clone(), &self.rt, "mpris.control", args)
    }

    // ═══════════════════════════════════════════════════════════
    //  PROCESS
    // ═══════════════════════════════════════════════════════════

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
    fn list_processes(&self) -> Json<Value> {
        block(&self.rt, do_execute(&self.state, "process.list", json!({})))
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
    fn start_process(
        &self,
        Parameters(ProcessStart { command, workdir }): Parameters<ProcessStart>,
    ) -> Json<Value> {
        let mut args = json!({"command": command});
        if let Some(w) = workdir {
            args["workdir"] = json!(w);
        }
        execute(self.state.clone(), &self.rt, "process.start", args)
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
    fn stop_process(
        &self,
        Parameters(ProcessSignal { pid, signal }): Parameters<ProcessSignal>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "process.stop",
            json!({"pid": pid, "signal": signal}),
        )
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
    fn signal_process(
        &self,
        Parameters(ProcessSignal { pid, signal }): Parameters<ProcessSignal>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "process.signal",
            json!({"pid": pid, "signal": signal}),
        )
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
    fn process_exists(
        &self,
        Parameters(ProcessPid { pid }): Parameters<ProcessPid>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "process.exists",
            json!({"pid": pid}),
        )
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
    fn wait_for_process(
        &self,
        Parameters(ProcessWait { pid, timeout_ms }): Parameters<ProcessWait>,
    ) -> Json<Value> {
        let mut args = json!({"pid": pid});
        if let Some(t) = timeout_ms {
            args["timeout_ms"] = json!(t);
        }
        execute(self.state.clone(), &self.rt, "process.wait", args)
    }

    // ═══════════════════════════════════════════════════════════
    //  NOTIFICATIONS
    // ═══════════════════════════════════════════════════════════

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
    fn send_notification(
        &self,
        Parameters(NotificationSend {
            app_name,
            title,
            body,
            urgency,
        }): Parameters<NotificationSend>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "notification.send",
            json!({"app_name": app_name, "title": title, "body": body, "urgency": urgency}),
        )
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
    fn close_notification(
        &self,
        Parameters(NotificationClose { notification_id }): Parameters<NotificationClose>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "notification.close",
            json!({"notification_id": notification_id}),
        )
    }

    // ═══════════════════════════════════════════════════════════
    //  HOTKEYS
    // ═══════════════════════════════════════════════════════════

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
    fn register_hotkey(
        &self,
        Parameters(HotkeyRegister { hotkey_id, keys }): Parameters<HotkeyRegister>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "hotkeys.register",
            json!({"hotkey_id": hotkey_id, "keys": keys}),
        )
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
    fn unregister_hotkey(
        &self,
        Parameters(HotkeyUnregister { hotkey_id }): Parameters<HotkeyUnregister>,
    ) -> Json<Value> {
        execute(
            self.state.clone(),
            &self.rt,
            "hotkeys.unregister",
            json!({"hotkey_id": hotkey_id}),
        )
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

/// Run the MCP server over TCP transport (for `deskbrid daemon --mcp-port`).
/// Self-contained: creates its own daemon state and backend.
pub async fn run_mcp_tcp_on_port(port: u16) -> anyhow::Result<()> {
    let event_tx = tokio::sync::broadcast::channel(256).0;
    let state = Arc::new(crate::DaemonState::new());
    let backend = crate::backend::create_backend(event_tx)
        .await
        .context("no desktop backend detected")?;
    *state.backend.write().await = Some(backend);
    run_mcp_tcp(state, port).await
}

/// Run the MCP server over TCP transport (for `deskbrid daemon --mcp-port`).
/// Uses rmcp's stream transport — same tool surface as stdio mode.
pub async fn run_mcp_tcp(state: Arc<DaemonState>, port: u16) -> anyhow::Result<()> {
    use rmcp::service::serve_server;
    use tokio::net::TcpListener;

    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("Deskbrid MCP (rmcp) TCP server listening on {addr}");

    loop {
        let (stream, peer) = listener.accept().await?;
        let state = state.clone();
        tokio::spawn(async move {
            let server = McpServer::new(state);
            if let Err(e) = serve_server(server, stream).await {
                tracing::error!("MCP connection error from {peer}: {e}");
            }
        });
    }
}
