# Deskbrid MCP Integration — Following the computer-use-linux Pattern

**Goal:** Deskbrid talks to Hermes (and any other MCP host) the exact same way `computer-use-linux` does — as a stdio MCP server built on `rmcp`. Users swap one config entry to switch, and Deskbrid brings 4× the tools plus Proxmox.

---

## Part 1: How computer-use-linux Does It

`computer-use-linux` v0.2.3 is the reference implementation for how a Linux desktop control
tool ships as an MCP server. Deskbrid can clone its entire MCP surface — then exceed it.

### Architecture

```
MCP Host (Hermes/Claude Desktop/Codex)
  └── stdio transport ──→ computer-use-linux mcp (rmcp ServerHandler)
                            ├── atspi crate → AT-SPI registry → accessibility tree
                            ├── zbus → XDG Desktop Portal → screenshots
                            ├── ydotoold socket → /dev/uinput → keyboard/mouse
                            └── Window Registry → GNOME/KWin/Hyprland/i3/COSMIC → window ops
```

### Tools (15 total)

| Category | Tools | Count |
|----------|-------|-------|
| Diagnostics | `doctor`, `setup_accessibility`, `setup_window_targeting` | 3 |
| Discovery | `list_apps`, `list_windows`, `focused_window`, `get_app_state` | 4 |
| Input | `click`, `drag`, `scroll`, `press_key`, `type_text` | 5 |
| Semantic | `perform_action`, `set_value` | 2 |
| Navigation | `activate_window` | 1 |

### MCP Safety Contract

Every tool declares its safety profile to the MCP host:

| Class | Example | Annotations |
|-------|---------|-------------|
| Read-only observation | `list_windows`, `get_app_state` | `readOnlyHint=true` |
| Local setup mutators | `setup_accessibility` | `readOnlyHint=false`, `idempotentHint=true` |
| UI state mutators | `activate_window`, `scroll` | `readOnlyHint=false` |
| Desktop action mutators | `click`, `type_text`, `press_key` | `readOnlyHint=false`, `destructiveHint=true` |

### Code Pattern

Every tool is a function annotated with `#[tool]` macro from `rmcp`:

```rust
use rmcp::{
    tool, tool_handler,
    model::ToolCallResult,
    handler::server::wrapper::Json,
};

#[tool(
    name = "list_windows",
    description = "List all open windows with metadata.",
    annotations(
        read_only_hint = true,
        destructive_hint = false,
        idempotent_hint = true,
        open_world_hint = true
    )
)]
async fn list_windows(
    &self,
) -> ToolCallResult<Json<Vec<WindowInfo>>> {
    let windows = self.backend.list_windows().await?;
    Ok(Json(windows))
}
```

