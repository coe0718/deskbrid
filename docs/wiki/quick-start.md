# Quick Start

Get Deskbrid running in 10 minutes.

## 1. Install

```bash
bash <(curl -fsSL https://deskbrid.patchhive.dev/install.sh)
```

Recommended for most systems: auto-detects distro + desktop environment, installs dependencies, configures `uinput`, and installs the latest binary.

Manual install is also available from the [releases page](https://github.com/coe0718/deskbrid/releases).

## 2. Start the daemon

```bash
deskbrid daemon
```

If you want it to start with your session, use the included systemd user unit:

```bash
cp ../../deploy/deskbrid.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now deskbrid
```

## 3. Run common commands

```bash
# List windows
deskbrid windows list

# Read the clipboard
deskbrid clipboard read

# Take a screenshot
deskbrid screenshot

# Get system info
deskbrid system info
```

## 4. Try agent control

```bash
# Focus a window
deskbrid windows focus --app code

# Type text
deskbrid input keyboard type "Hello, world!\n"

# Send a notification
deskbrid notify send "Deskbrid" "Setup complete!"
```

## 5. Use structured control

```bash
# Inspect the accessibility tree
deskbrid a11y tree --app code --max-depth 2

# Jump to an element
deskbrid a11y do --element "Submit" --action click

# Dump desktop settings for the current desktop
deskbrid settings dump
```

## 6. Connect AI tools

For Claude Desktop, Cursor, Codex, or another MCP client:

```bash
deskbrid mcp
```

Configure the client with:

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

## Next steps

- [Features](INDEX.md#features) — feature map with links
- [Protocol](protocol/overview.md) — JSON-over-socket protocol
- [Python Client](integrations/python.md) — Python SDK
- [AI Agents](integrations/agents.md) — agent integration patterns
