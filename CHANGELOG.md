## v1.3.0 — REPL, Power Profiles & Environment Control

**22 commits · 60 files · +6,680 −1,463 since v1.2.0**

Feature release: interactive REPL mode, power management, locale/timezone/env control, presence system with push events, and batch security hardening from Vex review.

### 🖥️ Interactive REPL (#48)
- **6ba7a85** — `deskbrid repl` with rustyline loop, tab completion over 250+ action types. `dry_run` and `timeout` flags. Persistent history across sessions. Uses `client::send_raw` to dispatch arbitrary daemon actions without enum routing.

### 🔋 Power & Battery (#56, #57)
- **39712ec** — `power.profile.list/get/set` over `net.hadess.PowerProfiles` D-Bus. Validates profile names, switches via `Properties.Set`. Permissions allow-list updated for the 3 actions.
- **34b488e** — `battery.threshold.get/set` via sysfs (`charge_control_start_threshold` / `charge_control_end_threshold`). Vendor auto-detect (Lenovo vs Linux). Three convenience profiles: `daily`, `travel`, `full`. Graceful `supported:false` on unsupported hardware.

### 🌍 Locale & Timezone (#127)
- **b19b14f** — `locale.get/set` — reads resolve from process env + `/etc/locale.conf`. Reports `source` per key. Writes target `/etc/locale.conf`. Path traversal rejected. Non-root fails cleanly with `requires_root:true`.
- **b19b14f** — `timezone.get/set` — reads `/etc/localtime` against `/usr/share/zoneinfo`, computes UTC offset + DST. Writes `/etc/localtime` symlink.
- **38f9c10** — `locale.changed` / `timezone.changed` push events via DBus `PropertiesChanged` signal monitors. Tokio monitors in `daemon/locale_monitor.rs` subscribe to `org.freedesktop.locale1` and `org.freedesktop.timedate1`. Initial value seeded at startup to avoid spurious events.

### 📝 Environment Variables (#116)
- **75d6a69** — `env.get/set` on daemon's process environment. Returns `{found, value, kind, byte_len}` for one var or `{vars, count, non_utf8_count}` for all. Validates names (rejects empty, `=`, NUL).
- **6bb7845** — `env.persist/unset/list_persisted` — writes to `~/.config/environment.d/deskbrid.conf` (systemd user-session standard, no root). Atomic writes (tmp file + rename). Preserves existing keys not in the request. Reads back with systemd-style `\"`/`\\` un-escaping for round-trip correctness.

### 👤 Presence System (#39, #138)
- **4bffa9b** — `system.presence.get` action: reads idle seconds via backend, returns `{state, idle_seconds}`. Background monitor polls idle every 5s, emits on state transitions.
- **acd60f3** — Full presence spec: `presence.returned` (`idle_duration_secs`), `presence.locked`, `presence.unlocked` push events. `PresenceSnapshot` carries `last_active` (epoch) + `locked` fields. `PresenceConfig` action with runtime-editable idle/away thresholds. Logind `LockedHint` detection. `PresenceStore` on `DaemonState` for lock-free snapshot reads.

### 🕐 Time of Day (#40)
- **256d46f** — `system.time_of_day` action with configurable sunrise/sunset times, business hours detection, uptime, boot time, day of week, hour of day.

### 👁️ Vision Action Stubs (#41)
- **bd33c2c** — `vision.find_element`, `vision.find_by_text`, `vision.detect_state` actions with protocol parsing, serialization, and stub execution handlers. Template matching and ML detection stubbed for future implementation.

### 🔒 Vex Security Hardening
- **0589c29** — Batch 1: atomic locale/tz writes, dashboard auth, TOCTOU fix, PSK redaction, macro secret redaction.
- **7bf9c89** — Batch 2: DBus validation, OS guard, global rate bucket, audit regression tests, sweeper abort handle.
- **55f0f6a** — Batch 3: `ENV_LOCK` serialization, rate limit validation, CSP header.
- **84900da** — Batch 4: rule cycle detection, monitor reconnect, SSE per-IP cap, 12 assorted hardening items.
- **b02435a** — All findings addressed: W8 clipboard redaction, W13 `/tmp` fallback, W24 nmcli stdin password, W26 env-configurable rule depth, W27 macro bounds, remaining V1 findings.