### Server Startup

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let backend = DesktopBackend::detect().await?;
    let server = ComputerUseServer::new(backend);

    // rmcp handles stdio framing, JSON-RPC, tool discovery, and lifecycle
    server
        .serve(rmcp::transport::stdio())
        .await?
        .waiting()
        .await?;

    Ok(())
}
```

### Hermes Config — Exactly what deskbrid would use

```json
{
  "mcp_servers": {
    "computer-use-linux": {
      "command": "computer-use-linux",
      "args": ["mcp"],
      "timeout": 120,
      "connect_timeout": 30
    }
  }
}
```

---

## Part 2: Deskbrid's MCP Surface — 4× Bigger

Deskbrid already exposes 90+ protocol actions. Mapped to MCP tools, that's 50+ tools
across 12 categories — quadruple what computer-use-linux offers.

### Complete Tool Map

Same format as computer-use-linux, but with every category Deskbrid covers:

#### Diagnostics (3 tools)
```
doctor                    — Full dependency health report
setup_accessibility       — Enable GNOME AT-SPI
setup_desktop             — Enable all desktop integration deps
```

#### Discovery (6 tools)
```
list_windows              — All open windows with geometry, class, workspace
focused_window            — Currently active window
list_workspaces           — Virtual desktops with current state
list_apps                 — AT-SPI application roots
get_accessibility_tree    — Full UI tree snapshot with bounds, roles, states, actions, text
get_app_state             — Combined screenshot + tree for a window
```

#### Input (7 tools)
```
type_text                 — Keyboard text input
press_key                 — Single key
press_keys                — Key combination
mouse_move                — Absolute cursor position
mouse_click               — Button click
mouse_scroll              — Scroll wheel
click_coordinate          — Move + click in one call
drag                      — Click-and-drag between coordinates
```

#### Window Control (9 tools)
```
focus_window              — Activate by ID
close_window              — Close by ID
minimize_window           — Minimize by ID
maximize_window           — Maximize by ID
move_resize_window        — Position and size
tile_window               — Snap to preset (left/right/max/fullscreen)
switch_workspace          — Jump to workspace N
move_window_to_workspace  — Send window to another workspace
activate_or_launch        — Focus existing window or launch app
```

#### Screenshots (3 tools)
```
screenshot                — Full desktop capture (PNG, base64)
screenshot_region         — Region capture
screenshot_diff           — Pixel diff between two screenshots
```

#### Clipboard (2 tools)
```
clipboard_read            — Get clipboard text
clipboard_write           — Set clipboard text
```

#### Audio (2 tools)
```
list_audio_sinks          — Output devices with volume/mute
set_volume                — Set sink volume
```

#### System (10 tools)
```
system_info               — OS, kernel, hostname, uptime, memory, CPU
battery_status            — Battery percentage, charging state
idle_seconds              — User idle time
network_status            — Network interfaces, IPs, connectivity
bluetooth_list            — Paired devices
bluetooth_scan            — Discover nearby devices
service_status            — systemd service state
service_start             — Start a service
service_stop              — Stop a service
journal_query             — Query system journal
```

#### File Operations (6 tools)
```
file_list                 — Directory listing
file_read                 — Read file contents
file_write                — Write file contents
file_search               — Search filesystem
file_watch                — Subscribe to file changes
file_copy                 — Copy file/directory
```

#### Terminal (4 tools)
```
terminal_create           — Spawn PTY
terminal_write            — Send keys to terminal
terminal_read             — Read terminal output
terminal_resize           — Resize PTY
```

#### Layout Profiles (4 tools)
```
layout_save               — Save current window layout
layout_restore            — Restore a saved layout
layout_list               — List saved profiles
layout_delete             — Delete a profile
```

#### Proxmox (13+ tools — future)
```
proxmox_status             — Full cluster inventory
proxmox_nodes              — Node listing
proxmox_vms                — QEMU VMs
proxmox_containers         — LXC containers
proxmox_guest_status       — Running state of a guest
proxmox_guest_config       — Guest configuration
proxmox_guest_action       — Start/stop/reboot/suspend
proxmox_guest_exec         — Run command inside container (★)
proxmox_storage_list       — Storage pool overview
proxmox_snapshot_list      — Snapshot listing
proxmox_version            — PVE version info
proxmox_task_wait          — Wait for async task completion
```

---

## Part 3: Implementation — Building deskbrid mcp

### Decision: Use rmcp Directly

`computer-use-linux` uses `rmcp` with `#[tool]` macros. Deskbrid should use the
same approach but wire tools to the existing `dispatch_action` system rather than
reimplementing tool logic.

```
MCP Host (Hermes/Claude Desktop/Codex)
  └── stdio transport ──→ deskbrid mcp (rmcp ServerHandler)
                            └── dispatch_action(Action::WindowsList)
                                  └── existing daemon code
                                        └── DesktopBackend trait
                                              ├── GNOME (D-Bus + Portal)
                                              ├── KDE (KWin scripting)
                                              ├── Hyprland (hyprctl IPC)
                                              ├── COSMIC (cosmic-helper)
                                              └── X11 (xdotool/wmctrl)
```

### Cargo.toml

```toml
rmcp = { version = "1.5", features = ["server", "transport-io", "macros"] }
schemars = "1"  # Required by rmcp for tool schema generation
```

### File Structure

