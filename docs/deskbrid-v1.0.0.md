# Deskbrid v0.13.0 — Release Documentation

> **Scope:** End-to-end product reference for Deskbrid v0.13.0 — architecture, protocol, API, MCP tooling, Python client, deployment, security model, troubleshooting, and release notes.
> **Audience:** Integrators, agent developers, operators, and contributors working with the v0.13.0 release line.

---

## Table of Contents

1. [Product Overview](#product-overview)
2. [High-Level Architecture](#high-level-architecture)
3. [Protocol Reference](#protocol-reference)
4. [API Reference](#api-reference)
5. [Client SDKs](#client-sdks)
6. [MCP Integration](#mcp-integration)
7. [Security Model](#security-model)
8. [v0.13.0 What's New](#v0130-whats-new)
9. [Configuration](#configuration)
10. [Monitoring & Observability](#monitoring--observability)
11. [Deployment & Operations](#deployment--operations)
12. [Troubleshooting](#troubleshooting)
13. [Related Products & Ecosystem](#related-products--ecosystem)
14. [Current Status & Roadmap](#current-status--roadmap)
15. [Links](#links)

---

## Product Overview

**Deskbrid** is a single Rust binary that exposes Linux desktop control as a JSON-over-Unix-socket protocol. It auto-detects the running desktop environment — GNOME, Hyprland, KDE Plasma, COSMIC, Sway, Niri, Wayfire, Labwc, MATE, Cinnamon, or X11 — and surfaces a uniform API for windows, input, clipboard, screenshots, system state, files, networking, audio, Bluetooth, notifications, and more.

**Problem it solves:** macOS has AppleScript; Windows has UI Automation. Linux has fragmented per-compositor tooling, and Wayland intentionally limits client-side automation. Deskbrid bridges that gap with one protocol and one socket.

**Releases:**
- v0.x: prototype/proof-of-concept with per-compositor backends.
- **v0.13.0:** hardened security, persistent SQLite state, rules engine, per-UID rate limiting, confirmation system, secret/keyring access, agent mailbox, unified search.

**License:** MIT

---

## High-Level Architecture

```
┌─────────────┐   NDJSON   ┌───────────────────────────────────────────────────┐
│   Client 1  │◄──────────►│               deskbrid daemon                     │
└─────────────┘  Unix Sock │                                                   │
                           │  ┌───────────────────────────────────────────┐   │
┌─────────────┐            │  │               DaemonState                  │   │
│   Client 2  │◄──────────►│  │  ┌────────────────────────────────────┐   │   │
└─────────────┘            │  │  │      Backend (trait object)          │   │   │
                           │  │  │  ┌──────────┐  ┌──────────┐       │   │   │
│ Python SDK  │◄──────────►│  │  │  │ GNOME    │  │ Hyprland │ ...  │   │   │
└─────────────┘            │  │  │  ├──────────┤  ├──────────┤       │   │   │
                           │  │  │  │   KDE    │  │   X11    │       │   │   │
│ GNOME Ext.  │◄─── DBus ─►│  │  │  └──────────┘  └──────────┘       │   │   │
└─────────────┘            │  │  │                                   │   │   │
                           │  │  │  Permissions  Events  Audit       │   │   │
┌─────────────┐            │  │  │  RateLimits  DB       Rules      │   │   │
│ MCP Client  │◄──────────►│  │  │  Confirmation Mailbox  Search    │   │   │
└─────────────┘            │  │  └────────────────────────────────────┘   │   │
                           │  └───────────────────────────────────────────┘   │
└──────────────────────────┘  Dashboard: localhost:20129 (SSE)               ┘
```

**Transport:** Unix domain socket at `$XDG_RUNTIME_DIR/deskbrid.sock`.
**Format:** NDJSON (newline-delimited JSON), 1 MiB max per message.
**Security:** Peer identity enforced via `SO_PEERCRED` (Linux kernel-provided UID/PID).

---

## Protocol Reference

This section covers the NDJSON contract for v0.13.0.

### Message Layout

| Direction | `type` | Required fields |
|-----------|--------|------------------|
| Request | Action name (`windows.list`, `clipboard.write`, …) | `id`, optional `dry_run`, `timeout_ms` |
| Response | `"response"` | `id`, `seq`, `status`, `data` or `error` |
| Event | `"event"` | `action_type`, `data`, optional subscription filter |

### Conventions

- `id` is client-chosen; server echoes it.
- `seq` is a per-connection monotonic counter.
- `dry_run` validates permissions, executes no side effects.
- `timeout_ms` overrides the action timeout per request.
- Events are pushed via a `tokio::broadcast` channel (capacity 256).
- Subscriptions support glob patterns (`file.*`, `window.*`, `*`).

### Confirmation Flow

Destructive/high-risk actions can be gated:

1. Client sends `confirmations.request` with `action_type` and optional reason.
2. Daemon returns a confirmation ID if required.
3. Client sends `confirmations.approve` / `confirmations.deny` with that ID.
4. Only after approval, the original action executes.

### Sessions (#31)

Each connection can declare a named session via the `session` field. Session state is persisted to SQLite and reloaded on daemon restart. Sessions carry isolated variables, metadata, and last-active timestamps.

---

## API Reference

This section documents the v0.13.0 actions organized by domain.

### Windows

#### `windows.list`
List open windows.

**Request:**
```json
{"type":"windows.list","id":"1"}
```

**Response:**
```json
{
  "type":"response","id":"1","seq":1,"status":"ok",
  "data":[
    {
      "id":"0x3a0000b",
      "title":"README.md — VS Code",
      "app_id":"code",
      "workspace_id":0,
      "is_focused":true,
      "is_minimized":false,
      "geometry":{"x":0,"y":0,"width":1920,"height":1080},
      "pid":1234
    }
  ]
}
```

#### `windows.focus`
Focus a window.

**Request:**
```json
{"type":"windows.focus","id":"2","window_id":"0x3a0000b"}
```

**Response:**
```json
{"type":"response","id":"2","seq":2,"status":"ok","data":{"focused":"0x3a0000b"}}
```

#### `windows.close`
Request close.

**Request:**
```json
{"type":"windows.close","id":"3","window_id":"0x3a0000b"}
```

**Response:**
```json
{"type":"response","id":"3","seq":3,"status":"ok","data":{"closed":"0x3a0000b"}}
```

#### `windows.tile`
Tile to screen regions.

**Request:**
```json
{"type":"windows.tile","id":"4","window_id":"0x3a0000b","region":"north-west"}
```

#### `windows.activate_or_launch`
Focus by selector, or launch by command/desktop-file.

**Request:**
```json
{"type":"windows.activate_or_launch","id":"5","selector":{"app_id":"code"},"launch_command":"code"}
```

### Workspaces

#### `workspaces.list`
List workspaces.

**Request:**
```json
{"type":"workspaces.list","id":"6"}
```

**Response:**
```json
{
  "type":"response","id":"6","seq":4,"status":"ok",
  "data":[{"id":0,"name":"1"},{"id":1,"name":"2"}]
}
```

#### `workspaces.switch`
Switch workspace.

**Request:**
```json
{"type":"workspaces.switch","id":"7","workspace_id":1}
```

### Input

#### `input.keyboard.type`
Type text.

```json
{"type":"input.keyboard.type","id":"8","text":"Hello"}
```

#### `input.keyboard.key`
Press a key by key name.

```json
{"type":"input.keyboard.key","id":"9","key":"Return"}
```

#### `input.keyboard.combo`
Send key combination.

```json
{"type":"input.keyboard.combo","id":"10","keys":["Ctrl","l"]}
```

#### `input.mouse.move` / `.click` / `.scroll`

```json
{"type":"input.mouse.move","id":"11","x":500,"y":300}
{"type":"input.mouse.click","id":"12","button":"left","x":500,"y":300}
{"type":"input.mouse.scroll","id":"13","direction":"up"}
```

### Clipboard

#### `clipboard.read`
```json
{"type":"clipboard.read","id":"14"}
```

**Response:**
```json
{"type":"response","id":"14","seq":5,"status":"ok","data":{"text":"..."}}
```

#### `clipboard.write`
```json
{"type":"clipboard.write","id":"15","text":"..."}
```

#### `clipboard.history`
```json
{"type":"clipboard.history","id":"16"}
```

### Screenshot

#### `screenshot`
```json
{"type":"screenshot","id":"17","monitor":0,"region":{"x":0,"y":0,"width":800,"height":600}}
```

**Response:**
```json
{"type":"response","id":"17","seq":6,"status":"ok","data":{"path":"/tmp/deskbrid/screenshot_<timestamp>.png","mime":"image/png"}}
```

#### `screenshot.ocr`
```json
{"type":"screenshot.ocr","id":"18","image_path":"/tmp/deskbrid/screenshot_<timestamp>.png"}
```

### System

#### `system.info`
```json
{"type":"system.info","id":"19"}
```

Returns desktop, backend, version, monitors, and capabilities.

#### `system.battery`
```json
{"type":"system.battery","id":"20"}
```

#### `system.health`
```json
{"type":"system.health","id":"21"}
```

Per-backend dependency checks.

#### `system.remediate`
```json
{"type":"system.remediate","id":"22","check":"ydotoold"}
```

Auto-fix known issues.

### Notifications

#### `notification.send`
```json
{"type":"notification.send","id":"23","summary":"Hello","body":"world"}
```

### Network & Bluetooth

#### `network.status`
```json
{"type":"network.status","id":"24"}
```

#### `bluetooth.list`
```json
{"type":"bluetooth.list","id":"25"}
```

### Files

#### `files.search`
```json
{"type":"files.search","id":"26","query":"*.rs","root":"/home/coemedia/projects"}
```

#### `files.watch` / `.unwatch`
Subscribe/unsubscribe to file events.

```json
{"type":"files.watch","id":"27","path":"/home/coemedia/projects"}
```

### Audio

#### `audio.list_sinks`
```json
{"type":"audio.list_sinks","id":"28"}
```

### Monitor

#### `monitor.list`
```json
{"type":"monitor.list","id":"29"}
```

### Confirmations

#### `confirmations.request`
```json
{"type":"confirmations.request","id":"30","action_type":"windows.close","window_id":"0x3a0000b"}
```

#### `confirmations.approve`
```json
{"type":"confirmations.approve","id":"31","confirmation_id":"confirm-1"}
```

### Agent Messaging

#### `agent.mailbox.send`
```json
{"type":"agent.mailbox.send","id":"32","to":"agent-b","subject":"deploy-ready","body":"v1.0.0 verified"}
```

### Rules Engine (v0.13.0)

#### `rules.list`
```json
{"type":"rules.list","id":"33"}
```

#### `rules.trigger`
```json
{"type":"rules.trigger","id":"34","rule_id":"rule-1","payload":{"source":"ci"}}
```

### Search (v0.13.0)

#### `search.query`
```json
{"type":"search.query","id":"35","query":"deskbrid","surfaces":["windows","files"]}
```

---

## Client SDKs

### Python Client

```python
from deskbrid import Deskbrid

client = Deskbrid()

windows = client.windows_list()
client.window_focus(id=windows[0]["id"])
client.keyboard_type("v1.0.0 release notes\n")

# Event subscription
@client.on("file.*")
def on_file(event):
    print(event["path"])
```

### CLI

```bash
deskbrid daemon
deskbrid windows list
deskbrid clipboard read
deskbrid screenshot
deskbrid system info
```

---

## MCP Integration

```bash
deskbrid mcp
```

**Claude Desktop config:**
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

MCP exposes window management, accessibility tree, keyboard/mouse, clipboard, screenshot, system, print, confirmation, agent mailbox, and search tools. The exact tool list is generated from the protocol at `src/mcp/`.

---

## Security Model

Deskbrid v0.13.0 uses layered controls:

1. **Unix socket + SO_PEERCRED**: peer UID/PID authenticated by the kernel.
2. **Permissions file** (`~/.config/deskbrid/permissions.toml`): per-UID allow/deny, glob matching.
   - **Default: deny-all if file is present but invalid.**
3. **High-risk action gating**: destructive actions require confirmation (unless pre-approved).
4. **Rate limiting**: per-namespace per-UID token buckets; responds with `retry_after_ms`.
5. **Audit trail**: every action is logged to SQLite and in-memory.
6. **File sandbox**: path resolution blocks directory traversal and escapes from user-owned paths.
7. **PID protection**: PIDs 0, 1, and the daemon PID are rejected.
8. **Secret/keyring access**: additional confirmation gating on Secret Service reads.

### permissions.toml Reference

```toml
[permissions.uid:1000]
allow = ["*"]

[permissions.uid:1001]
allow = ["windows.*", "clipboard.read"]
deny = ["screenshot"]

[rate_limits]
# namespace = { rpm = 120, burst = 20 }
```

---

## v0.13.0 What's New

**Persistent SQLite state** — the source of truth for audit log, clipboard history, sessions, schedules, rules, macros, and mail.

**Rules engine** — time-based timers, variable conditions, variable-based actions, app_id resolution when targeting windows, no stubs.

**Secret/keyring service** — read back credentials via `secret-tool` / Secret Service; gated by confirmation.

**Per-namespace rate limiting** — token buckets per namespace, per UID, with configurable overrides.

**Platform support** — robust dispatch for GNOME, KDE, Hyprland, COSMIC, Sway, Niri, Wayfire, Labwc, and shared X11.

---

## Configuration

### Runtime locations

| Path | Purpose |
|------|---------|
| `$XDG_RUNTIME_DIR/deskbrid.sock` | Unix socket |
| `~/.config/deskbrid/permissions.toml` | Permissions + rate limits |
| `~/.config/deskbrid/layout_profiles/` | Saved layouts |
| `~/.config/deskbrid/deskbrid.db` | SQLite database (audit, sessions, rules, …) |
| `~/.config/systemd/user/deskbrid.service` | systemd user unit |

### Environment

| Variable | Effect |
|-----------|--------|
| `DESKBRID_AUDIT_CAPACITY` | Max in-memory audit entries |
| `DESKBRID_ACTION_TIMEOUT_MS` | Default action timeout |
| `DESKBRID_CLIPBOARD_HISTORY_CAPACITY` | Max stored clipboard entries |

---

## Monitoring & Observability

- **Structured logs**: `tracing`-based output; filter by module.
- **Audit log**: in-memory ring + SQLite; `system.health`/`system.remediate` expose diagnostics.
- **Dashboard**: `localhost:20129` with SSE-connected live pages and cards for system info, monitors, windows, network, audio, clipboard history, confirmations, agent mailbox, and search index.
- **Rate limit signals**: `RATE_LIMITED` responses include `retry_after_ms`.
- **Captive dependencies**: `system.remediate` can auto-fix known unhealthy dependencies (e.g., `ydotoold`).

---

## Deployment & Operations

### Quick install

```bash
bash <(curl -fsSL https://deskbrid.patchhive.dev/install.sh)
```

### systemd

```bash
cp deploy/deskbrid.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now deskbrid
```

### Update

```bash
deskbrid update
```

### GNOME-specific setup

```bash
deskbrid setup
```

Installs `grim`, `wl-clipboard`, `python3-gi`, `gstreamer1.0-tools`, `gstreamer1.0-pipewire`.

### Wayland compositor prerequisites

- `ydotool` + `ydotoold`
- `/dev/uinput` access: `KERNEL=="uinput", GROUP="input", MODE="0660"` + `usermod -aG input $USER`
- Notification daemon (`dunst`, `mako`, or `swaync`)

---

## Troubleshooting

### Daemon not starting
- Check `systemd --user status deskbrid`
- Inspect logs: `journalctl --user -u deskbrid -n 100`
- Confirm socket path matches `$XDG_RUNTIME_DIR`

### Permission denied
- Verify `~/.config/deskbrid/permissions.toml` parse; invalid TOML causes deny-all.
- Confirm UID matches the rule set.
- If using UID rules, ensure the client connects as the intended Linux user.

### Input not working on Wayland
- Confirm `ydotoold` is running.
- Confirm GNOME Shell extension is installed and active (GNOME only).
- For KDE: confirm `ydotool` backend access and `ydotoold` running.
- `system.remediate` can restart or re-enable `ydotoold`.

### Screenshot empty/black
- Use PipeWire/ScreenCast portal helpers where available.
- Try `screenshot.diff` or `screenshot.ocr` to confirm stream validity.

### Rate limited unexpectedly
- `RATE_LIMITED` responses include `retry_after_ms`.
- Check `permissions.toml` rate limit section.

---

## Related Products & Ecosystem

- **PatchHive** — parent ecosystem positioning Deskbrid as desktop bridge.
- **GNOME Shell Extension** — enables window/workspace control via D-Bus for GNOME backend.
- **Python Client** — synchronous/async SDK for agent scripts.
- **XDG Desktop Portal helpers** — fallback screenshot and capture helpers for restrictive desktops.

---

## Current Status & Roadmap

**Current release:** v0.13.0

**Key subsystems in this release:**
- Desktop backends: GNOME, KDE, Hyprland, Sway, Niri, Wayfire, Labwc, X11.
- Persistence: SQLite-backed audit, sessions, clipboard history, rules, macros, search index.
- Rules engine: active.
- Confirmations + secret service: active.
- Per-UID rate limiting: active.

**In development / next:**
- Monitor topology restore improvements.
- Additional D-Bus-based backend coverage.
- Dashboard extensions.
- Portal-based screencast pipeline hardening.

For detailed item tracking, see `ROADMAP.md`.

---

## Links

- **Docs site:** <https://deskbrid.patchhive.dev>
- **Repository:** <https://github.com/coe0718/deskbrid>
- **Live dashboard demo:** <https://deskbrid.patchhive.dev/live>
- **Install script:** <https://deskbrid.patchhive.dev/install.sh>
