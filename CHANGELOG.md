## v0.12.3 — OCI Docker + MCP Registry

**Tuck · Dockerfile label + ghcr.io push + OCI package**

Switch from MCPB (broken async verification) to OCI. Docker image pushed
to ghcr.io with `io.modelcontextprotocol.server.name` label for ownership
verification. Release workflow builds and pushes Docker on every tag.

## v0.12.2 — MCP Registry Publishing

**Tuck · 1 commit · 3 files · server.json + publish workflow**

Publish Deskbrid to the official MCP Registry. Rename release tarballs to include
"mcp" for MCPB allowlist compatibility. Auto-compute SHA-256 in publish workflow.

## v0.12.1 — Async Safety + Path Sandbox

**2 commits (content) · 24 files changed · 205 insertions · 137 deletions**

Bugfix release. All blocking `std::fs` and `std::process::Command` calls in async
execution paths converted to `tokio::fs`/`tokio::process::Command`. Path sandbox
applied to print-file. Claude code review catch — three issues found in v0.12.0.

### 🔒 Security
- **c77a266** — `print_file` path sandbox: apply `expand_path()` before CUPS call.
  Prevents agents from printing any file on the system (`/etc/shadow`, SSH keys, etc.).
  One-line fix — the sandbox was already built, just needed to be hooked up.

### ⚡ Async Safety (30+ blocking calls eliminated)
- **c77a266** — `print.rs`: 10 `std::process::Command` → `tokio::process::Command`.
  All CUPS CLI wrappers now properly async.
- **c77a266** — `backlight.rs`: `std::fs::read_dir`/`read_to_string`/`write` → `tokio::fs`.
- **0eeac95** — `macro_engine.rs`: load/save/delete all async (4 I/O calls + 6 fn signatures
  cascaded to async)
- **0eeac95** — `schedule.rs`: `Schedule::load()`/`save()` async (3 calls)
- **0eeac95** — `labwc/keyboard.rs`: `read_env_file()`/`write_env_file()` async (4 calls +
  10 call sites)
- **0eeac95** — `kde/helpers.rs` + `gnome/inner.rs`: `probe_drm_monitors` → tokio fs (4 calls)
- **0eeac95** — `kde/desktop_settings.rs`: `desktop_list_schemas` → tokio fs
- **0eeac95** — `dashboard/server.rs`: screenshot reads → tokio fs (2 calls)
- **0eeac95** — `execute_screenshot.rs`: file copy → tokio fs
- **0eeac95** — `hyprland/free_functions.rs` + `gnome/screenshot/crop.rs` + `capture.rs`:
  `get_png_dimensions` async conversion (3 calls across 3 files)

`expand_path` (3 `std::fs::canonicalize` calls, 24 call sites) left as follow-up.

---

## v0.12.0 — Print, Desktop Settings, Backlight & Dashboard

**17 commits since v0.11.3 · 63 files changed · 1,990 insertions · 227 deletions**

Major feature release. CUPS printing pipeline, desktop settings read/write across
all 9 backends, backlight control via sysfs, and three new live dashboard cards.

### 🖨️ CUPS Printing (`system.print`)
- **bec4e2b** — `system.print_file` action: send any file to a CUPS printer.
  Full stack: protocol parse/serialize, backend trait, CUPS CLI impl, MCP tool,
  CLI subcommand, execute dispatch, stubs. The command literally didn't exist before.
- **996b8ee** — CLI: `deskbrid system print-list`, `print-file`, `print-cancel`,
  `print-pause`, `print-resume`, `print-set-default`. Graceful fallback when CUPS absent.
- **c7592aa** — Dashboard: Printers card with SSE live updates (name, status, jobs, default marker)

### ⚙️ Desktop Settings (`system.desktop_settings`)
- **6987545** — `desktop_settings.get/set/list` across all 9 backends:
  GNOME (gsettings), KDE (kreadconfig5/kwriteconfig5), Hyprland/Sway/Niri/Wayfire (config files),
  COSMIC (gsettings), Labwc (config files), X11 (xsettings)