### 🧹 Housekeeping
- **490f8af** — Quill docs update
- **d0adef5** — Removed all 4 code review artifacts
- **3f6dc72** — Cargo fmt formatting

## v1.2.0 — Sandboxed Profiles & Auto-Suspend

**Tuck · 19 commits · 147 files · +14,089 −1,894**

Agent safety release. Named sandboxed profiles with allow/deny/confirm gates. Auto-suspend system with dangerous command blocking, burst detection, and heartbeat canary suspension. Agent registry and distributed lock primitives. Screen region watching with text-change detection.

### 🛡️ Sandboxed Agent Profiles (#36)
- **90586be** — Named `[profile.NAME]` blocks in `permissions.toml` with allow/deny/confirm lists, audit levels, and profile-scoped rate buckets.
- **90586be** — Session profile binding on `session.create` and `connect`. Profile checks narrow (never widen) UID permissions.
- **90586be** — Profile confirmation requirements — actions can require explicit approval per profile.
- **90586be** — `PROFILE_DENIED` error responses at four dispatch gates: direct action, implied action, process starts, and confirmation mode.

### ⚠️ Auto-Suspend Safety System (#38)
- **90586be** — `src/daemon/auto_suspend.rs` — 216-line new module. Three suspension triggers:
  - Dangerous process commands: `rm -rf`, `mkfs.*`, fork bombs (`:(){:|:&};:`), `dd if=`
  - Suspicious action bursts: >10 `windows.focus` in 1s, >5 `files.delete` in 10s
  - Heartbeat timeout canary: agents that miss registered heartbeat intervals get suspended
- **90586be** — `AgentSuspended` / `AgentResumed` events emitted. Suspension bypass for resume, list, agent get, and confirmations.
- **90586be** — Configurable via `[auto_suspend]` block in `permissions.toml` (enabled, suspend_on_heartbeat_timeout, suspend_actions).

### 🤝 Agent Coordination
- **fb45875** — Agent registry: session and agent tracking, heartbeat registration, timeout sweeper.
- **fb45875** — Distributed lock primitives: acquire/release with token-based ownership. Token mismatch checking prevents one agent releasing another's lock.

### 👁️ Screen Region Watching
- **c6b213a** — `daemon/region_watch.rs` (722 lines): screen region monitoring with text-change detection, debounced via `DEFAULT_STABLE_DURATION_MS`. Async-correct, zero blocking calls.

### 🧪 Testing Infrastructure
- **6f13e8d** — Mock backend for protocol testing — enables zero-dependency daemon tests without a running desktop.
- **990d9a5** — Isolated daemon persistence tests — no shared on-disk DB between test runs.

### 🐛 Fixes
- **9fafc41** — Fix blocking `std::fs::write` in async context (`cmd/update/github.rs:68`). Caught by Claude (Sonnet 5) pre-release review.
- **723f39e** — Fix GitHub Actions workflow cleanup: add missing `actions:write` permission, run daily.
- **ad24de2** — Fix MCP Registry duplicate version rejection (bump required before republish).
- **56556f4** — Fix MCP `tools/list` returning empty — root cause was two bugs in tool registration.
- **e1155fd** — Fix code review findings.

### 📝 Docs & Polish
- **928caaa, bb0de0d, 1912c85, d34bb94** — Hermes MCP integration skill, LobeHub badge, README sections.
- **59ea607, 37157b9** — Quill: docs and wiki updates.
- **5332720** — ROADMAP: mark items #36 and #38 as ✅ Done.

## v1.1.0 — Security Hardening

**Tuck · 18 commits · 98 files · +3,756 −1,812**

Vex v2 audit resolution + structural hardening. All 6 warnings + 1 bonus resolved. Protocol refactored, lock ordering documented, DashMap migration, unreachable elimination, a11y selector baseline framework.