```
src/
├── mcp/
│   ├── mod.rs           # DeskbridMcpServer struct, #[tool_router], serve()
│   ├── tools.rs         # 50+ #[tool] functions — one per MCP tool
│   ├── types.rs         # Input/output types with JsonSchema derives
│   └── safety.rs        # Safety contract annotations per tool
├── main.rs              # Add "mcp" subcommand → run_mcp_server()
```

### Core Implementation

```rust
// src/mcp/mod.rs
use rmcp::{
    handler::server::wrapper::Json,
    tool, tool_handler, tool_router,
    model::ToolCallResult,
    service::{RoleServer, serve_server},
};

use crate::protocol::Action;
use crate::DaemonState;

/// Deskbrid MCP server — wraps the existing dispatch layer in MCP tools.
#[derive(Clone)]
pub struct DeskbridMcpServer {
    state: DaemonState,
}

#[tool_router]
impl DeskbridMcpServer {
    /// Create a new MCP server with shared daemon state.
    pub fn new(state: DaemonState) -> Self {
        Self { state }
    }

    // ── Diagnostics ─────────────────────────────────────────

    #[tool(
        name = "doctor",
        description = "Check desktop integration readiness.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn doctor(&self) -> ToolCallResult<Json<Value>> {
        let response = self.state.dispatch(Action::A11yDoctor).await?;
        Ok(Json(response))
    }

    // ── Discovery ──────────────────────────────────────────

    #[tool(
        name = "list_windows",
        description = "List all open windows with metadata.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn list_windows(&self) -> ToolCallResult<Json<Vec<WindowInfo>>> {
        let response = self.state.dispatch(Action::WindowsList).await?;
        let windows: Vec<WindowInfo> = serde_json::from_value(response)?;
        Ok(Json(windows))
    }

    #[tool(
        name = "get_accessibility_tree",
        description = "Full AT-SPI tree for an app or window. Returns labeled elements with bounds, roles, states, actions, and text content.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn get_accessibility_tree(
        &self,
        #[param(description = "App name to filter (optional)")]
        app_name: Option<String>,
        #[param(description = "PID to filter (optional)")]
        pid: Option<u32>,
        #[param(description = "Max nodes to return (default: 200)")]
        max_nodes: Option<usize>,
        #[param(description = "Max tree depth (default: 10)")]
        max_depth: Option<u32>,
    ) -> ToolCallResult<Json<Value>> {
        let response = self.state.dispatch(Action::A11yTree {
            app_name,
            pid,
            max_nodes,
            max_depth,
        }).await?;
        Ok(Json(response))
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
    async fn type_text(
        &self,
        #[param(description = "Text to type")]
        text: String,
    ) -> ToolCallResult<Json<Value>> {
        let response = self.state.dispatch(Action::InputKeyboardType { text }).await?;
        Ok(Json(response))
    }

    #[tool(
        name = "press_keys",
        description = "Press a key combination.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn press_keys(
        &self,
        #[param(description = "Keys to press (e.g., ['Control_L', 'c'])")]
        keys: Vec<String>,
    ) -> ToolCallResult<Json<Value>> {
        let response = self.state.dispatch(Action::InputKeyboardCombo { keys }).await?;
        Ok(Json(response))
    }

    #[tool(
        name = "mouse_move",
        description = "Move the mouse to absolute coordinates.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    async fn mouse_move(
        &self,
        #[param(description = "X coordinate")]
        x: f64,
        #[param(description = "Y coordinate")]
        y: f64,
    ) -> ToolCallResult<Json<Value>> {
        let response = self.state.dispatch(Action::InputMouse {
            action: "move".into(),
            x: Some(x),
            y: Some(y),
            button: None,
            dx: None,
            dy: None,
        }).await?;
        Ok(Json(response))
    }

    #[tool(
        name = "click",
        description = "Click a mouse button at the current position.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn click(
        &self,
        #[param(description = "Mouse button: 'left', 'middle', or 'right'")]
        button: Option<String>,
    ) -> ToolCallResult<Json<Value>> {
        let response = self.state.dispatch(Action::InputMouse {
            action: "click".into(),
            x: None,
            y: None,
            button: Some(button.unwrap_or_else(|| "left".into())),
            dx: None,
            dy: None,
        }).await?;
        Ok(Json(response))
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
        #[param(description = "X coordinate")]
        x: f64,
        #[param(description = "Y coordinate")]
        y: f64,
        #[param(description = "Mouse button")]
        button: Option<String>,
    ) -> ToolCallResult<Json<Value>> {
        let response = self.state.dispatch(Action::InputMouse {
            action: "click".into(),
            x: Some(x),
            y: Some(y),
            button: Some(button.unwrap_or_else(|| "left".into())),
            dx: None,
            dy: None,
        }).await?;
        Ok(Json(response))
    }

    // ── Screenshots ────────────────────────────────────────

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
    async fn screenshot(&self) -> ToolCallResult<Json<Value>> {
        let response = self.state.dispatch(Action::Screenshot).await?;
        Ok(Json(response))
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
    async fn clipboard_read(&self) -> ToolCallResult<Json<Value>> {
        let response = self.state.dispatch(Action::ClipboardRead).await?;
        Ok(Json(response))
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
        #[param(description = "Text to copy to clipboard")]
        text: String,
    ) -> ToolCallResult<Json<Value>> {
        let response = self.state.dispatch(Action::ClipboardWrite { text }).await?;
        Ok(Json(response))
    }

    // ── System ─────────────────────────────────────────────

    #[tool(
        name = "system_info",
        description = "System information — OS, kernel, memory, CPU.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn system_info(&self) -> ToolCallResult<Json<Value>> {
        let response = self.state.dispatch(Action::SystemInfo).await?;
        Ok(Json(response))
    }

    // ... 40+ more tools following the exact same pattern ...
}
```

