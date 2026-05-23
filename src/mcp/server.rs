//! MCP server — rmcp-based stdio server for `deskbrid mcp`.
//!
//! Each tool is a thin `#[tool]` wrapper that delegates to async helpers.
//! Safety annotations follow MCP_INTEGRATION.md Part 5.
//! Parameter types live in `types.rs` to keep this file under 250 lines.

use super::helpers::*;
use super::types::*;
use crate::DaemonState;
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

// ── Tool implementations ─────────────────────────────────────
// Thin wrappers: each tool parses params, delegates to helpers, returns Json.

#[tool_router(server_handler)]
impl McpServer {
    // ── Discovery ──────────────────────────────────────────

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
        description = "Take a screenshot of the desktop. Returns base64-encoded PNG.",
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

    // ── Window control ─────────────────────────────────────

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
        block(
            &self.rt,
            do_execute(
                &self.state,
                "windows.close",
                json!({"window_id": window_id}),
            ),
        )
    }

    // ── Input ──────────────────────────────────────────────

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

    // ── Clipboard ──────────────────────────────────────────

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

    // ── AT-SPI ─────────────────────────────────────────────

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

    // ── Diagnostics ────────────────────────────────────────

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

    // ── System ─────────────────────────────────────────────

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
}

/// Run the MCP server over stdio transport (for `deskbrid mcp`).
pub async fn run_mcp(state: Arc<DaemonState>) -> anyhow::Result<()> {
    use rmcp::{service::serve_server, transport::stdio};

    let server = McpServer::new(state);
    serve_server(server, stdio()).await?.waiting().await?;
    Ok(())
}
