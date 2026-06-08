# Deskbrid: MCP Server Mode + Native AT-SPI

**Goal:** Add MCP server mode alongside the existing Unix socket, and rebuild the AT-SPI
accessibility module to match (surpass) what computer-use-linux offers. Deskbrid becomes
the superset — any MCP client gets everything Deskbrid does, plus computer-use-linux users
migrate for free.

---

## Part 1: MCP Server Mode

**Crate:** `rmcp` v1.5+ (same as computer-use-linux)
**Transport:** Stdio (primary) + optional TCP (future)
**Status:** Not implemented. **Effort:** 3-4 days.

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     deskbrid daemon                           │
│                                                              │
│  ┌──────────────────┐  ┌──────────────────┐                  │
│  │  Unix Socket      │  │  MCP Server      │                  │
│  │  (NDJSON)         │  │  (rmcp stdio)    │                  │
│  │  Tokio listener   │  │  Per-connection   │                  │
│  │  SO_PEERCRED auth │  │  Stdio session    │                  │
│  └────────┬─────────┘  └────────┬─────────┘                  │
│           │                     │                             │
│           └──────────┬──────────┘                             │
│                      ▼                                         │
│           ┌──────────────────────┐                            │
│           │   Dispatch Layer     │                            │
│           │   (Action → backend) │                            │
│           └──────────────────────┘                            │
│                      │                                         │
│           ┌──────────────────────┐                            │
│           │   DesktopBackend     │                            │
│           │   (trait object)     │                            │
│           └──────────────────────┘                            │
│                                                              │
│  ┌──────────────────┐  ┌──────────────────┐                  │
│  │  AT-SPI2          │  │  XDG Portal      │                  │
│  │  (atspi crate)    │  │  RemoteDesktop   │                  │
│  └──────────────────┘  └──────────────────┘                  │
│                                                              │
│  ┌──────────────────┐  ┌──────────────────┐                  │
│  │  uinput           │  │  CLI tools       │                  │
│  │  AbsPointer       │  │  (ydotool, etc)  │                  │
│  └──────────────────┘  └──────────────────┘                  │
└─────────────────────────────────────────────────────────────┘
```

### MCP Mode Activation

Two modes, same binary:

```
deskbrid daemon          # Existing: Unix socket at /run/user/1000/deskbrid.sock
deskbrid mcp             # New: MCP stdio server (Claude Code, Cursor, etc.)
deskbrid daemon --mcp    # Both: Unix socket + MCP listener on TCP
```

### Tool Mapping

MCP tools map 1:1 to existing protocol actions plus new AT-SPI tools:

**Window Control (existing actions → MCP tools):**
| MCP Tool | Protocol Action | Notes |
|----------|----------------|-------|
| `list_windows` | `windows.list` | Already exists |
| `focus_window` | `windows.focus` | Already exists |
| `get_window` | `windows.get` | Already exists |
| `close_window` | `windows.close` | Already exists |
| `minimize_window` | `windows.minimize` | Already exists |
| `maximize_window` | `windows.maximize` | Already exists |
| `move_resize_window` | `windows.move_resize` | Already exists |
| `list_workspaces` | `workspaces.list` | Already exists |
| `switch_workspace` | `workspaces.switch` | Already exists |

**Input (existing actions → MCP tools):**
| MCP Tool | Protocol Action | Notes |
|----------|----------------|-------|
| `type_text` | `input.keyboard_type` | Already exists |
| `press_key` | `input.keyboard_key` | Already exists |
| `press_keys` | `input.keyboard_combo` | Already exists |
| `mouse_move` | `input.mouse_move` | Already exists |
| `mouse_click` | `input.mouse_click` | Already exists |
| `mouse_scroll` | `input.mouse_scroll` | Already exists |
| `click_coordinate` | → new | Use uinput AbsPointer for pixel-precise clicks |
| `drag` | → new | Portal RemoteDesktop drag |
| `screenshot` | `screenshot` | Already exists |

**Clipboard (existing actions → MCP tools):**
| MCP Tool | Protocol Action | Notes |
|----------|----------------|-------|
| `clipboard_read` | `clipboard.read` | Already exists |
| `clipboard_write` | `clipboard.write` | Already exists |

**System (existing actions → MCP tools):**
| MCP Tool | Protocol Action | Notes |
|----------|----------------|-------|
| `system_info` | `system.info` | Already exists |
| `battery_status` | `system.battery` | Already exists |
| `network_status` | `network.status` | Already exists |
| `idle_seconds` | `system.idle` | Already exists |

**New AT-SPI tools (not in protocol — added for MCP):**
| MCP Tool | Description | Key Output |
|----------|-------------|------------|
| `list_apps` | List AT-SPI application roots | `{name, pid, role, child_count, bounds}[]` |
| `get_accessibility_tree` | Snapshot AT-SPI tree for an app/window | `{nodes: AccessibilityNode[], count}` |
| `get_element_state` | Get details about a specific element | Full node info + actions + text |
| `perform_action` | Click/activate via AT-SPI Action interface | `{ok, action_index, action_name}` |
| `set_element_value` | Set element value (slider, text field, etc.) | `{method: "numeric" | "editable_text"}` |
| `get_element_text` | Read text content from an element | `{character_count, content, selections}` |
| `click_element` | Click element by AT-SPI path | Uses AT-SPI Action + coordinate fallback |
| `doctor` | Check desktop integration readiness | Dependency health report |
| `setup_accessibility` | Enable GNOME AT-SPI via gsettings | Same as computer-use-linux |
| `capabilities` | List available tools + per-backend status | Protocol-agnostic |

### Implementation: `deskbrid mcp` Mode

New binary entry or same daemon with `--mcp` flag.

**Option A (Same binary, no change to daemon):** `deskbrid mcp` starts as stdio MCP
server. Creates `DaemonState` with the detected backend. Uses `rmcp` transport.

```rust
// src/mcp/mod.rs
#[tokio::main(flavor = "current_thread")]
pub async fn run_mcp_server() -> anyhow::Result<()> {
    let event_tx = tokio::sync::broadcast::channel(256).0;
    let state = DaemonState::new(event_tx).await?;

    state.backend.read().await
        .as_ref()
        .context("no backend loaded")?;

    DeskbridMcp::new(state)
        .serve(rmcp::transport::stdio())
        .await?
        .waiting()
        .await?;
    Ok(())
}
```

**Option B (Dual protocol on TCP):** `deskbrid daemon --mcp` binds a TCP listener
on a configurable port (default `18796`). Each MCP-over-TCP connection gets its own
handler task.

```rust
// In daemon main loop
if args.mcp_port {
    let mcp_listener = TcpListener::bind(format!("127.0.0.1:{}", args.mcp_port)).await?;
    tokio::spawn(accept_mcp_connections(mcp_listener, state.clone()));
}
```

**Option A** is simpler and matches computer-use-linux (stdio only). **Option B**
is needed for remote client access. Ship Option A first, add Option B later.

### File Structure

```
src/
├── mcp/
│   ├── mod.rs          # MCP server struct, protocol mode
│   ├── tools.rs        # Tool dispatcher (routes to helpers + tool_list)
│   ├── tool_list.rs    # Tool definitions (list_tools)
│   ├── helpers.rs      # Shared helpers (do_execute, do_focus_window, etc.)
│   └── atspi.rs        # AT-SPI tools (delegates to a11y module)
├── main.rs             # Add "mcp" subcommand
```

### Dependencies (`Cargo.toml`)

```toml
rmcp = { version = "1.5", features = ["transport-io"], optional = true }
schemars = { version = "1", optional = true }