### Entry Point

```rust
// src/main.rs
#[derive(clap::Subcommand)]
enum Command {
    /// Run the deskbrid daemon (Unix socket listener)
    Daemon { /* ... */ },
    /// Run as an MCP stdio server (for Hermes, Claude Desktop, Codex)
    Mcp,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Daemon { .. } => run_daemon().await?,
        Command::Mcp => {
            let state = DaemonState::new().await?;

            let server = DeskbridMcpServer::new(state);

            // Same pattern as computer-use-linux
            server
                .serve(rmcp::transport::stdio())
                .await?
                .waiting()
                .await?;
        }
    }

    Ok(())
}
```

---

## Part 4: Hermes Config — Drop-In Replacement

### Before (computer-use-linux)

```json
{
  "mcp_servers": {
    "computer-use-linux": {
      "command": "computer-use-linux",
      "args": ["mcp"],
      "timeout": 120,
      "connect_timeout": 30
    }
  }
}
```

### After (deskbrid)

```json
{
  "mcp_servers": {
    "deskbrid": {
      "command": "deskbrid",
      "args": ["mcp"],
      "timeout": 120,
      "connect_timeout": 30
    }
  }
}
```

### What changes for Hermes

| Before | After |
|--------|-------|
| 15 tools from computer-use-linux | 50+ tools from deskbrid |
| Desktop-only operations | Desktop + systemd + clipboard + audio + bluetooth + files + terminal + proxmox |
| AT-SPI only for accessibility | AT-SPI + semantic screen indexing (LINUX_CONTROL.md §52) |
| No service management | Start/stop/restart systemd services |
| No file operations | Read/write/search/write files |
| No terminal | Full PTY terminal |
| No clipboard | Clipboard read/write |
| No Proxmox | Full cluster/VM/LXC management |

---

## Part 5: Safety Contract

Following computer-use-linux's exact annotation pattern:

