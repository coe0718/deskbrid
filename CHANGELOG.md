# Changelog

## v0.11.0 — Persistence & Multi-Agent Foundation

42 commits since v0.10.0. This release makes Deskbrid durable — data survives
daemon restarts — and adds the infrastructure for multi-agent coordination.

### Persistence Layer (#84)
- SQLite database at `~/.local/share/deskbrid/deskbrid.db` (WAL mode, 6 tables)
- **Clipboard history** persists across restarts — every read/write logged
- **Audit log** persists every action execution to SQLite
- **Blackboard** — shared KV store for agent coordination (`blackboard.set/get/delete/list`)
- Tables: clipboard_history, audit_log, notifications, macros, blackboard, sessions

### Multi-Agent Infrastructure
- **Named Sessions (#31)** — per-agent session isolation with scoped variables and persistence
- **Rules Engine (#83)** — event-driven triggers on subscription bus, cooldown/max_fires, persisted
- **Blackboard (#45)** — agents can publish/consume shared state via `blackboard.*` commands

### Notification History (#61)
- D-Bus notification interception stored to SQLite
- Query with filters (`notification.history { limit, app_name }`)
- Notification watch subscription for real-time event feed

### NetworkManager (#62)
- nmcli-backed: connection profiles, hotspot create/stop, WiFi/WWAN toggle
- DNS set/reset, VPN connect/disconnect
- Tested on Turtle (EndeavourOS)

### Automation
- **Macro Recording & Replay (#25)** — record action sequences, replay with fast/timed modes
- **Cron Engine (#27)** — scheduled actions via `~/.config/deskbrid/schedule.json`

### Connectivity
- **TCP Mode (#30)** — TCP listener with bearer token auth for remote agents
- **D-Bus Raw Access (#28)** — escape hatch for direct D-Bus calls
- **XDG Desktop Portal** — portal-based screenshots and screencast

### Desktop Support
- **Enlightenment DE** detection added
- **XFCE** X11 DISPLAY auto-detection
- **Audio** — full PipeWire/PulseAudio control (list sinks/sources, volume, mute)
- **Screen Recording** — `screencast.start/stop` with PipeWire + web dashboard preview

### Developer Experience
- **Self-update (#125)** — `deskbrid self-update` downloads latest from GitHub
- **System tray icon** with update notifications
- **Web dashboard** — bound to `0.0.0.0` for LAN access
- **Update check** — daemon polls GitHub releases, broadcasts `update.available` events

### Internal
- All 7 persistence tables have CRUD methods; clipboard, audit, blackboard wired
- Dead code eliminated: orphaned `cron_jobs` table removed, stale zbus code purged
- 67 tests pass, clippy + fmt clean (`-D warnings`)
- `is_network_action` dead code removed, NM implementation 100% nmcli

### Breaking Changes
- None. All new features are additive. Wire protocol is backward-compatible.
