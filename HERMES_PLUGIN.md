# Deskbrid as a Hermes Agent Plugin

**Goal:** Ship Deskbrid as a first-class Hermes Agent integration so Tuck's agents can control Jeremy's Linux desktop through the same tool system they use for everything else. Windows, input, screenshots, clipboard, AT-SPI UI inspection, and eventually Proxmox — all called like any other Hermes tool.

---

## Part 1: Understanding Hermes Plugin System

### Plugin Architecture

Plugins live in `~/.hermes/plugins/<name>/`. Each plugin has a `plugin.yaml` manifest and a Python `__init__.py` with a `register(ctx)` function. Hermes calls `register(ctx)` at startup, passing a `ctx` object that lets the plugin wire itself in.

```
~/.hermes/plugins/deskbrid/
├── plugin.yaml          # "I'm deskbrid, I provide tools + hooks + skill"
├── __init__.py          # register(ctx) — wires everything together
└── skills/
    └── deskbrid.md      # Bundled skill teaching agents how to use desktop tools
```

### What Plugins Can Do

| Capability | API | Use for Deskbrid |
|------------|-----|------------------|
| Register tools | `ctx.register_tool(name, toolset, schema, handler)` | Expose desktop control as Hermes tools |
| Register hooks | `ctx.register_hook("hook_name", callback)` | Auto-start daemon, health checks |
| Register CLI commands | `ctx.register_cli_command(name, help, setup, handler)` | `hermes deskbrid setup`, `hermes deskbrid health` |
| Bundle skills | `ctx.register_skill("deskbrid", path)` | Ship skill that teaches agents HOW to use desktop tools |
| Declare env vars | `requires_env` in plugin.yaml | DESKBRID_SOCKET (optional, auto-detected) |
| Platform discovery | User, pip, project, Nix | Install anywhere |

### Available Hooks

| Hook | When it fires | Deskbrid use |
|------|---------------|--------------|
| `pre_agent_start` | Before each agent session begins | Auto-start `deskbrid daemon` if socket missing |
| `post_agent_start` | After agent initializes | Health check — ping the socket |
| `post_tool_call` | After any tool completes | Log tool usage for debugging |
| `pre_llm_call` | Before each LLM turn | Inject desktop context (active window, etc.) |

### Tool Handler Rules

Tool handlers are Python functions with signature `def handler(params: dict, **kwargs) -> str`. They MUST:
1. Accept `params` (dict) and `**kwargs`
2. Return a JSON string — always, even on errors
3. Never raise — catch all exceptions, return error JSON instead

### Plugin States

- **Discovered** — Hermes finds the plugin directory and reads its manifest
- **Enabled** — Added to `plugins.enabled` in `~/.hermes/config.yaml`
- **Loaded** — `register(ctx)` has been called, tools are available to agents

General plugins with tools/hooks are **disabled by default**. The user must explicitly enable them.

---

## Part 2: Integration Architecture

### The Primary Path: MCP Native (Zero Plugin Code)

Hermes has native MCP client support (`tools/mcp_tool.py`) that connects to MCP servers over stdio. Deskbrid's `deskbrid mcp` mode (from MCP_ATSPI_DESIGN.md) maps directly onto this.

```
Hermes Agent
  └── mcp_tool.py (built-in)
        └── stdio ──→ deskbrid mcp
                         └── Unix socket ──→ deskbrid daemon
                                               └── /dev/uinput, AT-SPI, xdg-portal, ...
```

**Configuration — 5 lines in `~/.hermes/config.yaml`:**

```yaml
mcp_servers:
  deskbrid:
    command: deskbrid
    args: ["mcp"]
    timeout: 120
    supports_parallel_tool_calls: false
```

That's it. Hermes discovers every Deskbrid tool automatically — `list_windows`, `focus_window`, `type_text`, `press_keys`, `mouse_move`, `mouse_click`, `screenshot`, `clipboard_read`, `clipboard_write`, `list_apps`, `get_accessibility_tree`, `click_element`, etc. All available to agents with no plugin code.