[features]
default = []
mcp = ["dep:rmcp", "dep:schemars"]
```

### MCP Tool Annotations

Every tool gets MCP annotations matching computer-use-linux patterns:

```rust
#[tool(
    name = "click_coordinate",
    description = "Click at pixel coordinates (x, y) using absolute pointer.",
    annotations(
        read_only_hint = false,
        destructive_hint = true,
        idempotent_hint = false,
        open_world_hint = true
    )
)]
```

---

## Part 2: Native AT-SPI Rebuild

**Goal:** Replace the current raw-zbus a11y module with a proper `atspi` crate-based
implementation that matches computer-use-linux feature-for-feature, then extends it.

**Crate:** `atspi` v0.29+ (from odilia-app)
**Status:** Existing module at `src/a11y/` is functional but bare-bones.
**Effort:** 4-5 days.

### Gap Analysis

| Feature | computer-use-linux | Deskbrid (current) | Deskbrid (target) |
|---------|-------------------|-------------------|-------------------|
| Tree/child traversal | BFS, max_nodes/max_depth, PID/name filtering | BFS, depth limit, role/name filtering only | ✅ Match + add PID filtering |
| Node info | role, name, description, child_count, bounds, states, actions, value, text | role, name, description, child_count, states | ✅ Add bounds, actions, value, text |
| Bounding boxes | ✅ via `get_extents(CoordType::Screen)` | ❌ Not implemented | ✅ Add |
| State names | All 40+ AT-SPI states | All 40+ AT-SPI states | ✅ Already have |
| Action interface | `get_actions()`, `do_action()` | ❌ Not implemented | ✅ Add |
| Value interface | `get_current_value()`, `set_current_value()` | ❌ Not implemented | ✅ Add |
| EditableText | `set_text_contents()` | ❌ Not implemented | ✅ Add |
| Text interface | `get_text()`, `get_caret_offset()`, `get_selections()` | Basic `get_text()` (0-100 chars) | ✅ Full |
| Element click | Via Action interface + coordinate fallback | Via Action interface only | ✅ Add coordinate fallback |
| Application listing | `list_accessible_apps()` | Custom `get_element()` | ✅ Add |
| Auto-disabled detection | ❌ | ❌ | ✅ Check `org.a11y.Bus.GetAddress` |
| AT-SPI peer connection | ✅ Dedicated P2P connection | ✅ Via `Builder::address()` | ✅ Match |
| PID resolution | ✅ Via D-Bus `GetConnectionUnixProcessID` | ❌ | ✅ Add |

### Implementation

**A) Replace `src/a11y/bus.rs` with `atspi` crate:**

```rust
use atspi::{
    connection::AccessibilityConnection,
    proxy::accessible::AccessibleProxy,
    CoordType, ObjectRef,
};

