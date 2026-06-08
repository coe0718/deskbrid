# AI Agents

Deskbrid v1.0.0 exposes desktop control to AI coding agents through the MCP
server, or directly by sending actions over the daemon socket. The daemon is
the source of truth for available actions; see `docs/api-docs.md` for the
canonical action map.

## MCP server

### Stdio mode (AI tools)

```bash
deskbrid daemon
deskbrid mcp
```

Clients connect over stdin/stdout.

### TCP mode

```bash
deskbrid daemon --mcp-port 20129
```

Clients connect to `tcp://localhost:20129`.

## Client configs

Claude Desktop:

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

Cursor:

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

## Agent safety

- Elevated or sensitive actions flow through the dashboard challenge /
  confirmation UI unless explicitly allowed by policy.
- Use the `require_confirmation` / confirmation flow to review uncertain agent
  actions before dispatch.
- Policy enforcement mirrors socket authentication; the safer path is via
  `confirm.challenge` / `confirm.resolve`, not by passing extra permissions over
  the wire.

## Audit trail

Review agent actions via the dashboard or audit log:

```bash
deskbrid audit_log
```