**This is the recommended path.** MCP is Hermes' standard for external tool servers. Deskbrid is an external tool server. The integration is a configuration entry, not code.

### The Plugin Wrapper: Three Levels

A Hermes plugin adds UX polish on top of the MCP integration:

| Level | What it does | Effort | Ships with |
|--------|-------------|--------|------------|
| **Level 1: MCP only** | Config entry in config.yaml | 0 days (Tuck builds MCP mode) | Nothing — it's just config |
| **Level 2: Plugin + Skill** | Plugin wraps MCP + ships bundled skill | 0.5 days | Skill file teaching agents HOW to use tools |
| **Level 3: Plugin + Direct Socket** | Plugin talks to socket directly (no MCP dep) | 2 days | Full integration before MCP mode exists |

---

## Part 3: Plugin Design

### plugin.yaml

```yaml
name: deskbrid
version: 1.0.0
description: >
  Linux desktop control for Hermes agents — windows, input, clipboard,
  screenshots, AT-SPI UI inspection, audio, system controls, and more.
  Connects to the deskbrid daemon over a Unix socket.
homepage: https://deskbrid.patchhive.dev
requires_env: []
optional_env:
  - name: DESKBRID_SOCKET
    description: "Path to deskbrid Unix socket (default: /run/user/$UID/deskbrid.sock)"
  - name: DESKBRID_MCP
    description: "Set to '1' to prefer MCP mode over direct socket"
    default: "0"
```

### __init__.py — Plugin Entry Point

