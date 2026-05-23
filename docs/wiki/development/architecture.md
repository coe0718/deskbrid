# Architecture

Deskbrid's system design and internal structure.

## Overview

Deskbrid is organized into several key layers:

```
┌─────────────────────────────────────────────────────────┐
│                    MCP Server (stdio/TCP)                │
├─────────────────────────────────────────────────────────┤
│                    Protocol Layer                        │
│  (JSON parsing, serialization, event routing)           │
├─────────────────────────────────────────────────────────┤
│                    Backend Layer                         │
│  (GNOME, Hyprland, KDE, X11 implementations)              │
├─────────────────────────────────────────────────────────┤
│                    Core Operations                         │
│  (Windows, Input, Clipboard, Screenshots, etc.)         │
└─────────────────────────────────────────────────────────┘
```

## Directory Structure

```
src/
├── lib.rs              # Core library and Action enum
├── cli/                # Command-line interface
├── protocol/           # JSON protocol handling
│   ├── mod.rs          # Action enum (90+ variants)
│   ├── parse.rs        # Request parsing
│   ├── serialize.rs    # Response serialization
│   ├── types.rs        # Shared types
│   └── events.rs       # Event definitions
├── daemon/             # Daemon implementation
│   ├── dispatch.rs     # Action dispatch to backends
│   ├── apps.rs         # Application listing
│   ├── audit.rs        # Audit logging
│   ├── client.rs       # Client connection handling
│   └── *.rs            # Backend handlers
├── backend/            # Desktop backend implementations
│   └── mod.rs
├── mcp/                # MCP server
│   └── tools.rs        # MCP tool definitions
└── config.rs           # Configuration management
```

## Core Components

### 1. Action Enum

The heart of Deskbrid is the `Action` enum in `protocol/mod.rs`:

```rust
pub enum Action {
    // System
    Ping,
    SystemInfo,
    SystemPower { action: String },
    
    // Windows
    WindowsList,
    WindowsFocus(String),
    WindowsTile { ... },
    
    // Input
    InputKeyboardType { text: String },
    InputMouse { action: String, ... },
    
    // ... 90+ more variants
}
```

### 2. Dispatch System

`daemon/dispatch.rs` routes actions to the appropriate backend:

```rust
match action {
    Action::WindowsList => handle_windows_list(backend),
    Action::WindowsFocus(window_id) => handle_windows_focus(backend, window_id),
    // ...
}
```

### 3. Backend Abstraction

Each desktop environment has a backend:

```
backend/
├── gnome.rs      # GNOME Shell + AT-SPI
├── hyprland.rs   # Hyprland IPC + ydotool
├── kde.rs        # KWayland + KWin scripts
└── x11.rs        # X11 via xdotool/wmctrl
```

### 4. Protocol Layer

All communication is JSON over Unix socket:

```
Request:  {"action": "windows.list"}
Response: {"type": "response", "status": "ok", "data": [...]}
Event:    {"type": "event", "event": "window.focused", ...}
```

### 5. MCP Integration

The MCP server exposes all actions as tools:

```
deskbrid_list_windows
deskbrid_focus_window
deskbrid_type_text
deskbrid_screenshot
...
```

## Data Flow

### Request Handling

```
1. Client connects to /run/user/$UID/deskbrid.sock
2. Sends: {"action": "windows.list", "id": "req1"}
3. Protocol parses JSON into Action enum
4. Dispatch routes to backend handler
5. Backend executes (e.g., calls wmctrl)
6. Response serialized and sent back
```

### Event Streaming

```
1. Client subscribes: {"action": "events.subscribe", "events": ["window.*"]}
2. Daemon starts event listener (e.g., GNOME Shell extension)
3. Events are streamed as they occur
4. Client handles events in real-time
```

## Backend Detection

Deskbrid auto-detects the desktop environment:

```rust
// Check environment variables
if std::env::var("XDG_CURRENT_DESKTOP") == Ok("GNOME") {
    // Use GNOME backend
} else if hyprland_socket_exists() {
    // Use Hyprland backend
} else if x11_running() {
    // Use X11 backend
}
```

## Configuration

Configuration is stored in:

- `~/.config/deskbrid/config.json` - User settings
- `~/.local/share/deskbrid/` - Data directory

```json
{
  "socket_path": "/run/user/1000/deskbrid.sock",
  "log_level": "info",
  "mcp_port": null
}
```

## Error Handling

Errors follow a consistent format:

```json
{
  "type": "response",
  "status": "error",
  "error": {
    "code": "not_found",
    "message": "Window not found: code"
  }
}
```

Error codes are defined in the protocol:

- `invalid_params` - Bad request
- `not_found` - Resource missing
- `permission_denied` - Access denied
- `not_supported` - Feature unavailable
- `backend_error` - Backend failed

## Thread Model

The daemon is multi-threaded:

- **Main thread**: Accepts connections
- **Worker threads**: Handle requests
- **Event threads**: Stream real-time events
- **Backend threads**: Long-running operations

## State Management

State is kept in:

- Runtime (in-memory) - Current window focus, clipboard
- Persistent - Layout profiles, configuration
- Backend - Actual desktop state (queried live)