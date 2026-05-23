# AI Agents

Using Deskbrid with AI coding tools and agents.

## How It Works

AI agents (Claude Code, Cursor, Windsurf, etc.) can control your desktop through Deskbrid's MCP server. The agent:

1. Calls tools like `deskbrid_list_windows` or `deskbrid_focus_window`
2. Deskbrid executes the action on the desktop
3. The agent observes results and continues

## Setup for AI Agents

### Claude Code

Add to your MCP configuration:

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

Start Deskbrid daemon first:

```bash
deskbrid daemon &
```

### Cursor

Add `.cursor/mcp.json`:

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

### Other MCP-Compatible Agents

Most AI coding agents support MCP. Check your tool's documentation for adding MCP servers.

## Agent Capabilities

With Deskbrid, AI agents can:

### Debug Applications

```text
Agent: Let me see the error in your terminal.
→ deskbrid_screenshot_ocr()
← {"text": "Error: Connection refused"}
→ deskbrid_type_text("netstat -tlnp\n")
```

### Test UI Interactions

```text
Agent: Testing the save dialog...
→ deskbrid_type_text("Ctrl+Shift+S")
→ deskbrid_type_text("test-file.txt")
→ deskbrid_type_text("Enter")
```

### Automate Repetitive Tasks

```text
Agent: Organizing your windows...
→ deskbrid_tile_window(window_id, preset="left")
→ deskbrid_tile_window(other_id, preset="right")
```

### Read and Write Code

```text
Agent: Opening the file and checking syntax...
→ deskbrid_type_text("code src/main.rs")
→ deskbrid_screenshot_ocr()
```

## Safety Considerations

### Before You Start

1. **Review the code** - Understand what actions the agent will take
2. **Close sensitive apps** - Don't have password managers or banking open
3. **Watch the screen** - Observe actions as they happen
4. **Start simple** - Begin with read-only operations

### Permission Model

Deskbrid respects system permissions:

- No elevated privileges needed for basic operations
- Power actions require authorization
- System modifications may prompt for confirmation

### Audit Log

Check what actions were taken:

```bash
deskbrid audit_log
```

## Common Agent Workflows

### Web Testing

```
1. Agent opens browser to test URL
2. Takes screenshot with OCR
3. Reads error messages
4. Types fixes
5. Verifies fixes
```

### Development Assistance

```
1. Agent lists open files in editor
2. Reads code via screenshot if needed
3. Types corrections
4. Runs tests
5. Checks results
```

### System Administration

```
1. Agent lists running services
2. Checks logs via journal query
3. Starts/stops services
4. Monitors system status
```

## Best Practices

### For Users

1. **Explicit commands** - Be specific about what you want the agent to do
2. **Watch the screen** - Confirm actions are correct
3. **Interrupt if needed** - Ctrl+C stops most agent operations
4. **Use dry runs** - Ask "what would you do?" first

### For Agent Developers

1. **Prefer high-level actions** - `tile_window` over raw mouse movements
2. **Read before write** - Screenshot before typing
3. **Handle errors gracefully** - Windows might not exist
4. **Use events** - Subscribe to window focus for context

### Example Safe Interaction

```text
User: Can you help me organize my windows?

Agent: I'll list your windows first to see what's open.
→ deskbrid_list_windows()
← 3 windows: Terminal, Firefox, VS Code

Agent: I can tile these for you. Ready?
User: Yes
Agent: Tiling left/right...
→ deskbrid_tile_window(terminal_id, preset="left")
→ deskbrid_tile_window(firefox_id, preset="right")
```

## Troubleshooting

### Agent Can't Connect

1. Verify Deskbrid daemon is running: `deskbrid status`
2. Check socket exists: `ls /run/user/$UID/deskbrid.sock`
3. Restart daemon: `deskbrid daemon --verbose`

### Actions Don't Work

1. Verify desktop dependencies are installed
2. Check the backend supports the action
3. Look at daemon logs for errors

### Permissions Issues

1. On Wayland, ensure ydotoold is running
2. On X11, check xdotool is installed
3. Verify you're in the input group (ydotool)