async fn connect() -> Result<AccessibilityConnection> {
    hydrate_session_bus_env();
    AccessibilityConnection::new()
        .await
        .context("failed to connect to AT-SPI bus")
}
```

The `atspi` crate handles:
- AT-SPI bus discovery (P2P connection to `org.a11y.atspi.Registry`)
- ObjectRef → AccessibleProxy conversion
- All property caching and deserialization
- Proper error handling for stale objects

**B) New `src/a11y/tree.rs` + `tree/tree_queries.rs` — Snapshot builder (matching computer-use-linux):**

```rust
pub struct AccessibilityNode {
    pub index: u32,
    pub parent_index: Option<u32>,
    pub depth: u32,
    pub object_ref: String,
    pub role: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub child_count: i32,
    pub bounds: Option<Bounds>,
    pub states: Vec<String>,
    pub actions: Vec<AccessibilityAction>,
    pub value: Option<AccessibilityValue>,
    pub text: Option<AccessibilityText>,
    pub supports_editable_text: bool,
}
```

**C) New `src/a11y/actions.rs` — Action invocation:**

```rust
pub async fn perform_action(
    object_ref_id: &str,
    action_name: Option<&str>,
) -> Result<ActionInvocation> {
    // 1. Connect to AT-SPI
    // 2. Resolve object_ref_id → ObjectRef
    // 3. Get Action proxy via accessible.proxies().action()
    // 4. List actions, match by name or index
    // 5. Call action.do_action(index)
    // 6. Return { ok, action_index, action_name }
}
```

**D) New `src/a11y/value.rs` — Value + EditableText:**

```rust
pub async fn set_element_value(
    object_ref_id: &str,
    value: &str,
) -> Result<ValueSetInvocation> {
    // 1. Try Value proxy (numeric): set_current_value(f64)
    // 2. Fall back to EditableText proxy: set_text_contents(str)
    // 3. Return which method was used
}

pub async fn get_element_text(
    object_ref_id: &str,
    max_chars: i32,
) -> Result<AccessibilityText> {
    // character_count, caret_offset, content, selections
}
```

**E) Keep and extend `src/a11y/util.rs`:**

The existing state parsing and role name mapping is correct. Add:
```rust
pub fn coord_type_from_string(s: &str) -> CoordType;
pub fn resolve_app_pid(conn: &AccessibilityConnection, object: &ObjectRef) -> Option<u32>;
```

**F) Protocol actions for AT-SPI:**

Add new actions to the protocol enum (used by both Unix socket and MCP):

```rust
A11yTree {
    app_name: Option<String>,
    pid: Option<u32>,
    max_nodes: Option<usize>,
    max_depth: Option<u32>,
},
A11yPerformAction {
    object_ref: String,
    action_name: Option<String>,
},
A11ySetValue {
    object_ref: String,
    value: String,
},
A11yGetText {
    object_ref: String,
    max_chars: Option<i32>,
},
A11yListApps {
    limit: Option<usize>,
},
```

### AT-SPI2 Bus Detection + Auto-Setup

Re-use computer-use-linux's GNOME accessibility detection + remediation:

```rust
pub fn check_accessibility_enabled() -> bool {
    // Check gsettings: org.gnome.desktop.interface accessibility-enable
}