```python
"""Deskbrid plugin — Linux desktop control for Hermes agents.

Three integration modes (auto-detected):
  1. MCP mode (preferred) — deskbrid mcp subprocess, Hermes' built-in MCP client
  2. Direct socket mode — talks to deskbrid daemon over Unix socket
  3. Subprocess mode — spawns deskbrid for one-shot commands (fallback)

Agents get: windows, input, clipboard, screenshots, AT-SPI, audio, system."""
from __future__ import annotations

import json
import logging
import os
import socket
import subprocess
import time
from pathlib import Path

logger = logging.getLogger(__name__)

# ── Constants ────────────────────────────────────────────────────────
SOCKET_PATH = os.environ.get(
    "DESKBRID_SOCKET",
    f"/run/user/{os.getuid()}/deskbrid.sock",
)
DEFAULT_TIMEOUT = 10.0


def register(ctx):
    """Wire deskbrid into Hermes — tools, hooks, and bundled skill."""

    # ── 1. Ship the bundled skill ──────────────────────────────────
    skill_path = Path(__file__).parent / "skills" / "deskbrid.md"
    if skill_path.exists():
        ctx.register_skill("deskbrid", str(skill_path))
        logger.info("Registered bundled skill: deskbrid")

    # ── 2. Auto-start daemon ────────────────────────────────────────
    ctx.register_hook("pre_agent_start", _ensure_daemon_started)

    # ── 3. Register all tools ───────────────────────────────────────
    _register_window_tools(ctx)
    _register_input_tools(ctx)
    _register_clipboard_tools(ctx)
    _register_screenshot_tools(ctx)
    _register_system_tools(ctx)
    _register_audio_tools(ctx)

    # ── 4. Setup CLI command ────────────────────────────────────────
    ctx.register_cli_command(
        name="deskbrid-setup",
        help="Install and configure deskbrid daemon",
        setup_fn=None,
        handler_fn=_cli_setup,
    )


# ── Socket communication ────────────────────────────────────────────

def _send_action(action: dict, timeout: float = DEFAULT_TIMEOUT) -> dict:
    """Send a JSON action to deskbrid daemon and return the response.

    Action format: {"type": "windows.list", "id": "1"}
    Response format: {"ok": true, "data": {...}}
    """
    try:
        sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        sock.settimeout(timeout)
        sock.connect(SOCKET_PATH)
        sock.sendall((json.dumps(action) + "\n").encode())
        buf = b""
        while True:
            chunk = sock.recv(4096)
            if not chunk:
                break
            buf += chunk
            if b"\n" in buf:
                break
        sock.close()
        return json.loads(buf.decode().strip())
    except FileNotFoundError:
        return {"ok": False, "error": "deskbrid daemon not running (socket not found)"}
    except socket.timeout:
        return {"ok": False, "error": f"deskbrid timeout after {timeout}s"}
    except Exception as e:
        return {"ok": False, "error": str(e)}


# ── Hook: auto-start daemon ─────────────────────────────────────────

def _ensure_daemon_started(**kwargs):
    """Start deskbrid daemon if the socket doesn't exist yet."""
    if os.path.exists(SOCKET_PATH):
        return

    logger.info("deskbrid socket not found — starting daemon...")
    try:
        subprocess.Popen(
            ["deskbrid", "daemon"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
        )
        # Give it a moment to create the socket
        for _ in range(20):  # 2 second max wait
            if os.path.exists(SOCKET_PATH):
                logger.info("deskbrid daemon started successfully")
                return
            time.sleep(0.1)
        logger.warning("deskbrid daemon started but socket not found after 2s")
    except FileNotFoundError:
        logger.warning(
            "deskbrid binary not found — install with: "
            "curl -fsSL https://deskbrid.patchhive.dev/install.sh | bash"
        )


# ── Tool registrations ──────────────────────────────────────────────

def _register_window_tools(ctx):
    """Window management tools."""
    ctx.register_tool(
        name="list_windows",
        toolset="deskbrid",
        schema={
            "name": "list_windows",
            "description": "List all open windows on the Linux desktop. Returns window IDs, titles, classes, workspace, and geometry for each window.",
            "parameters": {
                "type": "object",
                "properties": {},
                "required": [],
            },
        },
        handler=lambda params, **kw: json.dumps(
            _send_action({"type": "windows.list", "id": "1"})
        ),
        description="List all open windows on the desktop.",
    )

    ctx.register_tool(
        name="focus_window",
        toolset="deskbrid",
        schema={
            "name": "focus_window",
            "description": "Focus (activate) a window by its ID. Brings the window to the foreground.",
            "parameters": {
                "type": "object",
                "properties": {
                    "window_id": {
                        "type": "string",
                        "description": "Window ID from list_windows",
                    },
                },
                "required": ["window_id"],
            },
        },
        handler=lambda params, **kw: json.dumps(
            _send_action({
                "type": "windows.focus",
                "id": "1",
                "window_id": params.get("window_id", ""),
            })
        ),
        description="Focus a window by its ID.",
    )

    ctx.register_tool(
        name="close_window",
        toolset="deskbrid",
        schema={
            "name": "close_window",
            "description": "Close a window by its ID.",
            "parameters": {
                "type": "object",
                "properties": {
                    "window_id": {"type": "string", "description": "Window ID to close"},
                },
                "required": ["window_id"],
            },
        },
        handler=lambda params, **kw: json.dumps(
            _send_action({
                "type": "windows.close",
                "id": "1",
                "window_id": params.get("window_id", ""),
            })
        ),
        description="Close a window by its ID.",
    )

    ctx.register_tool(
        name="minimize_window",
        toolset="deskbrid",
        schema={
            "name": "minimize_window",
            "description": "Minimize a window by its ID.",
            "parameters": {
                "type": "object",
                "properties": {
                    "window_id": {"type": "string"},
                },
                "required": ["window_id"],
            },
        },
        handler=lambda params, **kw: json.dumps(
            _send_action({
                "type": "windows.minimize",
                "id": "1",
                "window_id": params.get("window_id", ""),
            })
        ),
        description="Minimize a window.",
    )

    ctx.register_tool(
        name="maximize_window",
        toolset="deskbrid",
        schema={
            "name": "maximize_window",
            "description": "Maximize (or un-maximize) a window.",
            "parameters": {
                "type": "object",
                "properties": {
                    "window_id": {"type": "string"},
                },
                "required": ["window_id"],
            },
        },
        handler=lambda params, **kw: json.dumps(
            _send_action({
                "type": "windows.maximize",
                "id": "1",
                "window_id": params.get("window_id", ""),
            })
        ),
        description="Maximize a window.",
    )

    ctx.register_tool(
        name="move_resize_window",
        toolset="deskbrid",
        schema={
            "name": "move_resize_window",
            "description": "Move and/or resize a window. Coordinates are screen pixels.",
            "parameters": {
                "type": "object",
                "properties": {
                    "window_id": {"type": "string"},
                    "x": {"type": "integer", "description": "New X position"},
                    "y": {"type": "integer", "description": "New Y position"},
                    "width": {"type": "integer", "description": "New width"},
                    "height": {"type": "integer", "description": "New height"},
                },
                "required": ["window_id", "x", "y", "width", "height"],
            },
        },
        handler=lambda params, **kw: json.dumps(
            _send_action({
                "type": "windows.move_resize",
                "id": "1",
                "window_id": params.get("window_id", ""),
                "x": params.get("x", 0),
                "y": params.get("y", 0),
                "width": params.get("width", 800),
                "height": params.get("height", 600),
            })
        ),
        description="Move and/or resize a window.",
    )

    ctx.register_tool(
        name="list_workspaces",
        toolset="deskbrid",
        schema={
            "name": "list_workspaces",
            "description": "List all workspaces/virtual desktops with their current state.",
            "parameters": {
                "type": "object",
                "properties": {},
                "required": [],
            },
        },
        handler=lambda params, **kw: json.dumps(
            _send_action({"type": "workspaces.list", "id": "1"})
        ),
        description="List all workspaces/virtual desktops.",
    )

    ctx.register_tool(
        name="switch_workspace",
        toolset="deskbrid",
        schema={
            "name": "switch_workspace",
            "description": "Switch to a specific workspace by number.",
            "parameters": {
                "type": "object",
                "properties": {
                    "workspace_id": {"type": "integer"},
                },
                "required": ["workspace_id"],
            },
        },
        handler=lambda params, **kw: json.dumps(
            _send_action({
                "type": "workspaces.switch",
                "id": "1",
                "workspace_id": params.get("workspace_id", 0),
            })
        ),
        description="Switch to a workspace by number.",
    )


def _register_input_tools(ctx):
    """Keyboard and mouse input tools."""
    ctx.register_tool(
        name="type_text",
        toolset="deskbrid",
        schema={
            "name": "type_text",
            "description": "Type a string of text via keyboard input. Types at human speed.",
            "parameters": {
                "type": "object",
                "properties": {
                    "text": {"type": "string", "description": "Text to type"},
                },
                "required": ["text"],
            },
        },
        handler=lambda params, **kw: json.dumps(
            _send_action({
                "type": "input.keyboard",
                "id": "1",
                "action": "type",
                "text": params.get("text", ""),
            })
        ),
        description="Type text via keyboard input.",
    )

    ctx.register_tool(
        name="press_key",
        toolset="deskbrid",
        schema={
            "name": "press_key",
            "description": "Press and release a single key by name (e.g., 'Return', 'Escape', 'Tab', 'F5').",
            "parameters": {
                "type": "object",
                "properties": {
                    "key": {"type": "string", "description": "Key name (X11 keysym)"},
                },
                "required": ["key"],
            },
        },
        handler=lambda params, **kw: json.dumps(
            _send_action({
                "type": "input.keyboard",
                "id": "1",
                "action": "key",
                "key": params.get("key", ""),
            })
        ),
        description="Press a single key.",
    )

    ctx.register_tool(
        name="press_keys",
        toolset="deskbrid",
        schema={
            "name": "press_keys",
            "description": "Press a key combination. Keys are pressed in order, then released in reverse. Use for shortcuts like Control_L+c for copy.",
            "parameters": {
                "type": "object",
                "properties": {
                    "keys": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Keys to press, e.g., ['Control_L', 'c']",
                    },
                },
                "required": ["keys"],
            },
        },
        handler=lambda params, **kw: json.dumps(
            _send_action({
                "type": "input.keyboard",
                "id": "1",
                "action": "combo",
                "keys": params.get("keys", []),
            })
        ),
        description="Press a key combination.",
    )

    ctx.register_tool(
        name="mouse_move",
        toolset="deskbrid",
        schema={
            "name": "mouse_move",
            "description": "Move the mouse cursor to absolute screen coordinates.",
            "parameters": {
                "type": "object",
                "properties": {
                    "x": {"type": "number", "description": "X coordinate (pixels)"},
                    "y": {"type": "number", "description": "Y coordinate (pixels)"},
                },
                "required": ["x", "y"],
            },
        },
        handler=lambda params, **kw: json.dumps(
            _send_action({
                "type": "input.mouse",
                "id": "1",
                "action": "move",
                "x": params.get("x", 0),
                "y": params.get("y", 0),
            })
        ),
        description="Move the mouse cursor.",
    )

    ctx.register_tool(
        name="mouse_click",
        toolset="deskbrid",
        schema={
            "name": "mouse_click",
            "description": "Click a mouse button at the current cursor position.",
            "parameters": {
                "type": "object",
                "properties": {
                    "button": {
                        "type": "string",
                        "enum": ["left", "middle", "right"],
                        "description": "Mouse button to click",
                    },
                },
                "required": ["button"],
            },
        },
        handler=lambda params, **kw: json.dumps(
            _send_action({
                "type": "input.mouse",
                "id": "1",
                "action": "click",
                "button": params.get("button", "left"),
            })
        ),
        description="Click a mouse button.",
    )

    ctx.register_tool(
        name="mouse_scroll",
        toolset="deskbrid",
        schema={
            "name": "mouse_scroll",
            "description": "Scroll the mouse wheel. Positive dy = scroll down, negative = scroll up.",
            "parameters": {
                "type": "object",
                "properties": {
                    "dx": {"type": "number", "description": "Horizontal scroll (default: 0)"},
                    "dy": {"type": "number", "description": "Vertical scroll (positive = down)"},
                },
                "required": [],
            },
        },
        handler=lambda params, **kw: json.dumps(
            _send_action({
                "type": "input.mouse",
                "id": "1",
                "action": "scroll",
                "dx": params.get("dx", 0),
                "dy": params.get("dy", 0),
            })
        ),
        description="Scroll the mouse wheel.",
    )

    ctx.register_tool(
        name="click_coordinate",
        toolset="deskbrid",
        schema={
            "name": "click_coordinate",
            "description": "Move to pixel coordinates and click. Combines mouse_move + mouse_click in one operation.",
            "parameters": {
                "type": "object",
                "properties": {
                    "x": {"type": "number"},
                    "y": {"type": "number"},
                    "button": {"type": "string", "enum": ["left", "middle", "right"]},
                },
                "required": ["x", "y"],
            },
        },
        handler=lambda params, **kw: json.dumps(
            _send_action({
                "type": "input.mouse",
                "id": "1",
                "action": "click",
                "x": params.get("x", 0),
                "y": params.get("y", 0),
                "button": params.get("button", "left"),
            })
        ),
        description="Click at specific screen coordinates.",
    )


def _register_clipboard_tools(ctx):
    """Clipboard read/write tools."""
    ctx.register_tool(
        name="clipboard_read",
        toolset="deskbrid",
        schema={
            "name": "clipboard_read",
            "description": "Read the current clipboard contents. Returns the text currently on the clipboard.",
            "parameters": {
                "type": "object",
                "properties": {},
                "required": [],
            },
        },
        handler=lambda params, **kw: json.dumps(
            _send_action({"type": "clipboard.read", "id": "1"})
        ),
        description="Read clipboard contents.",
    )

    ctx.register_tool(
        name="clipboard_write",
        toolset="deskbrid",
        schema={
            "name": "clipboard_write",
            "description": "Write text to the system clipboard.",
            "parameters": {
                "type": "object",
                "properties": {
                    "text": {"type": "string", "description": "Text to copy to clipboard"},
                },
                "required": ["text"],
            },
        },
        handler=lambda params, **kw: json.dumps(
            _send_action({
                "type": "clipboard.write",
                "id": "1",
                "text": params.get("text", ""),
            })
        ),
        description="Write text to clipboard.",
    )


def _register_screenshot_tools(ctx):
    """Screenshot tools."""
    ctx.register_tool(
        name="screenshot",
        toolset="deskbrid",
        schema={
            "name": "screenshot",
            "description": "Take a screenshot of the entire desktop or a specific region. Returns a base64-encoded PNG image. Use this to verify UI state after performing actions.",
            "parameters": {
                "type": "object",
                "properties": {},
                "required": [],
            },
        },
        handler=lambda params, **kw: json.dumps(
            _send_action({"type": "screenshot", "id": "1"})
        ),
        description="Take a screenshot of the desktop.",
    )


def _register_system_tools(ctx):
    """System status tools."""
    ctx.register_tool(
        name="system_info",
        toolset="deskbrid",
        schema={
            "name": "system_info",
            "description": "Get system information — hostname, OS, kernel, uptime, memory, CPU.",
            "parameters": {
                "type": "object",
                "properties": {},
                "required": [],
            },
        },
        handler=lambda params, **kw: json.dumps(
            _send_action({"type": "system.info", "id": "1"})
        ),
        description="Get system information.",
    )

    ctx.register_tool(
        name="idle_seconds",
        toolset="deskbrid",
        schema={
            "name": "idle_seconds",
            "description": "Get how long the user has been idle (no mouse/keyboard input) in seconds.",
            "parameters": {
                "type": "object",
                "properties": {},
                "required": [],
            },
        },
        handler=lambda params, **kw: json.dumps(
            _send_action({"type": "system.idle", "id": "1"})
        ),
        description="Get user idle time in seconds.",
    )


def _register_audio_tools(ctx):
    """Audio control tools."""
    ctx.register_tool(
        name="list_audio_sinks",
        toolset="deskbrid",
        schema={
            "name": "list_audio_sinks",
            "description": "List all audio output devices (sinks) with volume and mute status.",
            "parameters": {
                "type": "object",
                "properties": {},
                "required": [],
            },
        },
        handler=lambda params, **kw: json.dumps(
            _send_action({"type": "audio.list_sinks", "id": "1"})
        ),
        description="List audio output devices.",
    )

    ctx.register_tool(
        name="set_volume",
        toolset="deskbrid",
        schema={
            "name": "set_volume",
            "description": "Set the volume of an audio sink. Volume is a float from 0.0 to 1.0.",
            "parameters": {
                "type": "object",
                "properties": {
                    "sink_name": {"type": "string", "description": "Sink name from list_audio_sinks"},
                    "volume": {"type": "number", "description": "Volume level (0.0 - 1.0)"},
                },
                "required": ["sink_name", "volume"],
            },
        },
        handler=lambda params, **kw: json.dumps(
            _send_action({
                "type": "audio.set_sink_volume",
                "id": "1",
                "sink_name": params.get("sink_name", ""),
                "volume": params.get("volume", 0.5),
            })
        ),
        description="Set audio volume.",
    )
```