- **5474174** — Fix serialization: proper `schema/key/value` field structure
- **9814003** — Clippy: drop needless borrow in KDE desktop_settings

### 💡 Backlight (`system.backlight`)
- **5c637c4** — `backlight.list/get/set` via sysfs `/sys/class/backlight/*`
- **6b0b645** — Install script: udev rule adds user to `video` group for sysfs access
- **1f5219b** — Dashboard: Backlight card with brightness slider

### ⌨️ Keyboard Layouts
- **96a611d** — Add layout support for Niri and Wayfire backends (completing coverage)

### 📊 Dashboard
- **1f5219b** — Desktop Settings card: view/edit DE configuration live
- **1f5219b** — Backlight card: per-display brightness with percentage bars
- **c7592aa** — Printers card: printer status, default badge, active job count
- All cards follow same pattern: render fn → SSE dispatch → volatile_cards → HTML template

### ♿ Accessibility
- **014d869** — AT-SPI2: wire all 12 action variants (was 8). Bypasses backend check so
  AT-SPI2 actions work regardless of detected DE.

### 🔧 Housekeeping
- **d212070** — CI: `cleanup-runs` targets all workflows via explicit token+repo
- **6008d23** — Remove duplicate a11y impl from execute_stubs
- **18563c0** — Checklist sweep: MCP docs + Hermes plugin for new features

## v0.11.3 — Async Safety & Code Quality

**17 commits since v0.11.2 · 15 files changed · 646 insertions · 619 deletions**

Patch release fixing blocking I/O calls, adding checksum verification to the
self-updater, and splitting 6 oversized files into 21 modules. Every source
file now under 300 lines.

### ⚡ Async Safety
- **6 commits** — Replace `std::fs::read_dir` with `tokio::fs::read_dir` across
  macro_engine, cosmic backend, X11 backend, GNOME DRM probe, KDE DRM probe, and
  the self-update binary scan. Eliminates tokio worker thread stalls on I/O.
- `abs_pointer.rs:191` intentionally left sync — documented `spawn_blocking` pattern.

### 🔒 Self-Update Security
- Replace `sha256sum` shell call with `sha2` crate — in-Rust verification that
  downloads `.sha256` from the release, computes the hash, and compares before
  any binary replacement.
- CI release workflow now generates `.sha256` files alongside tarballs.

### 📦 File Splits (Claude Code Review)
| File | Before | After |
|------|--------|-------|
| `cosmic_helper.rs` | 1173 | 6 modules (27–285) |
| `portal.rs` | 452 | 4 modules (12–207) |
| `tray.rs` | 417 | 3 modules (7–282) |
| `kde/networking.rs` | 381 | 3 modules (8–295) |
| `gnome/screenshot.rs` | 368 | 5 modules (6–134) |
| `labwc_helper.rs` | 330 | 5 modules (13–184) |

### 🐛 Bug Fixes
- `ActiveScreencast` visibility — type was `pub(crate)` but `DaemonState` field
  is `pub`. Fixed all the way through: struct, functions, and re-exports.
- `clippy::needless_return` in GNOME screenshot fallback path.

### 🔧 CI
- Add `workflow_dispatch` trigger — GitHub didn't fire on two pushes, now we
  can manually kick it.

## v0.11.2 — Repo Cleanup & Live Demo

**12 files changed · 58 insertions · 741 deletions · 5 commits since v0.11.1**

Housekeeping release. Dist zip nuked, stale design docs removed, version refs
fixed across agent files, live dashboard proxied publicly, README updated.

### 🧹 Repo Hygiene
- **`9d578a4`** — Remove `dist/deskbrid-gnome-extension-v0.4.1.zip` (binary artifact, `dist/` already gitignored)
- **`88542b5`** — Delete `docs/MCP_ATSPI_DESIGN.md` (shipped months ago), `docs/TESTING_NEEDED.md` (stale test tracker)
- **`9d578a4`** — Fix `CLAUDE.md` download URL: hardcoded `v0.10.0` → `releases/latest/download/deskbrid`
- **`9d578a4`** — Fix `hermes/deskbrid.md`: title `v0.10.0` → `v0.11.1`, highlights `v0.7.0` → `v0.11.1` features