### 🔒 Vex v2 Security Audit — 7/7 Resolved (CODE_REVIEW_VEX_V2.md)
- **c3145d2** — B1: `ui.tree.get` removed from capabilities unsupported list (AT-SPI working). CDP reasons clarified.
- **c3145d2** — W1: secret-tool stderr sanitized — logged server-side, "internal error" returned to clients.
- **c3145d2** — W2+W3: Rules engine blocked from HIGH_RISK dispatch without confirmation (`RULES_HIGH_RISK_BLOCKED` error code). `is_high_risk()` made pub(crate).
- **c3145d2** — W4: Rule dispatch depth counter (AtomicU32, cap 5 via MAX_RULE_DISPATCH_DEPTH) prevents infinite rule→action→event→rule cascades.
- **c3145d2** — W5: Migration wrapped in BEGIN EXCLUSIVE/COMMIT transaction — crash mid-migration no longer corrupts schema version.
- **c3145d2** — W6: Audit log `parse_audit_params` now emits `tracing::warn!` on malformed JSON instead of silent drop.

### 🏗️ Structural Hardening (#25)
- **a6d8b03** — Protocol refactor: `mod.rs` split 1505→481 lines into domain files.
- **bf1dc86** — DaemonState: `Mutex<HashMap>` → `DashMap` for lock ordering simplification.
- **e8fb71d** — `docs/CONTRIBUTING.md` + lock ordering documentation added.
- **5885efb** — All `unreachable!()` panics eliminated from non-test code.
- **d1c1ad5** — MCP rate limiting, session cleanup, backend lock scope reduction.

### 🐛 Fixes
- **0831edf, 44e2148** — Flaky audit persistence tests fixed with WAL checkpoint pattern.
- **5a30bb2** — Dual DaemonState bug + MCP TCP auth gap + dashboard 0.0.0.0 bind fix.
- **39552a3, 92be69d** — ydotool Enter key fix (numeric code 28 pipe workaround).

### ♿ A11y Selector Framework
- **e7ffed5** — Auto-normalization for same-group role remaps across compositors.
- **d81955c** — Per-compositor selector baselines with LOUD failure detection.

### 🌐 Site
- **03ed1d3** — v2 landing page redesign.

## v1.0.0 — First Stable Release

**Tuck + Scout + Vex · 60 commits · 134 files · +18033 −4310**

Production-ready release: DB-backed persistence, rules engine, keyring, rate limiting, pressure monitoring, and a hardened security model with Vex audit remediation. Every core subsystem hardened with tests, explicit error handling, and clear boundaries.

### 🏗️ DB as Source of Truth (#84)
- **a0a7e32** — SQLite schema migrations, PRAGMA user_version, synchronous writes via tokio::sync::Mutex + spawn_blocking. 23 new tests.
- **7c0b191** — Scout's v1.0.0 release notes and changelog.

### ⚙️ Rules Engine v1.0.0 (#83)
- **4cefad6** — Split rules/eval.rs (454→218) into matching/timerange/engine modules. TimeRange timer, VarEquals/VarExists conditions, app_id resolution from window list.

### 🔑 Keyring/Secrets (#29)
- Secret-tool executor, protocol actions, MCP tools, CLI subcommand, dashboard card, confirmation-gated access. ~500 lines.

### 🚦 Rate Limiting (#129)
- Per-namespace, per-UID token buckets. 8 namespaces, permissions.toml config, wildcard 120/min, UID isolation. +285 lines.

### 📊 System Pressure/PSI (#96)
- /proc/pressure/{cpu,memory,io} monitoring, dashboard card, unit tests. ~100 lines.

### 📋 Provider Manifest (#135)
- **681b8b8** — capabilities.list now exposes high_risk actions, sandbox dirs, transport constraints, and permissions model. Enables orchestrator integration (Monadix, etc.). +37 lines.

### 🔒 Vex Security Audit — 37/37 Resolved
- **2233902** — C1/C2: Dashboard bound to 127.0.0.1 by default.
- **4c891f8** — C3/W9: Confirmation ownership check + backend-free routing.
- **3c05555** — C4: Prevent macro recording of secrets/clipboard/process actions.
- **bcdc197** — W1/W2/W14: Default-deny permissions, expanded HIGH_RISK_ACTIONS (21 actions), release artifact naming fix.
- **7e67061** — W6/W7: Hard-fail on missing checksum, verify in install.sh.
- **f44befb** — W10/W11: Bound TCP+dashboard reads, constant-time token compare, connection cap.
- **26aef22** — W12: Switch DB Mutex from std::sync to tokio::sync.
- **0738b0d** — W3/W4/W5: Path sandbox for files.search/watch, secure screenshot paths.
- **a5eddfa** — W8/W13: Clipboard history opt-out, HOME /root fallbacks.
- **599498d** — W18-W23: Deploy script hardening, mcp-publisher pin, rustls, CI gates.
- **f905848** — W15/W16/W17: Fix docs — bare binary URL, version mismatches, CLI names.
- **2398aa8** — W24/W25/W26: WiFi password via stdin, fix allowlist comment, reject nested dbus arrays.
- **6f96f22** — S1/S2: Wire remove_peer, add stale sweep, graceful socket path fallback.
- **33621e6** — S3/S4/S5/S6/S7: Capped read_line, Python tests, unwrap audit, egg-info cleanup.
- **f21d58d** — Vex review summary updated — 37/37 resolved.