### skills/deskbrid.md — Bundled Skill

```markdown
---
name: deskbrid
description: >
  Full control over a Linux desktop — windows, keyboard/mouse input,
  AT-SPI UI inspection, clipboard, screenshots, audio, and system status.
  Use these tools to automate desktop tasks, interact with GUI applications,
  and inspect what's happening on screen.
tools: [list_windows, focus_window, close_window, minimize_window, maximize_window,
  move_resize_window, list_workspaces, switch_workspace, type_text, press_key,
  press_keys, mouse_move, mouse_click, mouse_scroll, click_coordinate,
  clipboard_read, clipboard_write, screenshot, system_info, idle_seconds,
  list_audio_sinks, set_volume]
---

# Deskbrid — Linux Desktop Control

You have full control over the Linux desktop through Deskbrid tools. Use them to
automate any GUI task — opening apps, filling forms, clicking buttons, reading text,
taking screenshots to verify results.

## Core Workflow

1. **See what's happening**: Use `screenshot` to take a picture of the screen
2. **Find your target**: Use `list_windows` to see what's open, or AT-SPI tools to
   inspect UI elements
3. **Act**: Use keyboard/mouse tools to interact
4. **Verify**: Take another screenshot to confirm the result

## Window Management

- `list_windows` — Get IDs for every open window
- `focus_window` — Bring a window to the foreground
- `close_window`, `minimize_window`, `maximize_window` — Standard window ops
- `move_resize_window` — Position and size windows precisely
- `list_workspaces` / `switch_workspace` — Navigate virtual desktops

## Keyboard Input

- `type_text` — Type a string at human speed. Use for text fields, terminal
  commands, search boxes.
- `press_key` — Single key: 'Return', 'Escape', 'Tab', 'F5', etc.
- `press_keys` — Key combos: ['Control_L', 'c'] for copy, ['Alt_L', 'F4'] for
  close, ['Super_L'] for app launcher

**Important**: After clicking into a text field, wait briefly before typing.

## Mouse Input

- `mouse_move` — Move to absolute coordinates
- `mouse_click` — Click left/middle/right at current position
- `mouse_scroll` — Scroll wheel (positive dy = down)
- `click_coordinate` — Move + click in one operation

## Clipboard

- `clipboard_read` — Get what's on the clipboard
- `clipboard_write` — Put text on the clipboard (then paste with Control_L+v)

## Screenshots

- `screenshot` — Capture the full screen as a base64 PNG

**Best practice**: ALWAYS take a screenshot after performing actions to verify
they worked. Take one before to understand the current state.

## Audio

- `list_audio_sinks` — See audio output devices
- `set_volume` — Set volume (0.0-1.0)

## System Status

- `system_info` — Hostname, OS, kernel, memory, CPU
- `idle_seconds` — How long since the user last touched mouse/keyboard

## Common Patterns

### Opening an Application

```
1. press_keys(["Super_L"])                  # Open app launcher
2. type_text("firefox")                     # Type app name
3. press_key("Return")                      # Launch it
4. Wait 2 seconds
5. screenshot()                             # Verify it opened
```

### Clicking a Button by Position

```
1. screenshot()                             # See where the button is
2. click_coordinate(x=450, y=320)           # Click it
3. screenshot()                             # Verify the action
```

### Filling a Form

```
1. focus_window("firefox-123")              # Focus the browser
2. click_coordinate(x=200, y=150)           # Click into field
3. type_text("hello@example.com")            # Type text
4. press_key("Tab")                         # Next field
5. type_text("mypassword")                  # Type password
6. press_key("Return")                      # Submit
```

### Reading Clipboard Content

```
1. press_keys(["Control_L", "a"])           # Select all
2. press_keys(["Control_L", "c"])           # Copy
3. clipboard_read()                         # Get the text
```

## Limitations

- Cannot read text directly from the screen (use AT-SPI or clipboard patterns)
- AT-SPI requires applications to support accessibility (most GTK/Qt apps do)
- Some applications (games, DRM-protected content) may not screenshot
- Wayland has stricter security — some input methods may be limited
```