pub fn enable_accessibility() -> Result<()> {
    // gsettings set org.gnome.desktop.interface accessibility-enable true
    // Check result and report
}

pub fn doctor_report() -> Value {
    // Check: gsettings, at-spi bus reachable, ydotool, grim, portal
}
```

---

## Part 3: Shared Infrastructure — uinput AbsPointer

**What:** Absolute pointer for pixel-precise clicks (bypasses portal RemoteDesktop).
**Crate:** `evdev`
**Status:** Not implemented. **Effort:** 1 day.

computer-use-linux uses uinput to create a virtual absolute-position mouse device:

```rust
// computer-use-linux abs_pointer.rs pattern
let mut device = evdev::Device::new()?;
device.set_name("deskbrid-virtual-pointer");
device.enable(evdev::RelativeAxisType::ABS_X)?;
device.enable(evdev::RelativeAxisType::ABS_Y)?;
// Set up abs range to match screen dimensions
let mut dev = device.borrow_fd()?;
dev.emit(&[ /* ABS_X + Y + SYN_REPORT */ ])?;
```

This gives zero-latency absolute positioning without the 120-second portal session
setup overhead. Use as the primary pointer when available, fall back to portal
RemoteDesktop when running under confinement (Flatpak/Snap).

---

## Implementation Roadmap

### Phase 1: AT-SPI Rebuild (4-5 days)

1. Add `atspi = "0.29"` dependency to `Cargo.toml`
2. Refactor `src/a11y/bus.rs` to use `atspi::AccessibilityConnection`
3. Add `src/a11y/tree.rs` + `tree/tree_queries.rs` — full snapshot builder with bounds, actions, value, text
4. Add `src/a11y/actions.rs` — perform_action with action selection
5. Add `src/a11y/value.rs` — Value + EditableText get/set
6. Add `src/a11y/text.rs` — full Text interface (character_count, caret, selections)
7. Add protocol actions: `A11yTree`, `A11yPerformAction`, `A11ySetValue`, `A11yGetText`, `A11yListApps`
8. Wire AT-SPI actions through daemon dispatch
9. Add `doctor` + `setup_accessibility` helpers

### Phase 2: MCP Server Mode (3-4 days)

1. Add `rmcp` + `schemars` dependencies (optional, feature-gated)
2. Create `src/mcp/` module structure
3. Implement `DeskbridMcp` struct with shared backend state
4. Implement core tools (list_windows, focus, type_text, click, screenshot)
5. Implement AT-SPI tools (list_apps, get_tree, perform_action, set_value, get_text)
6. Add `deskbrid mcp` subcommand to `main.rs`
7. Wire `mcp` feature in Cargo.toml

### Phase 3: Dual Protocol Mode (1-2 days)

1. TCP listener for MCP on configurable port
2. `deskbrid daemon --mcp` flag
3. Shared connection limit / rate limiting across both protocols

### Phase 4: Superset Features (2-3 days, optional)

1. uinput AbsPointer (`/dev/uinput` virtual absolute mouse)
2. Portal RemoteDesktop session for input (already partially in GNOME backend)
3. Coordinate-based click/drag tools
4. Remove portal session latency — keep persistent portal sessions

---

## Migration Path for computer-use-linux Users

Users switch by changing one line in their MCP config:

```json
// Before (computer-use-linux)
{
  "mcpServers": {
    "computer-use": {
      "command": "computer-use-linux",
      "args": ["mcp"]
    }
  }
}

// After (deskbrid)
{
  "mcpServers": {
    "deskbrid": {
      "command": "deskbrid",
      "args": ["mcp"]
    }
  }
}
```

**What they get:**
- All computer-use-linux tools (AT-SPI, windows, input, screenshots)
- Plus: clipboard ops, audio control, Bluetooth, network, file watching,
  workspace management, monitor control, systemd/journal, polkit, permissions,
  event subscriptions
- Multi-protocol (Unix socket for their own tools + MCP for AI coding tools)
- Same daemon instance for both protocols
- Better DE coverage (5 backends vs 6 in computer-use-linux, but Deskbrid also
  has X11, and MATE/Cinnamon are on the roadmap in this doc)

---

## AT-SPI Tool Details (Reference for Implementation)

### `get_accessibility_tree`

```
Inputs:
  app_name_or_bundle_identifier: Option<String>  # Filter by app
  window_id: Option<u64>        # Filter by window
  pid: Option<u32>              # Filter by PID
  max_nodes: Option<usize>      # Default: 200
  max_depth: Option<u32>        # Default: 10
  include_screenshot: Option<bool>  # Include screenshot of target window

