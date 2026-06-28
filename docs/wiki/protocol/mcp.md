# MCP Integration

Deskbrid v1.0.0 can expose desktop control through the Model Context Protocol.
Clients connect via stdio or TCP and invoke the same dot-notation actions used
over the Unix socket.

## Server entrypoints

```bash
# Stdio mode for AI clients
deskbrid mcp

# TCP mode
deskbrid daemon --mcp-port 20129
# then clients connect to tcp://localhost:20129
```

## Tool map

MCP tools follow the daemon's action namespace. Each tool maps to one action.

| MCP tool name | Action |
|---|---|
| `windows.list` | `windows.list` |
| `windows.focus` | `windows.focus` |
| `windows.get` | `windows.get` |
| `windows.close` | `windows.close` |
| `windows.tile` | `windows.tile` |
| `windows.activate_or_launch` | `windows.activate_or_launch` |
| `input.keyboard` | `input.keyboard` |
| `input.mouse` | `input.mouse` |
| `clipboard.read` | `clipboard.read` |
| `clipboard.write` | `clipboard.write` |
| `clipboard.history` | `clipboard.history` |
| `screenshot` | `screenshot` |
| `screenshot.ocr` | `screenshot.ocr` |
| `screenshot.diff` | `screenshot.diff` |
| `notification.send` | `notification.send` |
| `notification.close` | `notification.close` |
| `monitors.list` | `monitors.list` |
| `monitors.set` | `monitors.set` |
| `service.list` | `service.list` |
| `service.start` | `service.start` |
| `service.stop` | `service.stop` |
| `timer.list` | `schedule.list` |
| `timer.add` | `schedule.add` |
| `timer.remove` | `schedule.remove` |
| `system.info` | `system.info` |
| `system.capabilities` | `system.capabilities` |
| `system.power` | `system.power` |
| `system.idle` | `system.idle` |
| `rules.list` | `rules.list` |
| `rules.create` | `rules.create` |
| `rules.trigger` | `rules.trigger` |
| `secrets.set` | `secrets.set` |
| `secrets.get` | `secrets.get` |
| `query_records` | `query_records` |

> **Note:** This table shows a representative subset of ~30 commonly-used tools.
> For the complete list of 100+ MCP tools with full parameter schemas, see the
> [feature documentation](./features/) or the [API reference](../../API.md).

## Configuration

Claude Desktop, `claude_desktop_config.json`:

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

Cursor, `.cursor/mcp.json`:

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

## Example AI session

```text
Agent: Let me see what windows are open.
→ {"type":"windows.list"}
← [{"id":"12345678","title":"VS Code","app_id":"code",...}]

Agent: Focus VS Code and open test file.
→ {"type":"input.keyboard","action":"type","text":"code src/main.rs\\n"}
← {"type":"response","status":"ok"}
```

## Security

- Actions that require confirmation flow through `confirm.challenge` /
  `confirm.resolve` before dispatches are executed.
- Audit all agent actions via the dashboard or `audit_log`.
- The daemon enforces auth policy independently of the client.