---

## Part 4: Integration Levels

### Level 1 — MCP Only (Preferred)

**What**: Tuck builds `deskbrid mcp` mode, adds 5 lines to Hermes config.

**Effort**: Zero plugin code. The MCP mode is already designed in MCP_ATSPI_DESIGN.md.

**Config**:
```yaml
# ~/.hermes/config.yaml
mcp_servers:
  deskbrid:
    command: deskbrid
    args: ["mcp"]
    timeout: 120
    supports_parallel_tool_calls: false
```

**Pros**: Standard Hermes pattern, auto-tool-discovery, no Python wrapper needed.

**Cons**: No bundled skill (agent doesn't know best practices for desktop tools).
No auto-start. Purely a tool server — the agent gets tools but no guidance.

### Level 2 — Plugin + Skill (Recommended)

**What**: Thin plugin from Part 3 above. Ships the skill file, auto-starts daemon,
registers direct-socket tools as fallback.

**Effort**: 0.5 days (copy the code from this document, test).

**Pros**: Bundled skill teaches agents HOW to use desktop tools (massive quality
improvement). Auto-start means no manual daemon management. Skill evolves
independently from the binary.

**Cons**: Requires plugin to be enabled in config.

### Level 3 — Plugin + Direct Socket (MCP-less)

**What**: Plugin talks directly to the Unix socket. No MCP mode needed.

**Effort**: 2 days (copy code from Part 3, test all tool handlers, handle edge cases).

**Pros**: Works before MCP mode is built. Full control over tool schemas.

**Cons**: More code to maintain. Doesn't benefit from MCP tool discovery.

---

## Part 5: Installation

### User Setup Flow

```
# 1. Install deskbrid binary (one-liner)
curl -fsSL https://deskbrid.patchhive.dev/install.sh | bash

# 2. Start the daemon (or let the plugin auto-start it)
deskbrid daemon &

# 3. Install the Hermes plugin
mkdir -p ~/.hermes/plugins/deskbrid
# Copy plugin files from Part 3

# 4. Enable the plugin
# Add 'deskbrid' to plugins.enabled in ~/.hermes/config.yaml

# 5. Restart Hermes
```

### Config — Single Entry to Enable

```yaml
# ~/.hermes/config.yaml
plugins:
  enabled:
    - deskbrid

mcp_servers:
  deskbrid:
    command: deskbrid
    args: ["mcp"]
    timeout: 120
    supports_parallel_tool_calls: false
```

---

## Part 6: Skill System — The Secret Sauce

The bundled skill is the most important part of this integration. Without it,
agents get a list of tools with descriptions — they know they CAN take a screenshot
but don't know they SHOULD take one after clicking.

The skill teaches patterns:

| Without skill | With skill |
|---------------|------------|
| "I'll click the button" | "I'll click the button, wait, then screenshot to verify" |
| "I'll type into the field" | "I'll click into the field first, then type" |
| Gets confused by window focus | Uses `focus_window` before interacting |
| Screenshots randomly | Screenshots before AND after every action |

### Where Skills Load

Skills are injected into the system prompt at session start. This means every
agent session using deskbrid gets the best-practices guide without the user
having to explain anything.

---

## Part 7: AT-SPI Tools (Future)

When the AT-SPI rebuild (MCP_ATSPI_DESIGN.md Phase 1) lands, these additional
tools become available:

```
list_apps               — List AT-SPI application roots
get_accessibility_tree  — Full UI tree with bounds, roles, states
get_element_text        — Read text content from an element
click_element           — Click via AT-SPI path (more reliable than coordinates)
perform_action          — Activate/expand/collapse elements
set_element_value       — Set slider value, select dropdown option
doctor                  — Check AT-SPI readiness
```

These should be added to the plugin's tool registrations and the bundled skill
when available.

---

## Part 8: Proxmox Integration (Future)

When the Proxmox module (PROXMOX.md) lands, the plugin adds:

```
proxmox_status           — Full cluster inventory
proxmox_containers       — List LXC containers
proxmox_vms              — List QEMU VMs
proxmox_guest_action     — Start/stop/reboot guests
proxmox_guest_exec       — Run command inside a container
```

These are registered in the same `register(ctx)` function, same `deskbrid` toolset,
same socket. No separate plugin needed — Deskbrid handles routing internally.

---

## Part 9: Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Primary integration | MCP (Hermes native) | Zero code, standard pattern, auto-discovery |
| Plugin role | Skill bundling + auto-start | Skill is the value; MCP handles tools |
| Direct socket fallback | Yes | Works before MCP mode is complete |
| One plugin for everything | Yes | Windows + AT-SPI + Proxmox all through same socket |
| Toolset name | `deskbrid` | Single toolset for all desktop/proxmox tools |
| Skill format | Bundled markdown | Standard Hermes skill pattern |
| Daemon management | Auto-start hook | No manual `deskbrid daemon &` needed |
| No external deps | Socket + json only | Uses Python stdlib, no pip installs |
| Env vars | Optional, auto-detected | Works out of the box, configurable if needed |