### 🔨 Refactoring
- **ec77126** — Split protocol/types.rs (265→209) into common/envelope modules.
- **d97a9dd** — Split daemon/helpers.rs (329→186) into paths/process/responses.
- **8725836** — Split mcp/types.rs (696→120) into 5 domain modules.

### 📝 Documentation
- **7d69b7b** — Recover 7 deleted design docs from git history (10K lines) → docs/archive/.
- **0f8bedd** — permissions.example.toml with all actions, presets, high-risk ★ markers.
- **8468c1b** — Archive Vex review, Scout changelog, permissions example into docs/.
- **6a468b8** — Scout's rate limiting design spec.
- **f0071b4** — Scout's v1.0.0 test plan.
- **8e3db22** — Fix v1.0.0 doc version strings, add security/pressure/permissions sections.
- **ba45008–c1a3c71** — Fix broken docs links, PatchHive relationship, ecosystem references.

### 🐛 Fixes
- **bd72132** — Stable /tmp path in expand_path test.
- **f3ac61d** — Remove invalid `needs: ci` from release workflow.
- **2837a64** — Remove duplicate `use super::*` in paths.rs tests.
- **503fabe** — Drop state before creating state2 in test to ensure WAL checkpoint.
- **d7e3e7d** — CI: install libpipewire-0.3-dev for --all-features builds.

## v0.13.0 — Action Confirmation, Agent Messaging, Unified Search

**Tuck · 8 commits · 32 files · +1197 −20**

Three major features: destructive-action gating, inter-agent communication, and cross-surface search — all with live dashboard cards, MCP tools, and background TTL sweeping.

### 🛡️ Action Confirmation Mode (#37)
- **b98e37e** — Protocol core: 9 new action variants (`confirmation.*`, `agent.*`, `search.*`), parse modules, execute handlers, dispatch gating. Pending confirmations queue in `DaemonState` with `require_confirmation` flag on destructive actions.
- **096ce65** — MCP tools: `confirm`, `deny`, `list` (confirmation); `send`, `broadcast`, `mailbox` (agent); `search`, `index` (search). 8 new tools wired via `block_state` and $crate macros.
- **1a091e2** — Dashboard: three new SSE cards — confirmation queue, agent mailbox, search index.
- **216f58c** — Background sweeper: `spawn_confirmation_sweeper()` runs every 30s, purges confirmations older than 5min. Wired at daemon startup.

### 📬 Agent-to-Agent Messaging (#44)
- **b98e37e** — In-process `HashMap<SessionId, Vec<AgentMessage>>` mailbox with TTL-based expiry.
- **0544142** — TTL cleanup: `is_expired()` prunes on store and `get_for()`. Messages expire after configurable TTL.
- **096ce65** — MCP: `agent.send`, `agent.broadcast`, `agent.mailbox` tools for inter-session messaging.

### 🔍 Unified Search (#80)
- **b98e37e** — `SearchIndex` struct in `DaemonState` indexing windows, apps, files, clipboard, and audit log with relevance scoring.
- **0544142** — Async safety: `std::fs::read_dir` → `tokio::fs::read_dir` with async iteration. Scope documented as v1 (4 directories).
- **8f8b203** — Protocol fix: MCP tools used wrong prefix `unified.*` → corrected to `search.*` (wire format from `action_type.rs`).

### 🧹 Fixes
- **0544142** — Claude review: blocking `read_dir` eliminated, TTL sweepers added for both confirmation queue and agent mailbox.
- **87b84c8** — Leftover `cargo fmt` from protocol core commit (7 files missed in original add).
- **07e4aa9** — Agent file refresh: AGENTS.md, CLAUDE.md, hermes skill updated for new features.

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
