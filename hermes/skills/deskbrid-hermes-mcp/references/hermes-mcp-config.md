# Hermes MCP Config for Deskbrid

## Config Format

After running `hermes mcp add deskbrid --command "deskbrid mcp"`, Hermes adds this to `~/.hermes/config.yaml`:

```yaml
mcp_servers:
  deskbrid:
    command: deskbrid
    args:
    - mcp
    timeout: 30
```

## Verification

```bash
# Check it's registered
hermes mcp list | grep deskbrid

# Test connectivity
hermes mcp test deskbrid

# Expected output:
# ✓ deskbrid: connected (100+ tools)
```

## Manual Registration

```bash
hermes config set mcp_servers.deskbrid.command deskbrid
hermes config set mcp_servers.deskbrid.args.0 mcp
hermes config set mcp_servers.deskbrid.timeout 30
```

## Removing

```bash
hermes mcp remove deskbrid
```
