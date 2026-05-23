# MCP Integration

Use Deskbrid with AI coding tools via the Model Context Protocol.

## What is MCP?

The Model Context Protocol (MCP) is a standard for AI agents to interact with external tools and services. Deskbrid provides an MCP server that exposes desktop control capabilities to AI agents.

## Available Tools

When MCP is enabled, Deskbrid exposes 50+ tools:

### Window Management
- `deskbrid_list_windows` - List all windows
- `deskbrid_focus_window` - Focus a window
- `deskbrid_close_window` - Close a window
- `deskbrid_tile_window` - Tile window to preset
- `deskbrid_activate_or_launch` - Launch or focus app

### Input Control
- `deskbrid_type_text` - Type text
- `deskbrid_send_keys` - Send key combination
- `deskbrid_mouse_click` - Click mouse
- `deskbrid_mouse_move` - Move mouse cursor
- `deskbrid_mouse_scroll` - Scroll mouse wheel

### Clipboard
- `deskbrid_clipboard_read` - Read clipboard
- `deskbrid_clipboard_write` - Write clipboard

### Screenshots
- `deskbrid_screenshot` - Capture screen
- `deskbrid_screenshot_ocr` - Capture with OCR

### Notifications
- `deskbrid_notify` - Send notification

### System
- `deskbrid_system_info` - Get system info
- `deskbrid_system_battery` - Get battery status
- `deskbrid_system_power` - Power actions

### Media
- `deskbrid_mpris_list` - List media players
- `deskbrid_mpris_control` - Control playback

### Services
- `deskbrid_service_list` - List services
- `deskbrid_service_start` - Start service
- `deskbrid_service_stop` - Stop service

### Terminals
- `deskbrid_terminal_create` - Create PTY
- `deskbrid_terminal_write` - Write to terminal
- `deskbrid_terminal_read` - Read from terminal

### Monitors
- `deskbrid_list_displays` - List monitors
- `deskbrid_set_monitor_resolution` - Set resolution

### Layout Profiles
- `deskbrid_save_layout_profile` - Save layout
- `deskbrid_restore_layout_profile` - Restore layout

## Starting MCP Server

### Stdio Mode (for AI tools)

```bash
deskbrid mcp
```

This starts the MCP server on stdin/stdout for communication with AI agents.

### TCP Mode

```bash
deskbrid daemon --mcp-port 18796
```

Connect at `tcp://localhost:18796`.

## Configuration

### Claude Desktop

`~/.config/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "deskbrid": {
      "command": "/usr/local/bin/deskbrid",
      "args": ["mcp"]
    }
  }
}
```

### Cursor

`.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "deskbrid": {
      "command": "deskbrid",
      "args": ["mcp"]
    }
  }
}
```

### Windsurf

`~/.codeium/windsurf/mcp.json`:

```json
{
  "mcpServers": {
    "deskbrid": {
      "command": "deskbrid",
      "args": ["mcp"]
    }
  }
}
```

## AI Agent Usage Examples

### Window Management

An AI agent can:

1. List windows to understand the current state
2. Focus a specific application
3. Type input into a window
4. Handle multiple windows in sequence

Example interaction:
```
AI: Let me check what windows are open.
→ deskbrid_list_windows()
← [{"id": "abc", "title": "VS Code", ...}]

AI: Let me focus VS Code and run a test.
→ deskbrid_focus_window(app_id="code")
→ deskbrid_type_text("npm test\n")
```

### Debugging Assistance

An AI agent can:
1. Take screenshots to see error messages
2. Run OCR to extract text
3. Fix issues based on what it reads

```
AI: Let me see what the error says.
→ deskbrid_screenshot_ocr()
← {"text": "Error: Cannot find module 'lodash'"}
```

## Tool Schema

Each tool has a JSON schema for parameters:

```json
{
  "name": "deskbrid_focus_window",
  "description": "Focus a window by app ID or title",
  "parameters": {
    "type": "object",
    "properties": {
      "app_id": {"type": "string", "description": "Application ID"},
      "title": {"type": "string", "description": "Window title substring"},
      "exact": {"type": "boolean", "description": "Exact title match"}
    }
  },
  "required": ["app_id", "title"]
}
```

## Security Considerations

When using MCP with AI agents:

1. The daemon runs with the user's permissions
2. AI agents can type and click anywhere
3. Review AI-generated actions before they execute
4. Use `deskbrid system check_auth` for sensitive actions

For development, you may want to:

1. Run Deskbrid with verbose logging
2. Review actions in the audit log
3. Use `deskbrid audit_log` to see what actions were taken