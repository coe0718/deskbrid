# Changelog

## v0.11.0 — The Durable Desktop HAL

> 42 commits · May 31, 2026 · [Full diff](https://github.com/coe0718/deskbrid/compare/v0.10.0...v0.11.0)

Deskbrid v0.11.0 makes the daemon durable: clipboard history, audit logs, and agent
state survive restarts. It adds the foundation for multi-agent coordination — named
sessions, event-driven rules, and a shared blackboard — plus full NetworkManager
control, macro recording, scheduled actions, TCP mode, and five new desktop features.

---

### 🗄️ Persistence Layer (#84)

The headline feature. Deskbrid now ships a SQLite database at
`~/.local/share/deskbrid/deskbrid.db` with WAL mode for concurrent reads.

**Six active tables, all wired:**

| Table | Wired via |
|---|---|
| `clipboard_history` | `record_clipboard_text()` — every read/write logged |
| `audit_log` | `record_audit_entry()` — every action execution persisted |
| `blackboard` | `blackboard.set/get/delete/list` — shared agent KV store |
| `notifications` | D-Bus interception → SQLite (from #61) |
| `rules` | `rule.create/delete` persistence (from #83) |
| `sessions` | `session.create/destroy` persistence (from #31) |

The `macros` table exists but the macro engine uses file-based storage by design.
An orphaned `cron_jobs` table was removed — the scheduler uses `schedule.json`.

**Impact:** Clipboard history, audit trail, and blackboard state are no longer
lost on daemon restart. Agents can leave sticky state that survives crashes.

---

### 🤝 Multi-Agent Infrastructure

Three new subsystems for agent coordination:

**Named Sessions (#31)** — Each connected agent gets an isolated session with
scoped variables. Create, destroy, list, and switch between sessions. Variables
persist to SQLite. Use case: one agent per workspace, or per task, with clean
state separation.

```
session.create { name, clone_from? }   session.var.set { name, value }
session.destroy { name }               session.var.get { name }
session.list                           session.var.list
session.switch { name }
```

**Rules Engine (#83)** — Event-driven triggers on the subscription bus. Define
rules that fire when a window focuses, clipboard changes, or workspace switches.
Configurable cooldown and max_fires prevent runaway loops. Rules persist to SQLite.

```
rule.create { name, trigger, action_type, action_params, cooldown_ms?, max_fires? }
rule.list | rule.get { rule_id } | rule.delete { rule_id }
rule.pause { rule_id } | rule.resume { rule_id }
```

**Shared Blackboard (#45)** — Namespace-scoped key-value store for agents to
publish and consume shared state. SQLite-backed, survives restarts. No TTL or
subscription events yet — those ship in v0.12.0.

```
blackboard.set { key, value, namespace? }   blackboard.delete { key, namespace? }
blackboard.get { key, namespace? }          blackboard.list { namespace? }
```

---

### 📡 Network & Connectivity

**NetworkManager D-Bus (#62)** — Full rewrite from fragile zbus D-Bus calls to
reliable nmcli subprocess. Every command tested on real hardware (Turtle, EndeavourOS).
WiFi toggle requires polkit authorization; connections and profiles work without
elevation.

```
network.connections.list     network.wifi.enable { enabled }
network.connections.profiles network.wwan.enable { enabled }
network.hotspot.start { ssid, password? }  network.dns.set { dns: [...] }
network.hotspot.stop                       network.dns.reset
network.vpn.connect { profile_name }       network.vpn.disconnect
```

**TCP Mode (#30)** — TCP listener with bearer token auth. Agents on remote machines
or Docker containers can connect via TCP instead of Unix socket. Synthetic UID
for permissions, CLI flags `--tcp-port`/`--tcp-token`.

**D-Bus Raw Access (#28)** — Escape hatch for direct D-Bus calls. When the
structured protocol doesn't cover a service, fall back to raw `dbus.call`.

---

### ⚡ Automation

**Macro Recording & Replay (#25)** — Record action sequences and replay them.
Two replay modes: fast (no delays) and timed (preserves original timing).
Stored as JSON files at `~/.local/share/deskbrid/macros/`.

```
macro.record.start { name }   macro.list
macro.record.stop             macro.get { name }
macro.replay { name, mode? }  macro.delete { name }
macro.export { name }         macro.import { name, data }
```

**Cron Engine (#27)** — Schedule actions at intervals. Reads
`~/.config/deskbrid/schedule.json`, polls every 60 seconds. CLI:
`deskbrid schedule list|add|remove`.

---

### 🖥️ Desktop Features

**Audio Control** — Full PipeWire/PulseAudio integration. List sinks/sources, get/set
volume (per-sink), mute/unmute, set default sink. `audio.list_sinks`,
`audio.set_volume`, `audio.mute`, etc.

**Screen Recording** — `screencast.start { output_path }` and `screencast.stop`.
PipeWire-based capture with real-time events. Companion web dashboard at
`http://localhost:4199` for live preview. Dashboard bound to `0.0.0.0` for LAN access.

**XDG Desktop Portal** — Portal-based screenshots (`portal.screenshot`) and
screencast (`portal.screencast_start/stop`) for sandboxed environments.

**System Tray** — Tray icon with update notifications. Daemon polls GitHub
releases and broadcasts `update.available` events.

**Self-Update (#125)** — `deskbrid self-update` downloads the latest binary from
GitHub releases, replaces the running binary, and restarts the daemon.

**Enlightenment DE** — Detection and basic window management support added.
Desktop environment count now at 9: GNOME, KDE, Hyprland, COSMIC, Sway, Labwc,
XFCE, Budgie, Enlightenment.

---

### 🧹 Code Quality

- **Dead code eliminated:** Orphaned `is_network_action()` function removed,
  stale zbus constants purged, unused `cron_jobs` table dropped from schema
- **Clippy clean:** All warnings fixed, CI enforces `-D warnings`
- **fmt clean:** No formatting violations
- **67 tests pass:** Zero failures, zero ignored, zero filtered out
- **NM refactor:** Replaced fragile zbus D-Bus signature matching with proven
  nmcli subprocess calls — fewer moving parts, easier to debug

---

### 🌐 Website & Docs

- Landing page updated with v0.11.0 feature highlights
- Install script defaults to v0.11.0 with GitHub API fallback
- `CHANGELOG.md` created (you're reading it)
- `ROADMAP.md` updated: #45, #84 marked complete; #62 description fixed
- Nick Launches badge added to footer

---

### 📦 Breaking Changes

None. All 42 commits are additive. Wire protocol is backward-compatible.
Config files, schedule.json, and macro format are unchanged from v0.10.0.

---

[Full commit list on GitHub](https://github.com/coe0718/deskbrid/compare/v0.10.0...v0.11.0)