| Category | Tools | readOnlyHint | destructiveHint | idempotentHint |
|----------|-------|:---:|:---:|:---:|
| Diagnostics | `doctor`, `setup_*` | ✗ | ✗ | ✓ |
| Discovery | `list_windows`, `list_apps`, `get_accessibility_tree`, `screenshot` | ✓ | ✗ | ✓ |
| Clipboard | `clipboard_read`, `clipboard_write` | ✗* | ✗ | ✗ |
| Input | `type_text`, `press_key`, `click`, `drag` | ✗ | ✓ | ✗ |
| Window control | `focus_window`, `close_window`, `minimize_window` | ✗ | ✓ | ✗ |
| System read | `system_info`, `battery_status`, `network_status` | ✓ | ✗ | ✓ |
| System write | `service_start`, `service_stop` | ✗ | ✓ | ✗ |
| File read | `file_read`, `file_list`, `file_search` | ✓ | ✗ | ✓ |
| File write | `file_write`, `file_copy`, `file_delete` | ✗ | ✓ | ✗ |
| Terminal | `terminal_create`, `terminal_write` | ✗ | ✓ | ✗ |
| Proxmox read | `proxmox_status`, `proxmox_nodes` | ✓ | ✗ | ✓ |
| Proxmox write | `proxmox_guest_action`, `proxmox_guest_exec` | ✗ | ✓ | ✗ |

*\* clipboard_write is destructive; clipboard_read matches its category*

---

## Part 6: Implementation Roadmap

### Phase 1: Core MCP Bridge (1 day)

Build the thinnest possible bridge: a `deskbrid mcp` command that connects to the
existing Unix socket daemon and translates MCP tool calls to NDJSON protocol actions.

```
deskbrid mcp (rmcp stdio)
  └── Connects to Unix socket /run/user/1000/deskbrid.sock
        └── Forwards tool calls as NDJSON actions
```

**Why:** This works TODAY. No refactoring. No code changes to the daemon.
Proof-of-concept that Hermes can control the desktop through deskbrid.

**Files:**
- `src/mcp/mod.rs` — `DeskbridMcpServer` with socket client back to daemon
- `src/mcp/tools.rs` — All tool definitions pointing at daemon socket
- `src/main.rs` — Add `Mcp` subcommand

### Phase 2: Embedded MCP (2 days)

Move MCP into the daemon itself. `deskbrid daemon` starts both the Unix socket
listener AND the MCP server on a separate port or socket.

```
deskbrid daemon
  ├── Unix socket listener (/run/user/1000/deskbrid.sock)
  └── MCP-over-TCP listener (127.0.0.1:18796)
        └── Accepts MCP connections, dispatches through same Action system
```

**Why:** Single binary, single state. No double-stack. MCP connections share the
same backend, same permissions, same rate limits as Unix socket clients.

### Phase 3: Full MCP Coverage (2 days)

Register every deskbrid action as an MCP tool — 50+ tools spanning all 12 categories.
This is mechanical work: one `#[tool]` function per Action variant.

---

## Part 7: Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| MCP framework | `rmcp` v1.5+ | Same crate as computer-use-linux; industry standard for Rust MCP |
| Transport | Stdio first, TCP later | Stdio is simplest for Hermes; TCP for remote/containerized MCP hosts |
| Tool implementation | Wrap existing `dispatch_action` | Don't reimplement logic; reuse proven dispatch layer |
| Tool annotations | Same as computer-use-linux | Follows the standard MCP safety contract |
| Server struct | `DeskbridMcpServer` with `DaemonState` | Shared state with daemon; consistent backend |
| Discovery tools output | JSON matching protocol types | Same schemas whether via Unix socket or MCP |
| Desktop backends | All existing backends | GNOME, KDE, Hyprland, COSMIC, X11 — all work through same dispatch |
| Proxmox tools | Same MCP server, same binary | One `deskbrid mcp` command exposes everything; no separate server |
| Config | Same deskbrid TOML | SSH keys, Proxmox tokens, desktop preferences all in one place |

---

## Part 8: Why This Matters

`computer-use-linux` proved the pattern: a Rust MCP server for desktop control
works beautifully with Hermes. But it's purpose-built for one thing (desktop control),
exposing 15 tools.

Deskbrid already has 90+ protocol actions covering 12 domains. The MCP layer is a
new *transport* for the same engine — the same dispatch, the same backends, the same
permissions. Every tool, every category, every backend comes for free.

**The one semantic difference:** Deskbrid adds `proxmox.*` tools. No other desktop
control MCP server lets an agent query cluster status, start a container, or run a
command inside an LXC. That's deskbrid's superpower — it controls the machine AND
the cluster the machine lives on.