### 📡 Dashboard & Site
- **`b78625b`** — Proxy `/live`, `/events`, `/screenshot` through Caddy to Turtle's live dashboard
- **`b78625b`** — Add Live Demo nav link to `site/index.html` with green pulse animation
- **`b33cabd`** — Add `🔴 Live Demo →` link above dashboard screenshot in README

### 📋 Agent Files
- **`2434ac8`** — Rewrite `AGENTS.md` from 23-line skeleton to 52-line usage-focused landing page (features, dashboard, MCP, Python client, supported desktops, self-update)

### 🐛 Bug Fixes
- **`e48c050`** — Audio volume shows 0%: Hyprland backend now parses full `pactl list sinks` output; SSE parser handles `%` sign in volume values

---

## v0.11.1 — Audio Fix & Dashboard Polish

**7 commits since v0.11.0**

Quick patch on top of v0.11.0. Audio volume was hardcoded to 0% in the Hyprland
backend and the SSE parser choked on the `%` sign from `pactl`. Dashboard
got a screenshot and README polish.

### 🐛 Bug Fixes
- **`e48c050`** — Audio volume: Hyprland backend switched from `pactl list short sinks` (no volume field) to full `pactl list sinks`; SSE handler applies `trim_end_matches('%')` before parsing

### 📡 Dashboard
- **`b78625b`** — Live dashboard proxied at `deskbrid.patchhive.dev/live`
- **`5cbe515`** — Dashboard screenshot added to README

---

## v0.11.0 — The Durable Desktop HAL

**94 files changed · 7,832 insertions · 259 deletions · 42 commits since v0.10.0**

Deskbrid stops being ephemeral. Clipboard history, audit trails, and agent state
survive daemon restarts via SQLite. Multi-agent coordination arrives: named
sessions, event-driven rules, and a shared blackboard. NetworkManager goes
nmcli-native. Plus macros, cron, TCP mode, audio, screencast, self-update, and
a system tray.

---

### 🗄️ Persistence Layer (#84) — 694 lines

SQLite database at `~/.local/share/deskbrid/deskbrid.db` with WAL mode.
`src/daemon/persistence.rs` — 694 lines, 26 public methods, 6 active tables.

