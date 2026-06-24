---
name: deskbrid-hermes-mcp
description: Use when the user wants to wire Deskbrid as a Hermes MCP server — start the daemon, register MCP tools, test connectivity, and verify available tool categories. For initial system install, load the devops/deskbrid-install skill first.
version: 1.0.0
author: Tuck
license: MIT
metadata:
  hermes:
    tags: [deskbrid, desktop, linux, mcp, hermes, integration]
    related_skills: [deskbrid-release-checklist]
---

# Deskbrid Hermes MCP Integration

Wire Deskbrid's 100+ Linux desktop control tools into Hermes as an MCP server. After setup, the agent can control windows, keyboard, mouse, clipboard, screenshots, AT-SPI accessibility, terminals, and more.

**Prerequisites:** Deskbrid daemon installed and running. See `devops/deskbrid-install` skill for system installation.

## What This Skill Covers

1. Start the Deskbrid daemon (if not running)
2. Register Deskbrid as a Hermes MCP server
3. Test MCP connectivity
4. Verify available tool categories

## Step 1 — Ensure Daemon Is Running

```bash
pgrep -a deskbrid
```

If no daemon, start it:

```bash
nohup deskbrid daemon --dashboard-bind 0.0.0.0 > /tmp/deskbrid.log 2>&1 &
sleep 2
curl -s -o /dev/null -w "%{http_code}" http://127.0.0.1:20129
```

**`--dashboard-bind 0.0.0.0`** makes the web dashboard accessible from LAN. Use `127.0.0.1` on single-user workstations. The dashboard exposes clipboard history, screenshots, and window titles — only bind to 0.0.0.0 on trusted networks.

## Step 2 — Register MCP Server

```bash
# Check if already registered
hermes mcp list | grep deskbrid

# Register
hermes mcp add deskbrid --command "deskbrid mcp"
```

This adds to `~/.hermes/config.yaml`:

```yaml
mcp_servers:
  deskbrid:
    command: deskbrid
    args:
    - mcp
    timeout: 30
```

## Step 3 — Test

```bash
hermes mcp test deskbrid
```

Expected: `✓ deskbrid: connected`

## Step 4 — Session Restart

MCP servers are loaded at session start. Run `/reset` or start a new `hermes` session. After restart, the agent has 100+ new tools.

## Available Tool Categories

After MCP registration, the agent gains these tool categories:

| Category | Tools | Examples |
|----------|-------|----------|
| **Windows** | list, focus, close, minimize, maximize, move, resize, tile | `list_windows`, `focus_window` |
| **Input** | keyboard typing, key combos, mouse, click, scroll, drag | `type_text`, `press_key`, `mouse_move` |
| **Clipboard** | read, write, history | `clipboard_read`, `clipboard_write` |
| **Screenshots** | capture, OCR, diff | `screenshot`, `screenshot_ocr` |
| **System** | info, battery, idle, power, pressure (PSI) | `system_info`, `system_pressure` |
| **AT-SPI A11y** | tree, get element, click, set value, perform action | `get_accessibility_tree`, `click_element` |
| **Terminal** | create PTY, send input, read output | `terminal_create`, `terminal_send` |
| **Files** | search, read, write, watch | `files_search`, `files_read` |
| **Audio** | volume, mute, sink management | `audio_volume`, `audio_mute` |
| **Network** | WiFi, Bluetooth, connections | `network_status`, `wifi_scan` |
| **MPRIS** | media player control | `mpris_play`, `mpris_pause` |
| **Print** | list printers, print files, manage jobs | `print_file`, `print_list` |
| **Notifications** | send, dismiss, history | `notify_send`, `notify_dismiss` |
| **Keyring** | Secret Service credentials | `secrets_get`, `secrets_store` |
| **Rules** | event-triggered automation | `rule_create`, `rule_list` |
| **Confirmation** | approval-gated destructive actions | `confirm`, `deny` |
| **Agent** | inter-agent mailbox messaging | `agent_send`, `agent_check` |
| **Search** | unified cross-surface search | `search_query` |
| **Macros** | record and replay action sequences | `macro_record`, `macro_replay` |
| **Capabilities** | supported actions, high-risk listing | `capabilities` |

## Troubleshooting

### MCP test fails
```bash
# Verify daemon is running
pgrep -a deskbrid

# Test MCP manually
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | deskbrid mcp
```

### Tools not appearing after /reset
MCP servers load at session start. Verify registration: `hermes mcp list`. If `deskbrid` shows as disabled, run `hermes mcp configure deskbrid` to toggle tools on.

### "Permission denied" on high-risk tools
Deskbrid uses `~/.config/deskbrid/permissions.toml`. High-risk actions (clipboard read, screenshot, keyboard input, process start, terminal create) require explicit allow-listing. Edit the permissions file and restart the daemon.

### Dashboard connection pool full
If the dashboard returns 000 but the daemon is running, the SSE connection pool (max 32) is full. Restart the daemon to clear it.