Output:
  nodes: Vec<AccessibilityNode>
  count: u32
  screenshot: Option<String>    # Base64 PNG if include_screenshot=true
```

### `perform_action`

```
Inputs:
  element_index: Option<u32>    # Index from last tree snapshot
  object_ref: Option<String>    # Direct AT-SPI object ref
  action_name: Option<String>   # "click", "focus", "activate", etc. (auto if omitted)

Output:
  ok: bool
  action_index: i32
  action_name: Option<String>
```

### `click_element` vs `perform_action`

`perform_action` uses the AT-SPI Action D-Bus interface on the element directly.
`click_element` tries Action first, falls back to:
1. Get element bounds (x, y, width, height) from AT-SPI
2. Calculate center point: (x + width/2, y + height/2)
3. Move pointer to center via AbsPointer or Portal RemoteDesktop
4. Click

This fallback is why bounds extraction is critical — without it, headless element
clicking doesn't work on elements that don't expose AT-SPI Action.

---

## File diff: Changes to Existing Deskbrid

### New files
```
src/a11y/tree.rs       ~145 lines  AT-SPI tree types + BFS snapshot builder
src/a11y/tree/tree_queries.rs ~203 lines  AT-SPI D-Bus query functions
src/a11y/actions.rs    ~150 lines  AT-SPI action invocation
src/a11y/value.rs      ~100 lines  AT-SPI value + editable text
src/a11y/setup.rs      ~80 lines   Accessibility enable/doctor
src/mcp/mod.rs         ~155 lines  MCP server struct + serve()
src/mcp/tools.rs       ~96 lines   MCP tool dispatcher
src/mcp/tool_list.rs   ~211 lines  MCP tool definitions
src/mcp/helpers.rs     ~138 lines  Shared async helpers
```

### Modified files
```
src/a11y/bus.rs        → Refactor to use atspi crate
src/a11y/util.rs       → Add: coord_type_from_string, PID resolution
src/a11y.rs            → Re-export new submodules
src/backend/mod.rs     → No change (AT-SPI is daemon-level, not per-backend)
src/daemon/dispatch.rs → Add A11y* action handlers
src/protocol/mod.rs    → Add A11y* action variants
src/main.rs            → Add "mcp" subcommand
Cargo.toml             → Add atspi, rmcp, schemars, evdev deps
```

### Computer-use-linux features NOT porting

- **Terminal PTY metadata** — `src/terminal.rs` in computer-use-linux scans
  `/proc` to attach terminal PID, TTY, command, and CWD to each window. This is
  valuable but not AT-SPI or MCP-specific. Document as a future enhancement.
- **Compositor-specific absolute pointer** — computer-use-linux has a
  COSMIC-specific helper binary. Deskbrid already has this as `cosmic-helper`.
- **Gnome Shell extension** — Deskbrid already has one. The MCP tools just call
  the same backend.

### Computer-use-linux features to adopt

| Feature | computer-use-linux | Deskbrid Action |
|---------|-------------------|-----------------|
| `atspi` crate (not raw zbus) | Uses `atspi 0.29` | Switch dependency |
| Full accessibility node data | bounds, actions, value, text | Add to `AccessibilityNode` |
| Action invocation | `perform_action(object_ref, action_name)` | New protocol action |
| Value set (numeric + editable text) | `set_element_value(object_ref, value)` | New protocol action |
| Text interface (content, caret, selections) | `get_text` with full params | Extend existing |
| Application listing | `list_accessible_apps(50)` | New protocol action |
| PID-constrained tree | `snapshot_tree(None, Some(pid), ...)` | Add PID filter |
| uinput AbsPointer | `abs_pointer.rs` | New `src/abs_pointer.rs` |
| Portal RemoteDesktop sessions | `remote_desktop.rs` | Already has (partial) |
| Doctor report | `diagnostics.rs` | New `src/diagnostics.rs` |
| Setup accessibility | `setup_accessibility_report()` | New `src/a11y/setup.rs` |