| Table | Status | Wired via |
|---|---|---|
| `clipboard_history` | ✅ | `record_clipboard_text()` — fire-and-forget on every read/write |
| `audit_log` | ✅ | `record_audit_entry()` — every action, success or failure |
| `blackboard` | ✅ | `blackboard.set/get/delete/list` — 62-line executor |
| `notifications` | ✅ | D-Bus interception → SQLite (from #61) |
| `rules` | ✅ | `rule.create/delete` persistence (from #83) |
| `sessions` | ✅ | `session.create/destroy` persistence (from #31) |
| `macros` | — | Table exists, engine uses file-based storage by design |
| `cron_jobs` | ☠️ | Removed — scheduler uses `schedule.json` |

**`4516639`** — 12 files, 205 additions. The big wiring commit. Clipboard,
audit, and blackboard were all schema-only before this. Now they're live.

---

### 🤝 Multi-Agent Infrastructure

**Named Sessions (#31) — 160 lines** (`a28a601`)
Per-agent isolation with scoped variables. Each connection gets a session;
sessions have independent variable namespaces. Variables survive restarts
via SQLite.

```
session.create { name, clone_from? }   session.var.set { name, value }
session.destroy { name }               session.var.get { name }
session.list                           session.var.list
session.switch { name }
```

**Bug fix** (`85d9c34`): `SessionVarSet` was using variable name as the session
lookup key. Now correctly looks up by `session_id`.

**Rules Engine (#83) — 315 lines** (`d1d23c0`)
Event-driven triggers on the subscription bus. Define rules that fire on
window focus, clipboard change, or workspace switch events. Configurable
cooldown and max_fires prevent runaway loops.

```
rule.create { name, trigger, action_type, action_params, enabled, cooldown_ms?, max_fires? }
rule.list | rule.get { rule_id } | rule.delete { rule_id }
rule.pause { rule_id } | rule.resume { rule_id }
```

`src/daemon/rules.rs` — 315 lines: rule engine with event matching, cooldown
tracking, fire counting, and background evaluation task. `src/daemon/execute_rules.rs`
— 164 lines: socket command handler with full CRUD + pause/resume.

**Shared Blackboard (#45) — 62 lines** (`4516639`)
Namespace-scoped KV store. SQLite-backed via `upsert_blackboard()` /
`get_blackboard()` / `delete_blackboard()` / `blackboard_keys()` from the
persistence layer.

```
blackboard.set { key, value, namespace? }   blackboard.delete { key, namespace? }
blackboard.get { key, namespace? }          blackboard.list { namespace? }
```

No TTL, exclusive locks, or subscription events yet — those are v0.12.0.

---

### 📡 Network & Connectivity

**NetworkManager (#62) — refactored to 286 lines** (`a2ed848`)
Complete rewrite: replaced fragile zbus D-Bus signature matching with nmcli
subprocess calls. 471 lines deleted, 239 added. The zbus implementation had
signature mismatches on `Properties.Get` variant wrapping and `ObjectPath`
deserialization — nmcli sidesteps all of it. Tested on Turtle (EndeavourOS).

```
network.connections.list     network.hotspot.start { ssid, password? }
network.connections.profiles network.hotspot.stop
network.wifi.enable { enabled }           network.dns.set { dns: [...] }
network.wwan.enable { enabled }           network.dns.reset
network.vpn.connect { profile_name }      network.vpn.disconnect
```

WiFi toggle requires polkit authorization — returns permission denied on Turtle.
Connections and profiles work without elevation.

**SessionVarSet fix** (`85d9c34`): Key lookup was broken — using variable name
instead of session ID. Found and fixed during Turtle testing.

**Dead code cleanup** (`a41ceaa`): `is_network_action()` — 26 lines, zero
callers, had `#[allow(dead_code)]`. Removed. Stale "hybrid zbus + nmcli"
header comment fixed to reflect 100% nmcli implementation.

**TCP Mode (#30) — 143 lines** (`7e0c8bd`)
TCP listener with bearer token auth. Agents on remote machines or Docker
containers connect via TCP instead of Unix socket. CLI flags `--tcp-port`
and `--tcp-token`. Synthetic UID for permissions. 349 lines total including
Python client updates.

**D-Bus Raw Access (#28) — 78 lines** (`a9a97b1`)
Escape hatch for direct D-Bus calls when the structured protocol doesn't
cover a service. `dbus.call { bus, service, path, interface, method, args? }`.
Added to high-risk permission gate.

---

### ⚡ Automation

**Macro Recording & Replay (#25) — 319 + 123 lines** (`f89273a`)
Record action sequences as JSON and replay them. Two modes: fast (no delays)
and timed (preserves original timing). Stored at `~/.local/share/deskbrid/macros/`.

`src/daemon/macro_engine.rs` — 319 lines: recording state machine, file I/O,
replay engine with mode selection. `src/daemon/execute_macro.rs` — 123 lines:
socket command handler. Protocol parser at `src/protocol/parse/macro_cmd.rs`
— 79 lines.

```
macro.record.start { name }   macro.list
macro.record.stop             macro.get { name }
macro.replay { name, mode? }  macro.delete { name }
macro.export { name }         macro.import { name, data }
```

**Cron Engine (#27) — 174 lines** (`a5a4c14`)
Schedule actions at intervals. Reads `~/.config/deskbrid/schedule.json`.
Polls every 60 seconds. Actions dispatched through the same pipeline as
socket requests.

```
schedule.list
schedule.add { name, interval_secs, action_type, action_params? }
schedule.remove { name }
```

---

### 🖥️ Desktop Features

**Screen Recording + Web Dashboard — 514 lines** (`c78bf6f`, `c00c0d2`)
`screencast.start { output_path }` / `screencast.stop`. PipeWire-based capture
via GNOME ScreenCast portal. Real-time events broadcast on subscription bus.
Web dashboard at `http://localhost:4199` — 514 lines of Rust. Bound to
`0.0.0.0` for LAN access (`62a1ecb`). MCP tools for screencast control
(`c00c0d2`).

**XDG Desktop Portal — 178 lines** (`3ab61cc`)
`portal.screenshot` and `portal.screencast_start/stop`. Portal-based capture
for sandboxed environments (Flatpak, Snap). Full Rust implementation — no
shelling out to `gdbus`.

**Audio Control — 178 lines** (`75359d0`)
Full PipeWire/PulseAudio integration. List sinks/sources, get/set volume
per-sink, mute/unmute, set default sink. MCP tools included. 13 new Action
variants, 7 protocol events.

```
audio.list_sinks          audio.list_sources
audio.get_volume          audio.set_volume { level, sink? }
audio.mute { mute, sink? }  audio.set_default { sink }
```

**Self-Update (#125) — 326 lines** (`89589bc`)
`deskbrid self-update` downloads the latest binary from GitHub releases,
replaces the running binary, and restarts the daemon. `src/cmd/update.rs`
(131 lines), `src/cmd/update/github.rs` (100 lines), `src/cmd/update/install.rs`
(95 lines). No external updater needed.

**Update Check — 60 lines** (`83e1401`)
Background daemon task polls GitHub releases API. Broadcasts `update.available`
events to all subscribers when a newer version is detected.

**System Tray — 417 lines** (`116d14a`)
Tray icon with update notifications. Uses `tray-icon` crate. Shows version
info, update alerts, and quick actions. 417 lines in `src/tray.rs`.

**Enlightenment DE** (`0a207e8`)
Detection and basic window management support. Desktop environment count now
at 9: GNOME, KDE, Hyprland, COSMIC, Sway, Labwc, XFCE, Budgie, Enlightenment.

---

### 🧹 Code Quality

- **`f611542`** — Clippy fixes: `collapsible_if`, `redundant_closure`. CI
  enforces `-D warnings` — any warning is fatal.
- **`96f0d8e`** — `collapsible_if` in X11 backend (Rust 1.95 edition 2024 lint)
- **`76a7cbf`** — Doc comment empty line + needless borrow clippy lints
- **`a41ceaa`** — Dead code removal: `is_network_action()` + stale header
- **`3e30a71`** — Dead NM zbus constants suppressed (later removed entirely)
- **`dcdc9ac`** — NM ObjectPath deserialization fix (intermediate step)
- **67 tests pass** — zero failures, zero ignored
- **fmt clean** — no formatting violations
- **clippy clean** — zero warnings with `-D warnings`

---

### 🌐 Website & Docs

- **`07b93ac`** — Site refresh: real hardware badges, Turtle test rig specs,
  Sway 33/33 matrix
- **`75554e5`** — Nick Launches featured badge
- **`216a917`**, **`9fe5baf`**, **`7326730`**, **`9f8795e`** — README badges:
  release, Discord, Nick Launches, repo stats. Vercel 503 workaround.
- **`8963b95`** — Site bumped to v0.11.0
- **`6c3f446`** — CHANGELOG.md with full release notes
- **`29c1c1f`** — ROADMAP updated: #45, #84 marked done; #62 description fixed

### 🤝 Community

- **@brauliobo** — PR #24: Fixed MCP stdio startup under Codex CLI. MCP server
  now correctly initializes when launched via `copilot --acp --stdio`. Merged
  May 26, 2026. First external MCP contribution.

---

### 📦 Breaking Changes

None. All 42 commits are additive. Wire protocol backward-compatible.
Config files, schedule.json, and macro format unchanged from v0.10.0.

---

**Full diff:** https://github.com/coe0718/deskbrid/compare/v0.10.0...v0.11.0
