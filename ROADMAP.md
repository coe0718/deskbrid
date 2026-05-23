# Deskbrid — Linux Control Expansion

**Purpose:** Catalog every mechanism Deskbrid can use to gain deeper control over Linux
systems — beyond what the current DE backends already provide.

**Current state:** v0.6.1. Deskbrid already has an impressive surface (90+ actions, 5 DE
backends). This doc focuses on what's **not yet in the code** — the remaining 80% of the
operating system that agents need to control.

### Roadmap Status

Use this document as the source of truth while features land. When a roadmap item
ships, keep the section in place, add a `**Status:**` line under its heading, and add
it to the completed table below.

| Status | Meaning |
|---|---|
| ✅ Done | Landed on `main` and exposed through protocol/client surfaces |
| 🚧 In Progress | Actively being implemented |
| ⏭️ Next | Selected as an upcoming implementation target |
| 🧭 Planned | Not started yet |
| ⚠️ Blocked | Waiting on design, dependency, permissions, or platform work |

### Completed From This Roadmap

| Section | Landed Scope | Code |
|---|---|---|
| [1. systemd (logind + manager)](#1-systemd-logind--manager) | Inhibit/release, session list/lock/switch, service and timer control, journal query | `src/daemon/system/`, `src/protocol/`, `src/cli/`, `clients/python/` |
| [2. polkit (PolicyKit Privilege Escalation)](#2-polkit-policykit-privilege-escalation) | Check/request authorization and ship Deskbrid policy actions | `src/daemon/system/polkit.rs`, `deploy/org.deskbrid.policy` |
| [9. Confinement Detection](#9-confinement-detection-flatpak--snap--selinux--apparmor) | Detect Flatpak, Snap, AppImage, containers, WSL, AppArmor, and SELinux via `system.confinement` and capability/health reports | `src/daemon/capabilities/confinement.rs`, `src/protocol/`, `clients/python/` |
| [12. OCR / Text Extraction](#12-ocr--text-extraction) | OCR existing screenshots or fresh captures with optional word boxes through Tesseract | `src/ocr.rs`, `src/protocol/`, `src/cli/`, `clients/python/` |
| [13. Terminal / PTY Multiplexer](#13-terminal--pty-multiplexer) | Create, write, read, resize, list, and kill interactive PTY sessions | `src/daemon/terminal.rs`, `src/protocol/`, `src/cli/`, `clients/python/` |
| [14. MPRIS Media Control](#14-mpris-media-control) | List MPRIS players, inspect playback status/metadata, and send common playback commands | `src/daemon/mpris.rs`, `src/protocol/`, `src/cli/`, `clients/python/` |
| [15. Drag & Drop](#15-drag--drop) | Drag pointer coordinates with configurable button and duration through backend input injection | `src/backend/`, `src/protocol/`, `src/cli/`, `clients/python/` |
| [16. Application Menu Catalog](#16-application-menu-catalog) | List, search, and inspect installed `.desktop` applications from XDG application directories | `src/daemon/apps.rs`, `src/protocol/`, `src/cli/`, `clients/python/` |
| [18. Clipboard History](#18-clipboard-history) | Query and clear bounded text entries observed through Deskbrid clipboard reads/writes | `src/daemon/clipboard.rs`, `src/protocol/`, `src/cli/`, `clients/python/` |
| [19. Window Tiling Presets](#19-window-tiling-presets) | Move windows into named monitor-aware presets with optional padding | `src/tiling.rs`, `src/protocol/`, `src/cli/`, `clients/python/` |
| [20. Color Picker](#20-color-picker) | Sample pixel colors from a screenshot path or live 1x1 capture and return RGBA/hex | `src/color.rs`, `src/protocol/`, `src/cli/`, `clients/python/` |
| [24. Screenshot Diffing](#24-screenshot-diffing) | Pixel diff screenshots with tolerance, changed bounding boxes, optional diff images, and wait-driven stability | `src/visual.rs`, `src/daemon/wait.rs`, `src/protocol/`, `clients/python/` |
| [26. Wait-for Conditions](#26-wait-for-conditions) | Daemon-polled waits for windows, clipboard, processes, files, idle time, and screenshot stability | `src/daemon/wait.rs`, `src/protocol/`, `src/cli/`, `clients/python/` |
| [sysfs Backlight](#6-sysfs--procfs--devfs-direct-hardware-access) | Read and set `/sys/class/backlight` devices by percentage with permission-aware metadata | `src/daemon/sysfs.rs`, `src/protocol/`, `src/cli/`, `clients/python/` |
| [Backend Parity Pass](docs/BACKEND_PARITY.md) | Matrix-backed compositor audit plus X11/COSMIC shared Linux domain implementations and explicit unsupported capability reporting | `src/backend/`, `src/daemon/capabilities/`, `docs/BACKEND_PARITY.md` |
| [33. Dry-Run Mode](#33-dry-run-mode) | Request-level `dry_run` option validates permissions and reports would-execute metadata without loading a backend | `src/protocol/parse.rs`, `src/daemon/dispatch.rs`, `src/client.rs`, `src/cli/` |
| [34. Audit Log](#34-audit-log) | In-memory action ring buffer with query/clear actions, duration, status, UID, and error metadata | `src/daemon/audit.rs`, `src/daemon/dispatch.rs`, `src/protocol/`, `src/cli/`, `clients/python/` |
| [35. Rate Limiting](#35-rate-limiting-per-client) | Per-UID token bucket with configurable rate/burst and audited `RATE_LIMITED` responses | `src/daemon/rate_limit.rs`, `src/daemon/dispatch.rs` |
| [71. Action Timeouts](#71-action-timeouts-with-kill-guarantees) | Request-level/default timeout wrapper around dispatched actions, with `wait.for` preserving its own deadline | `src/daemon/dispatch.rs`, `src/protocol/parse.rs`, `src/client.rs`, `src/cli/` |

### Already Built (not covered here)

These features exist in the codebase already for reference:

| Feature | File(s) | Protocol Actions |
|---|---|---|
| **AT-SPI2 Accessibility** | `src/a11y/` (tree, bus, actions, value, setup, util) | `a11y.tree`, `a11y.get_element`, `a11y.click_element`, `a11y.get_text` |
| **File CRUD** | `src/daemon/execute.rs` (dispatcher) + `execute_files.rs` | `files.read/write/copy/move/delete/mkdir/list` |
| **Browser CDP** | `src/browser.rs` | `browser.list_tabs/navigate/evaluate/screenshot_tab/click` |
| **Screen recording (half-built)** | `src/backend/gnome/` (core, screenshot, screencast) | `screencast.start/stop` in protocol, Mutter ScreenCast session exists but recording output not wired |
| **Event filtering (Subscribe/Unsubscribe)** | `src/daemon/client.rs` | `subscribe`, `unsubscribe` — glob patterns (`window.*`, `file.*`) |
| **Systemd/logind control** | `src/daemon/system/{logind,systemd}.rs` | `system.inhibit/release_inhibit`, `system.sessions`, `system.lock_session`, `system.switch_user`, `service.*`, `journal.query`, `timer.*` |
| **Polkit auth checks** | `src/daemon/system/polkit.rs`, `deploy/org.deskbrid.policy` | `system.check_auth`, `system.elevate` |
| **System health checks** | `src/daemon/capabilities/` | `system.health` — dependency reporting with per-backend remediation tips |
| **Idle detection** | `src/daemon/execute.rs` (dispatcher) → `execute_system.rs` | `SystemIdle` — current idle seconds |
| **Active window context** | (implied by `windows.list` + `SystemInfo`) | Agents can query current state — no auto-attach |

---

## Table of Contents

1. [✅ systemd (logind + manager)](#1-systemd-logind--manager)
2. [✅ polkit (PolicyKit Privilege Escalation)](#2-polkit-policykit-privilege-escalation)
3. [Linux Capabilities](#3-linux-capabilities)
4. [cgroups v2 (Process Resource Control)](#4-cgroups-v2-process-resource-control)
5. [udev (Device Event Monitoring)](#5-udev-device-event-monitoring)
6. [sysfs / procfs / devfs (Direct Hardware Access)](#6-sysfs--procfs--devfs-direct-hardware-access)
7. [fanotify (System-Wide File Monitoring)](#7-fanotify-system-wide-file-monitoring)
8. [eBPF / LSM BPF](#8-ebpf--lsm-bpf)
9. [✅ Confinement Detection (Flatpak / Snap / SELinux / AppArmor)](#9-confinement-detection-flatpak--snap--selinux--apparmor)
10. [Desktop Portal Integration (XDG Portals)](#10-desktop-portal-integration-xdg-portals)
11. [elogind (Non-systemd Systems)](#11-elogind-non-systemd-systems)
12. [✅ OCR / Text Extraction](#12-ocr--text-extraction)
13. [✅ Terminal / PTY Multiplexer](#13-terminal--pty-multiplexer)
14. [✅ MPRIS Media Control](#14-mpris-media-control)
15. [Drag & Drop](#15-drag--drop)
16. [✅ Application Menu Catalog](#16-application-menu-catalog)
17. [Screen Recording (Finish Half-Built)](#17-screen-recording-finish-half-built-implementation)
18. [✅ Clipboard History](#18-clipboard-history)
19. [✅ Window Tiling Presets](#19-window-tiling-presets)
20. [✅ Color Picker](#20-color-picker)
21. [Desktop Settings](#21-desktop-settings-readwrite-configuration)
22. [Keyboard Layout Management](#22-keyboard-layout-management)
23. [Session & User Management](#23-session--user-management)
24. [✅ Screenshot Diffing](#24-screenshot-diffing)
25. [Action Recording & Replay](#25-action-recording--replay-macros)
26. [✅ Wait-for Conditions](#26-wait-for-conditions)
27. [Cron / Scheduled Actions](#27-cron--scheduled-actions)
28. [D-Bus Raw Access](#28-d-bus-raw-access-escape-hatch)
29. [Secret / Keyring Access](#29-secret--keyring-access)
30. [TCP Mode (Network)](#30-tcp-mode-network-control)
31. [Named Sessions](#31-named-sessions-multi-agent-isolation)
32. [Remote Screenshot Streaming](#32-remote-screenshot-streaming)
33. [✅ Dry-Run Mode](#33-dry-run-mode)
34. [✅ Audit Log](#34-audit-log)
35. [✅ Rate Limiting](#35-rate-limiting-per-client)
36. [Sandboxed Profiles](#36-sandboxed-agent-profiles)
37. [Action Confirmation](#37-action-confirmation-mode)
38. [Canary Actions](#38-canary-actions--auto-suspend)
39. [User Presence](#39-user-presence-events)
40. [Time & Location](#40-time-of-day--location-awareness)
41. [CV Element Detection](#41-element-detection-via-screenshot-cv)
42. [Region Watching](#42-screen-region-watching)
43. [Text Change Events](#43-text-change-events-watched-regions)
44. [Agent Messaging](#44-agent-to-agent-messaging)
45. [Shared Blackboard](#45-shared-blackboard-kv-store)
46. [Lock / Mutex](#46-lock--mutex-primitives)
47. [Agent Registry](#47-agent-registry)
48. [REPL Mode](#48-repl-mode)
49. [Action Simulator](#49-action-simulator-replay-capture)
50. [Protocol Fuzzer](#50-protocol-fuzzer)
51. [OpenAPI Schema](#51-openapi--json-schema-export)
52. [Semantic Screen Indexing](#52-semantic-screen-indexing)
53. [Wayland Protocols](#53-wayland-protocols-not-yet-wrapped)
54. [Audio Control (PipeWire)](#54-audio-control-pipewire--pulseaudio-d-bus)
55. [GPU Power States](#55-gpu-power-states)
56. [Battery Thresholds](#56-battery-charge-threshold-management)
57. [Power Profiles](#57-power-profiles-daemon)
58. [USB Device Control](#58-usb-device-power-control)
59. [Input Device Config](#59-input-device-configuration)
60. [Monitor DDC/CI](#60-monitor-ddcci)
61. [Notification History](#61-notification-history--action-buttons)
62. [NetworkManager D-Bus](#62-networkmanager-d-bus)
63. [Tailscale / WireGuard](#63-tailscale--wireguard-status)
64. [mDNS Discovery](#64-mdns-advertisement-deskbrid-instance-discovery)
65. [Distrobox / Toolbox](#65-distrobox--toolbox-integration)
66. [Docker / Podman](#66-docker--podman-container-awareness)
68. [mTLS for TCP](#68-mtls-for-tcp-mode)
69. [Landlock + Seccomp](#69-landlock--seccomp-for-spawned-processes)
70. [Immutable Permissions](#70-immutable-permissions)
71. [✅ Action Timeouts](#71-action-timeouts-with-kill-guarantees)
72. [Audit Signing](#72-audit-trail-signing)
73. [Test Suite](#73-protocol-test-suite)
74. [Benchmarking](#74-action-benchmarking)
75. [Version Negotiation](#75-version-negotiation)
76. [Action Queue](#76-action-queue-with-ordering)
77. [Retry Backoff](#77-retry-with-exponential-backoff)
78. [Health Webhook](#78-health-webhook)
79. [Session Persistence](#79-session-persistence-survive-logout)
80. [Unified Search](#80-unified-search)
81. [Plugin System](#82-plugin-system)
82. [Rules Engine](#83-event-driven-triggers-rules-engine)
83. [Persistence (SQLite)](#84-persistence-layer-sqlite)
84. [MCP Server](#85-mcp-server-mode)
85. [Declarative Workflows](#86-declarative-workflows--state-machines)
86. [Web Dashboard](#87-built-in-web-dashboard)
87. [Screenshot Timeline](#88-screenshot-timeline)
88. [Degradation Profiles](#89-graceful-degradation-profiles)
89. [Self-Healing](#90-self-healing-fallback-chains)
90. [Desktop Apps](#91-desktop-application-management)
91. [Compositor Rules](#92-compositor--window-manager-rules)
92. [Workspace Lifecycle](#93-workspace-lifecycle)
93. [File Metadata](#94-advanced-file-metadata)
94. [Storage Monitor](#95-storage-monitoring)
95. [System Pressure](#96-system-pressure--psi)
96. [Firewall](#97-network-firewall-management)
97. [Proxy](#98-network-proxy-management)
98. [Captive Portal](#99-captive-portal-detection)
99. [IME Control](#100-input-method-engine-control)
100. [Virtual Monitor](#101-virtual-monitor-support)
101. [Monitor Color](#102-monitor-color-management)
102. [Bluetooth Profiles](#103-bluetooth-profile--device-control)
103. [Print](#104-print-cups-control)
104. [Scan](#105-scanner-sane-support)
105. [Privacy Devices](#106-privacy-device-monitoring)
106. [Portal Permissions](#107-xdg--flatpak-portal-permissions)
107. [Do Not Disturb](#108-do-not-disturb--notification-policy)
108. [SSH/GPG Agents](#109-ssh--gpg-agent-awareness)
109. [Layout Profiles](#111-layout-profiles-window-snapshots)
110. [Gamepad Input](#112-gamepad--joystick-input-injection)
111. [RGB Lighting](#113-rgb--peripheral-lighting-openrgb)
112. [Desktop Search](#114-desktop-search-tracker--recoll)
113. [Display Manager](#115-greeter--display-manager-control)
114. [Session Env Vars](#116-session-environment-variable-management)
115. [Prometheus / OTel](#117-prometheus--opentelemetry-export)
116. [Package Manager](#118-cross-distro-package-management)
117. [Nix / Guix](#119-nix--guix-awareness)
118. [TPM / Security](#120-tpm--hardware-security)
119. [Headless Compositor](#121-headless-wayland-compositor-ci--testing)
120. [Mock Backend](#122-mock-backend-for-agent-testing)
121. [Plugin Hot-Reload](#123-plugin-hot-reload)
122. [Graceful Restart](#124-graceful-restart--config-live-reload)
123. [Auto-Update](#125-auto-update-with-rollback)
124. [Shared Memory](#126-shared-memory--zero-copy-buffer-passing)
125. [Locale Events](#127-locale--timezone-change-events)
126. [Filesystem Snapshots](#128-btrfs--zfs-snapshot-integration)
127. [Priority Roadmap](#129-priority-roadmap)

---

## 1. systemd (logind + manager)

**Status:** ✅ Done on `main` as a CLI-backed implementation. Deskbrid now exposes
inhibitors, logind session operations, systemd service/timer control, and journal
queries through the protocol, CLI, and Python client. A native `zbus_systemd`
implementation remains an optional future refinement.

### Original Gap

Deskbrid can already do `system.power` (power off/reboot/suspend) via backend-specific
commands, but has no control over:
- **Inhibiting sleep/shutdown** while the agent is working
- **Managing user sessions** (list, lock, switch users)
- **Controlling systemd services** (start/stop/restart/enable/disable)
- **Querying the journal** (agent needs logs for debugging)
- **Managing timers** (agent-scheduled recurring jobs)
- **Getting systemd status** (boot time, unit states, failed services)

### Implementation Notes

The shipped implementation uses async CLI wrappers in `src/daemon/system/`:
`systemd-inhibit` for inhibitors, `loginctl`/`dm-tool` for sessions, `systemctl`
for units and timers, and `journalctl` for journal queries.

The original native D-Bus plan remains useful as a future hardening path:

**Crate:** `zbus_systemd` (v0.26.0) — official zbus companion, auto-generated DBus
bindings for all systemd services. Feature-gated modules per service.

Already has `zbus` as a dependency — adding `zbus_systemd` is zero friction.

**Key interfaces:**

| Interface | zbus_systemd module | Actions |
|---|---|---|
| `org.freedesktop.login1.Manager` | `login1` | `Inhibit()`, `ListSessions()`, `LockSession()`, `SwitchToUser()` |
| `org.freedesktop.login1.Seat` | `login1` | `SwitchTo()`, session management |
| `org.freedesktop.systemd1.Manager` | `systemd1` | `StartUnit()`, `StopUnit()`, `EnableUnitFiles()`, `Reload()`, `GetUnit()` |
| `org.freedesktop.systemd1.Unit` | `systemd1` | `Start()`, `Stop()`, `Reload()`, active state queries |
| `org.freedesktop.journal1` | (separate) | `GetCursor()`, `SeekTail()`, `Next()` for log iteration |

### Protocol Actions

```rust
// Login/Session management
SystemInhibit {
    what: String,        // "sleep", "shutdown", "idle", etc.
    who: String,         // "Agent running task X"
}
SystemReleaseInhibit(u32),  // fd or cookie
SystemListSessions,
SystemLockSession { session_id: String },
SystemSwitchUser { username: String },

// Service management
ServiceStatus { name: String },
ServiceStart { name: String },
ServiceStop { name: String },
ServiceRestart { name: String },
ServiceEnable { name: String, runtime: bool },
ServiceDisable { name: String, runtime: bool },
ServiceList { unit_type: Option<String> },  // "service", "timer", "socket"

// Journal
JournalQuery {
    since: Option<u64>,   // unix timestamp
    until: Option<u64>,
    unit: Option<String>,
    priority: Option<u8>, // 0=emerg .. 7=debug
    tail: Option<u32>,    // last N lines
},

// Timer
TimerList,
TimerStart { name: String },
TimerStop { name: String },
```

**Future refinement:** Add `JournalFollow` for streaming log tails.

### Permission Implications

These are relatively safe actions (inhibit, query status) mixed with system-affecting
ones (service control, user switch). **Per-action permissions via the existing
`permissions.toml` system** are essential here.

---

## 2. polkit (PolicyKit Privilege Escalation)

**Status:** ✅ Done on `main` as a `pkcheck`-backed implementation. Deskbrid now exposes
`system.check_auth` and `system.elevate`, and ships `deploy/org.deskbrid.policy`.
Transparent per-action dispatch checks and native `zbus_polkit` integration remain
optional future refinements.

### Original Gap

Deskbrid currently runs at the user's privilege level. Some actions need more authority:
- Installing/removing system packages (agent-driven dev environment setup)
- Modifying system-wide configuration
- Controlling systemd services as root
- Writing to protected paths

Currently, the answer is "run deskbrid as root" — which is terrible. polkit gives
a proper elevation path.

### Implementation Notes

The shipped implementation calls `pkcheck` from `src/daemon/system/polkit.rs` and
ships `deploy/org.deskbrid.policy` for Deskbrid action definitions.

The original native D-Bus plan remains useful as a future hardening path:

**Crate:** `zbus_polkit` (v5.0.0) — same zbus ecosystem, provides `AuthorityProxy`
for checking authorizations. Already has `zbus` as a dep.

**Pattern:**

```rust
use zbus_polkit::policykit1::{AuthorityProxy, Subject, CheckAuthorizationFlags};

let proxy = AuthorityProxy::new(&connection).await?;
let subject = Subject::new_for_owner(std::process::id(), None, None)?;
let result = proxy.check_authorization(
    &subject,
    "org.deskbrid.system.service-install",  // action ID
    &HashMap::new(),
    CheckAuthorizationFlags::AllowUserInteraction.into(),
    "",
).await?;
```

**Shipped pieces and remaining refinements:**

1. A `.policy` file installed to `$PREFIX/share/polkit-1/actions/org.deskbrid.policy`
   defining available actions and their auth levels (`auth_admin`, `auth_self`, `yes`) ✅
2. `pkcheck`-backed auth checks for explicit `system.check_auth` and `system.elevate` ✅
3. Zbus-based polkit checks before executing elevated actions ⏭️ optional refinement
4. A `PolkitAction` field on the backend or daemon to track which actions need auth ⏭️ optional refinement

**Example `.policy` actions:**

| Action ID | Auth Level | Purpose |
|---|---|---|
| `org.deskbrid.system.service-install` | `auth_admin` | systemctl enable/install |
| `org.deskbrid.system.package-install` | `auth_admin` | apt/pacman/dnf install |
| `org.deskbrid.system.config-system` | `auth_admin` | Write to /etc |
| `org.deskbrid.system.suspend-inhibit` | `auth_self` | Inhibit sleep |
| `org.deskbrid.hardware.gpio` | `auth_self` | GPIO access |
| `org.deskbrid.files.system-write` | `auth_admin` | Write outside home |

### Protocol Actions

```rust
// Check if an action would pass polkit before attempting it
SystemCheckAuth {
    action_id: String,
}
// Request elevation via polkit dialog
SystemElevate {
    action_id: String,
    reason: String,
}
```

**Better design:** Integrate polkit checks *transparently* inside the dispatch layer.
When an action has a polkit requirement, check silently first. If denied, attempt to
show auth dialog via `AllowUserInteraction` flag. If that also fails, return
`status: "error"` with `code: "AUTHENTICATION_REQUIRED"`.

---

## 3. Linux Capabilities

### What's Missing

Deskbrid currently drops all capabilities (runs as user). For deeper control, it needs
specific capabilities for specific operations — **without running as root**.

**Current capability requirements by feature:**

| Feature | Requires | Current Solution |
|---|---|---|
| Input injection | `/dev/uinput` write | udev rule + `input` group |
| Network config | `CAP_NET_ADMIN` | Hardcoded sudo nmcli calls |
| Raw/bind sockets | `CAP_NET_RAW` / `CAP_NET_BIND_SERVICE` | Not available |
| Process priority | `CAP_SYS_NICE` | Not available |
| Block suspend | `CAP_BLOCK_SUSPEND` | Not available |
| fanotify | `CAP_SYS_ADMIN` | Not available |
| cgroups | `CAP_SYS_RESOURCE` | Not available |

### Implementation

**Crate:** `caps` (v0.5.6) — pure Rust, covers all 5 POSIX capability sets.

**Two approaches (use both):**

**A) Binary capabilities** — `setcap` on the installed binary:
```bash
setcap 'cap_net_raw,cap_net_admin,cap_block_suspend+ep' /usr/bin/deskbrid
```

**B) systemd service unit** — `AmbientCapabilities=` for runtime-dropped privileges:
```ini
[Service]
AmbientCapabilities=CAP_NET_RAW CAP_NET_ADMIN CAP_BLOCK_SUSPEND CAP_SYS_NICE
CapabilityBoundingSet=CAP_NET_RAW CAP_NET_ADMIN CAP_BLOCK_SUSPEND CAP_SYS_NICE
```

**Runtime management via `caps` crate:**

```rust
use caps::{Capability, CapSet, has_cap, raise};

// Check if we have what we need — if not, report it in system.capabilities
if !has_cap(None, CapSet::Effective, Capability::CAP_NET_RAW)? {
    // Can't use raw sockets — mark as degraded in capabilities response
}

// Raise into ambient set when spawning child processes
raise(None, CapSet::Ambient, Capability::CAP_BLOCK_SUSPEND)?;
```

**What to add to `system.capabilities` response:**

```json
{
  "effective": ["cap_net_raw", "cap_net_admin"],
  "available": ["cap_net_raw", "cap_net_admin", "cap_block_suspend"],
  "missing": ["cap_sys_nice"],
  "actions_degraded": ["process.set_priority"]
}
```

### Protocol Actions

```rust
// Query available capabilities
CapabilitiesList,  // already exists in protocol but looks empty

// Manage capabilities programmatically (if running with sufficient privilege)
CapabilityDrop {
    capability: String,
}
CapabilityInfo,  // detailed capability status
```

---

## 4. cgroups v2 (Process Resource Control)

### What's Missing

Deskbrid can spawn processes via `ProcessStart`, but has zero control over their
resource consumption. An agent could:
- Fork-bomb the system
- Exhaust all memory
- Pin 100% CPU and starve the daemon
- Spawn an infinite loop with no kill switch

### Implementation

**Crate:** `cgroups-rs` — native Rust, supports v1 and v2. Available on crates.io.

**Pattern for sandboxing spawned processes:**

```rust
use cgroups_rs::{Cgroup, CgroupPid, MaxValue};
use cgroups_rs::cgroup_builder::CgroupBuilder;

let cg = CgroupBuilder::new("deskbrid/sandbox")
    .cpu()
        .shares(256)           // low priority
        .quota(50_000)         // 50ms per 100ms period = 50% max CPU
        .period(100_000)
        .done()
    .memory()
        .memory_hard_limit(1024 * 1024 * 1024)  // 1GB max
        .memory_soft_limit(512 * 1024 * 1024)    // 512MB soft
        .done()
    .pid()
        .maximum_number_of_pids(50)              // max 50 child processes
        .done()
    .build()?;

// Add spawned PID to the cgroup
cg.add_task(CgroupPid::from(child_pid))?;
```

### Protocol Actions

```rust
ProcessStart {
    command: Vec<String>,
    workdir: Option<String>,
    env: Option<HashMap<String, String>>,
    // NEW fields:
    cpu_quota: Option<u32>,       // CPU time max (μs per period)
    cpu_period: Option<u32>,      // CPU period (μs, default 100000)
    cpu_shares: Option<u32>,      // CPU weight (lower = less priority)
    memory_max: Option<u64>,      // Hard memory limit (bytes)
    memory_soft: Option<u64>,     // Soft memory limit (bytes)
    pids_max: Option<u32>,        // Max child PIDs
    io_max: Option<String>,       // I/O limit (e.g. "8:0 10485760")
    cgroup_name: Option<String>,  // Custom cgroup name for grouping
}

// Runtime adjustment
ProcessSetCgroup {
    pid: u32,
    cpu_quota: Option<u32>,
    memory_max: Option<u64>,
    pids_max: Option<u32>,
}

// Query cgroup stats
ProcessResourceUsage {
    pid: u32,
}
```

**Note:** cgroups v2 requires `CAP_SYS_RESOURCE` or `Delegate=yes` in the service
unit. For user-scoped cgroups, systemd's `--user` instance manages a delegated
subtree at `/sys/fs/cgroup/user.slice/user-${UID}.slice/`.

---

## 5. udev (Device Event Monitoring)

### What's Missing

Deskbrid can query hardware state (battery, monitors), but can't react to hardware
changes. An agent should be able to:
- Detect USB drive insertion → auto-mount
- Detect monitor hotplug → re-run display config
- Detect input device changes → re-init input injection
- Detect network interface changes → trigger reconnection

### Implementation

**Crate:** `udev` (bindings to libudev). Already has PipeWire/ScreenCast deps so
system libraries aren't a blocker.

```rust
use udev::MonitorBuilder;

let mut monitor = MonitorBuilder::new()?
    .match_subsystem("usb")?
    .listen()?;

for event in monitor.iter() {
    match event.event_type() {
        udev::EventType::Add => {
            let dev = event.device();
            if is_block_device(&dev) {
                trigger_auto_mount(&dev);
            }
        }
        udev::EventType::Remove => {
            // clean up
        }
        _ => {}
    }
}
```

**Subsystems to monitor:**

| Subsystem | Events | Agent Actions |
|---|---|---|
| `usb` | Add/Remove | Auto-mount, notify, trigger backup |
| `drm` | Change | Re-run monitor detection |
| `input` | Add/Remove | Re-init input devices |
| `block` | Add/Remove | Mount filesystems |
| `net` | Add/Remove | Restart network monitor |
| `power_supply` | Change | Battery status updates |
| `backlight` | Change | Screen brightness adaptation |

### Implementation Strategy

Add a udev event listener thread in the daemon that converts events into
`DeskbridEvent::DeviceAdded`, `DeskbridEvent::DeviceRemoved`, etc. These get
broadcast to subscribed clients — clients subscribe via `files.watch`-style
subscription patterns.

### Protocol Actions

```rust
// Subscribe to device events
DeviceSubscribe {
    subsystems: Vec<String>,  // ["usb", "drm", "block"]
}

// Mount/umount (requires polkit or capabilities)
DeviceMount {
    device: String,
    mount_point: String,
    fs_type: Option<String>,
    options: Option<Vec<String>>,
}
DeviceUnmount {
    mount_point: String,
}
DeviceInfo {
    device: String,
}
```

---

## 6. sysfs / procfs / devfs (Direct Hardware Access)

**Status:** 🚧 In Progress. Backlight read/write support is done through
`system.backlight.get` and `system.backlight.set`. Thermal and CPU readouts are next.

### What's Missing

Deskbrid has no backend-agnostic access to hardware knobs:
- Screen brightness (beyond what DE-specific APIs provide)
- Backlight control
- LEDs (keyboard, notification)
- CPU frequency scaling
- Thermal sensors
- Fan speed (when available)
- GPIO pins (on embedded/SBC setups)

### Implementation

These are all simple file reads/writes. No crate needed for most — just `std::fs`.

```rust
// Screen brightness
fn set_brightness(percent: f64) -> Result<()> {
    let max = fs::read_to_string("/sys/class/backlight/intel_backlight/max_brightness")?
        .trim().parse::<u32>()?;
    let value = (max as f64 * percent / 100.0) as u32;
    fs::write("/sys/class/backlight/intel_backlight/brightness", value.to_string())?;
    Ok(())
}
```

### Protocol Actions

```rust
// Brightness
SystemBacklightGet,
SystemBacklightSet {
    percent: f64,
    device: Option<String>,       // backlight device name
}

// Keyboard LEDs
SystemLedsList,
SystemLedSet {
    name: String,
    brightness: u32,
}

// Thermal
SystemThermalGet,                 // returns all thermal zone temps

// CPU
SystemCpuFrequency,               // current CPU freq per core
SystemCpuGovernor,                // current cpufreq governor
SystemCpuSetGovernor {
    governor: String,             // "performance", "powersave", "ondemand"
}
```

**Permission note:** `/sys/class/backlight/*/brightness` often requires root.
Check via polkit when applicable. On some distros, the `video` group grants access.

**Better alternative for brightness:** Use `org.freedesktop.portal.Settings` or
logind/power-profiles-daemon DBus API instead of raw sysfs — works across Wayland
compositors and doesn't need special permissions.

---

## 7. fanotify (System-Wide File Monitoring)

### What's Missing

Deskbrid's current `files.watch` uses notify (inotify), which:
- Requires watching each directory individually
- Can't detect *who* changed a file
- Misses events on files outside watched directories
- Hit `fs.inotify.max_user_watches` limits easily

fanotify gives:
- **System-wide monitoring** — watch entire mount points or filesystems
- **PID attribution** — know which process made each change
- **Permission events** — intercept and allow/deny file access before it happens
- **No per-directory setup** — one watch covers everything

### Implementation

**Crate:** `fsmon` or `rfanotify` — Rust implementations of fanotify.

**Key use cases for Deskbrid:**

1. **Security monitoring:** Watch `/home` for unauthorized file access by spawned
   subprocesses
2. **Change detection:** Get notified of *any* file change on the system (config
   files, package manager writes, etc.)
3. **Permission-based file access control:** Use permission events to grant/deny
   specific file access for agent-spawned processes

**Requires:** `CAP_SYS_ADMIN` or `CAP_SYS_RAWIO` capability.

### Protocol Actions

```rust
FilesWatchSystem {
    mount_point: Option<String>,  // defaults to "/"
    track_pid: bool,
}
FilesPermissionEvent {
    action: String,               // "allow", "deny"
    event_id: String,
}
```

---

## 8. eBPF / LSM BPF

### What's Missing

eBPF gives kernel-level observability and control. For an agent daemon, the valuable
use cases are:
- **Run-time process monitoring** without polling `/proc`
- **Network flow visibility** (which connections are being made)
- **File access tracing** with zero overhead (tracepoints, kprobes)
- **LSM programs** for custom security policies (allow/deny specific syscalls)

### Honest Assessment

**This is overkill for v1.** eBPF requires:
- Kernel compiled with `CONFIG_BPF=y`, `CONFIG_BPF_LSM=y`
- `CAP_BPF` + `CAP_SYS_ADMIN` or root
- The `aya` crate (pure Rust eBPF framework)
- A separate BPF compilation step (or pre-compiled `.o` files)

**Recommendation:** Skip for now. File as a future enhancement. The value
proposition doesn't justify the complexity unless Deskbrid becomes a security
monitoring agent.

---

## 9. Confinement Detection (Flatpak / Snap / SELinux / AppArmor)

**Status:** ✅ Done. Deskbrid exposes `system.confinement` and includes the same
report in `system.capabilities` and `system.health`. It detects Flatpak, Snap,
AppImage packaging, containers, WSL, AppArmor profiles, and SELinux mode.

### What's Missing

Portal-specific permission repair is future work. Deskbrid can now tell agents
when sandbox/container/security context is likely to affect behavior:
1. Tell the agent "you're sandboxed, here's what you can't do"
2. Request portal permissions when available
3. Give meaningful error messages instead of cryptic failures

### Implementation

**Detection (env vars + procfs):**

```rust
fn detect_confinement() -> Confinement {
    if std::env::var("container").as_deref() == Ok("flatpak-spawn") {
        return Confinement::Flatpak;
    }
    if std::env::var("SNAP").is_ok() {
        return Confinement::Snap;
    }
    if std::env::var("APPIMAGE").is_ok() {
        return Confinement::AppImage;
    }
    // AppArmor: check /proc/self/attr/current for "enforce"
    if let Ok(ctx) = std::fs::read_to_string("/proc/self/attr/current") {
        if ctx.trim() != "unconfined" {
            return Confinement::AppArmor(ctx.trim().into());
        }
    }
    // SELinux: check getenforce or /sys/fs/selinux/enforce
    if let Ok(enforce) = std::fs::read_to_string("/sys/fs/selinux/enforce") {
        if enforce.trim() == "1" {
            return Confinement::SELinux;
        }
    }
    Confinement::None
}
```

**Flatpak portals:** `zbus` already supports calling portal APIs. Key portals
for Deskbrid:

| Portal | Purpose | DBus Interface |
|---|---|---|
| Screenshot | Capture screen inside Flatpak | `org.freedesktop.portal.Screenshot` |
| Inhibit | Prevent idle/sleep | `org.freedesktop.portal.Inhibit` |
| OpenURI | Open URLs/files | `org.freedesktop.portal.OpenURI` |
| FileChooser | Select files | `org.freedesktop.portal.FileChooser` |
| NetworkMonitor | Network status | `org.freedesktop.portal.NetworkMonitor` |
| Background | Run in background | `org.freedesktop.portal.Background` |
| Account | User info | `org.freedesktop.portal.Account` |
| RemoteDesktop | Remote desktop (not in portal yet) | Mutter-specific |

### Protocol Changes

```rust
// Add to SystemInfo
SystemConfinement,
```

`SystemInfo` should include a `confinement` field:

```json
{
  "confinement": {
    "type": "flatpak",
    "portals_available": ["Screenshot", "Inhibit", "OpenURI"],
    "portals_missing": ["RemoteDesktop"]
  }
}
```

---

## 10. Desktop Portal Integration (XDG Portals)

### What's Missing

Deskbrid uses portals for screenshots (the `screenshot_portal.py` script), but
doesn't expose the full portal API. XDG Portals are the *official* way for
sandboxed apps and Wayland-native tools to access desktop services.

**Key portals an agent would want:**

| Portal | Action |
|---|---|
| `org.freedesktop.portal.Settings` | Read system settings (dark mode, font, accent color, cursor size) |
| `org.freedesktop.portal.Wallpaper` | Set desktop wallpaper (currently done via DE-specific APIs) |
| `org.freedesktop.portal.Inhibit` | Inhibit screen idle/screensaver/notifications |
| `org.freedesktop.portal.Notification` | Send notifications (Deskbrid already does this via DE-specific APIs, but portal version works everywhere) |
| `org.freedesktop.portal.GameMode` | Request game mode (CPU governor, scheduler) |
| `org.freedesktop.portal.Trash` | Move files to trash |
| `org.freedesktop.portal.Print` | Print documents |
| `org.freedesktop.portal.Camera` | Access camera |
| `org.freedesktop.portal.Location` | Get location (already in protocol as `LocationGet`, implement it via portal) |
| `org.freedesktop.portal.Email` | Compose email |

### Implementation

All portals are DBus interfaces on `org.freedesktop.portal.Desktop`. Already have
`zbus` — this is just calling methods on known object paths.

```rust
async fn get_system_settings(&self, key: &str) -> Result<serde_json::Value> {
    let reply = self.conn.call_method(
        Some("org.freedesktop.portal.Desktop"),
        "/org/freedesktop/portal/desktop",
        Some("org.freedesktop.portal.Settings"),
        "Read",
        &("org.gnome.desktop.interface", key),
    ).await?;
    // parse response
}
```

---

## 11. elogind (Non-systemd Systems)

### What's Missing

Deskbrid doesn't handle non-systemd distros at all for session management features.
Distros using elogind (Void, Alpine, Devuan) have the same `org.freedesktop.login1`
DBus interface but no systemd journal/unit control.

### Implementation

**No separate crate needed.** `zbus_systemd`'s `login1` module talks to the same
DBus interface whether it's systemd-logind or elogind. Just check if
`org.freedesktop.login1` is available on the system bus.

```rust
// Capability detection — check if login1 is available
async fn has_login1(&self) -> bool {
    let reply = self.conn.call_method(
        Some("org.freedesktop.DBus"),
        "/",
        Some("org.freedesktop.DBus"),
        "NameHasOwner",
        &("org.freedesktop.login1",),
    ).await;
    reply.map(|r| {
        r.body().deserialize::<bool>().unwrap_or(false)
    }).unwrap_or(false)
}
```

### For `systemd1` (unit/journal control)

Only available on systemd systems. When absent, these actions should return
`"code": "NOT_SUPPORTED"` with `"message": "systemd not available (non-systemd distro)"`.

---

## 12. OCR / Text Extraction

**Status:** ✅ Done on `main`.

### What's Missing

The screenshot pipeline works and returns PNG paths, but nothing extracts text from
those images. When the accessibility tree isn't available (games, Electron apps,
cross-platform toolkits, PDF viewers, terminal emulators), OCR is the fallback that
lets agents *read what they see*.

### Implementation

**Tool:** Tesseract OCR (`tesseract` CLI or `leptess` crate).

The screenshot pipeline already writes to `/tmp/deskbrid/`. Add a processing step:

```rust
// Simple CLI approach — no new Rust crate, just shell out
let output = Command::new("tesseract")
    .args([screenshot_path, "stdout", "-l", "eng"])
    .output().await?;
let text = String::from_utf8_lossy(&output.stdout);

// For structured output (bounding boxes + text):
// tesseract --psm 6 output.tsv
let tsv = Command::new("tesseract")
    .args([screenshot_path, "stdout", "-l", "eng", "tsv"])
    .output().await?;
```

**Crate approach:** `leptess` — Rust bindings to Leptonica + Tesseract. More
performant for repeated calls, but adds a build dependency on libtesseract dev
headers.

**Two integration paths:**

1. **Inline in screenshot response** — every `screenshot` action returns OCR text
   as an optional field. Simple but wasteful (screenshot is often needed visually).
2. **Separate action** — Add `ScreenshotOcr` that takes an
   existing screenshot path (or takes a fresh one) and returns extracted text with
   bounding boxes. Better separation.

### Protocol Actions

```rust
// OCR an existing screenshot by path, or take a fresh screenshot + OCR it
ScreenshotOcr {
    path: Option<String>,       // existing screenshot, or None = fresh capture
    language: Option<String>,   // "eng" default
    psm: Option<u32>,           // Tesseract page segmentation mode (default: 3)
    bounding_boxes: bool,       // return word-level positions (default: false)
}

// Returns:
{
    "text": "The extracted text content...",
    "confidence": 92.5,
    "words": [
        {"text": "The", "x": 10, "y": 20, "width": 30, "height": 15, "confidence": 95.0},
        ...
    ],
    "source_path": "/tmp/deskbrid/screenshot_12345.png"
}
```

### Dependency

- **System:** `tesseract-ocr`, `tesseract-ocr-eng` (and other language packs)
- **Optional Rust crate:** `leptess` (for in-process OCR without CLI overhead)

---

## 13. Terminal / PTY Multiplexer

**Status:** ✅ Done on `main`.

### What's Missing

This is Deskbrid's biggest gap for coding agents. `ProcessStart` is fire-and-forget
(stdin=null, stdout=null, stderr=null). An agent can run `ls` but can't:
- Run `apt install` and watch progress
- Pipe output between commands
- Handle interactive prompts (password, confirmations)
- Resize a terminal for structured output
- Send Ctrl+C to interrupt a running process

### Implementation

**Crate:** `portable-pty` (from the wezterm project) or raw `tokio::process::Command`
with pseudo-terminal (PTY) master/slave.

A new module (`src/terminal.rs`) maintaining PTY sessions per client.

```rust
pub struct TerminalSession {
    id: String,
    pid: u32,
    stdin: tokio::sync::mpsc::Sender<String>,
    stdout: tokio::sync::watch::Receiver<String>,
    size: (u16, u16),  // rows, cols
    created_at: u64,
}
```

**Architecture:**

```
Client ──create──→ Daemon allocates PTY, spawns shell
Client ──send────→ Daemon writes to PTY stdin
Client ──read────→ Daemon returns buffered stdout since last read
Client ──resize──→ Daemon sends SIGWINCH to PTY
Client ──kill────→ Daemon sends signal, cleans up PTY
```

**Key design decisions:**

- **PTY vs pipes:** PTY is required for interactive programs (they check `isatty()`).
  Pipes break `apt`, `less`, password prompts, colored output.
- **Buffering strategy:** Ring buffer of last N lines. Clients poll on read, or
  subscribe to push events for real-time output streaming.
- **Shell choice:** Default to `$SHELL`, fall back to `/bin/bash`. Let clients
  override.

### Protocol Actions

```rust
TerminalCreate {
    shell: Option<String>,        // $SHELL or /bin/bash
    cwd: Option<String>,          // working directory
    env: Option<HashMap<String, String>>,
    rows: Option<u16>,            // default 24
    cols: Option<u16>,            // default 80
}
// Returns: { "terminal_id": "t-001", "pid": 12345, "shell": "/bin/bash" }

TerminalWrite {
    terminal_id: String,
    input: String,                // text to send to stdin
}
// Returns: { "bytes_written": 5 }

TerminalRead {
    terminal_id: String,
    max_bytes: Option<u64>,       // cap output (default: 65536)
    flush: bool,                  // drain the buffer after read
}
// Returns: { "output": "user@host:~$ ", "bytes": 14, "closed": false }

TerminalResize {
    terminal_id: String,
    rows: u16,
    cols: u16,
}

TerminalList,                     // list all active terminal sessions

TerminalKill {
    terminal_id: String,
    signal: Option<String>,       // default: SIGHUP
}
```

### Subscription Events

```rust
// Push events for real-time terminal output
DeskbridEvent::TerminalOutput {
    terminal_id: String,
    data: String,
}
DeskbridEvent::TerminalClosed {
    terminal_id: String,
    exit_code: i32,
}
```

### Implementation Effort

~500-800 lines of Rust. The PTY abstraction is the trickiest part — need to handle
SIGCHLD, terminal size changes, and proper cleanup on client disconnect.

---

## 14. MPRIS Media Control

**Status:** ✅ Done. Deskbrid exposes `mpris.list`, `mpris.get`, and
`mpris.control` for standard session-bus MPRIS players, including playback status,
metadata, volume, and common transport controls.

### What's Missing

Seek, set-position, shuffle/repeat, raise, and quit controls are future work.

MPRIS (`org.mpris.MediaPlayer2.*`) is the freedesktop standard for media player
control. Every major player exposes it: Spotify, Rhythmbox, VLC, Firefox, Chromium.

### Implementation

Each media player registers its own bus name:
- `org.mpris.MediaPlayer2.spotify`
- `org.mpris.MediaPlayer2.vlc`
- `org.mpris.MediaPlayer2.firefox`
- `org.mpris.MediaPlayer2.chromium`

**Query pattern via zbus:**

```rust
// 1. Find all MPRIS players on the bus
let reply = conn.call_method(
    Some("org.freedesktop.DBus"),
    "/",
    Some("org.freedesktop.DBus"),
    "ListNames",
    &(),
).await?;
let names: Vec<String> = reply.body().deserialize()?;
let players: Vec<&str> = names.iter()
    .filter(|n| n.starts_with("org.mpris.MediaPlayer2."))
    .collect();

// 2. Call standard methods on each player
// org.mpris.MediaPlayer2.Player: Play, Pause, PlayPause, Stop, Next, Previous, Seek
// org.freedesktop.DBus.Properties: Get(interface, property) — PlaybackStatus, Metadata, Position

// 3. Read metadata
// Metadata map includes: xesam:title, xesam:artist, xesam:album, mpris:artUrl, mpris:length
```

### Protocol Actions

```rust
// List active media players
MediaPlayerList,
// Returns: [{"name": "spotify", "identity": "Spotify"}]

// Transport control
MediaPause { player: Option<String> },
MediaPlay { player: Option<String> },
MediaPlayPause { player: Option<String> },
MediaStop { player: Option<String> },
MediaNext { player: Option<String> },
MediaPrevious { player: Option<String> },
MediaSeek { player: Option<String>, offset: i64 },
MediaSetPosition { player: Option<String>, position: i64 },

// Query
MediaStatus { player: Option<String> },
// Returns: {
//   "player": "spotify",
//   "status": "Playing",
//   "metadata": { "title": "...", "artist": [...], "album": "...", "length": 354000000 },
//   "position": 120000000,
//   "volume": 0.75,
//   "shuffle": false,
//   "repeat": "Playlist"
// }

MediaVolume { player: Option<String>, volume: f64 },
MediaShuffle { player: Option<String>, shuffle: bool },
MediaRepeat { player: Option<String>, mode: String },

// Raise player window
MediaRaise { player: String },
MediaQuit { player: String },
```

### Effort

~300 lines of Rust. Straightforward zbus calls — Deskbrid already has the DBus
infrastructure.

---

## 15. Drag & Drop

**Status:** ✅ Done. Deskbrid exposes `input.mouse.drag` and maps it to
Mutter RemoteDesktop on GNOME, `ydotool` on Wayland backends, and `xdotool` on X11.

### What's Missing

`InputMouse` supports move, click (press-release), and scroll — but no press-move-release
sequence. Agents can't:
- Drag files between file manager windows
- Reorder items in design tools
- Drag to resize panels
- Drag-and-drop into browser upload zones

### Implementation

Small gap. Each backend just needs a sequence command.

**Mutter RemoteDesktop (GNOME):**
```rust
async fn mouse_drag(&self, from: (f64, f64), to: (f64, f64), button: i32) -> Result<()> {
    self.rd_call("NotifyPointerButton", &(button, true)).await?;  // press
    self.rd_call("NotifyPointerMotion", &(to.0, to.1)).await?;    // move
    self.rd_call("NotifyPointerButton", &(button, false)).await?; // release
}
```

**ydotool (KDE/Hyprland):** `ydotool mousedown 1 && ydotool mousemove --absolute X Y && ydotool mouseup 1`

**xdotool (X11):** `xdotool mousedown 1 && xdotool mousemove X Y && xdotool mouseup 1`

### Protocol Changes

Add a dedicated action rather than extending the overloaded `InputMouse`:

```rust
InputMouseDrag {
    from_x: f64,
    from_y: f64,
    to_x: f64,
    to_y: f64,
    button: Option<String>,       // "left" (default), "middle", "right"
    duration_ms: Option<u64>,     // animation duration
}
```

### Effort

~100 lines. The backend trait gets one new method, 4 backends implement it.

---

## 16. Application Menu Catalog

**Status:** ✅ Done. Deskbrid scans standard XDG application directories for
`.desktop` files and exposes `apps.list`, `apps.search`, and `apps.get` through
the protocol, CLI, and Python client.

### What's Missing

Running application menubar exploration via DBus menu model remains future work.

### Implementation

**A) .desktop file parser (simple, high value)**

Scan `$XDG_DATA_DIRS/applications/`:
```rust
fn parse_desktop(path: &Path) -> Result<DesktopEntry> {
    // .desktop files are INI-format
    // Extract: Name, Exec, Icon, Categories, MimeType, NoDisplay, Terminal
}
```

**B) DBus menu model (complex, specialized — future)**

`com.canonical.dbusmenu` interface for querying running application menus.
~1000+ lines. Ship the `.desktop` catalog first.

### Protocol Actions

```rust
// Phase 1 — App catalog
AppList {
    categories: Option<Vec<String>>,
    mime_types: Option<Vec<String>>,
    include_hidden: bool,
}
AppSearch { query: String, limit: Option<u32> },
AppGet { app_id: String },

// Phase 2 — Running app menus (future)
AppWinMenuGet { window_id: String, depth: Option<u32> },
AppWinMenuActivate { window_id: String, menu_path: Vec<String> },
```

---

## 17. Screen Recording (Finish Half-Built Implementation)

### Current Status

The protocol has `screencast.start` and `screencast.stop` in `public_action_types()`
but both return `"supported": false`.

The GNOME backend already has a real ScreenCast session:
```rust
// src/backend/gnome.rs:
fn init_screen_cast() {
    // Creates a Mutter ScreenCast session via DBus
    // Records a monitor → gets a stream path (self.sc_stream_path)
}
```

The stream path is only used for absolute mouse positioning. The video data is
never delivered to clients.

### What Needs to Be Built

**A) PipeWire stream reader**
Mutter ScreenCast outputs video via PipeWire. Deskbrid has `pipewire` and `spa`
crates as optional deps. Read the stream frames from the PipeWire node.

**B) Recording lifecycle**
```rust
ScreencastStart {
    monitor: Option<u32>,
    framerate: Option<u32>,        // default: 15
    output_format: Option<String>, // "frames", "mp4"
    max_duration_secs: Option<u64>,
}
ScreencastStop,
```

**C) Encoding**
- **Option 1 (simple):** Save frames as PNGs. Agent processes later.
- **Option 2 (moderate):** Pipe through FFmpeg subprocess for real-time encoding.
- **Option 3 (complex):** In-process via gstreamer Rust bindings.

**Recommendation:** Start with Option 1, then Option 2.

### Events

```rust
DeskbridEvent::ScreencastFrame {
    path: String, timestamp: u64, frame_number: u32,
}
DeskbridEvent::ScreencastStopped {
    frames: u32, duration_secs: u64, output_path: Option<String>,
}
```

### Effort

400-600 lines. The PipeWire stream reader is the hardest part.

---

## 18. Clipboard History

**Status:** ✅ Done. Deskbrid stores a bounded in-memory history of clipboard text
observed through `clipboard.read` and `clipboard.write`, dedupes consecutive
duplicates, and exposes `clipboard.history` / `clipboard.history.clear` through
the protocol, CLI, and Python client.

### What's Missing

Background clipboard watching, restore-by-entry, and persistence across daemon
restarts are still future work.

### Implementation

Add an in-memory ring buffer in the daemon that records every clipboard write:

```rust
pub struct ClipboardHistory {
    entries: Vec<ClipboardEntry>,
    max_entries: usize,    // configurable (default: 50)
    current_index: usize,  // which entry is the "live" one
}

pub struct ClipboardEntry {
    text: String,
    copied_at: u64,        // unix timestamp
    source: Option<String>, // app_id that wrote it (from peer_uid if available)
}
```

**Key design:** Intercept `ClipboardWrite` actions in the dispatch layer to
auto-store the entry. Weekly users may run `ClipboardRead` periodically — make
reads also check for new content and auto-log it (the way macOS clipboard managers
do).

**Persistence:** Keep in-memory by default. Optionally persist to
`~/.local/share/deskbrid/clipboard_history.json` for survival across daemon
restarts. Cap at ~1MB to avoid bloat.

### Protocol Actions

```rust
// Get clipboard history
ClipboardHistory {
    limit: Option<u32>,      // how many entries (default: 20)
    search: Option<String>,  // text search filter
    since: Option<u64>,      // only entries after this timestamp
}
// Returns: {
//   "entries": [
//     {"text": "...", "copied_at": 1747000000, "source": "firefox"},
//     ...
//   ],
//   "total": 50,
//   "returned": 20
// }

// Restore a specific history entry
ClipboardRestore {
    index: u32,              // which entry (0 = most recent)
}
// Returns: { "restored": "text snippet...", "index": 0 }

// Clear history
ClipboardClearHistory,

// Configure max entries
ClipboardSetMaxHistory {
    max_entries: u32,        // default: 50, max: 500
}
```

### Subscription Events

```rust
DeskbridEvent::ClipboardChanged {
    text: String,
    source: Option<String>,
    history_index: u32,
    timestamp: u64,
}
```

### Effort

~200 lines. Simple ring buffer, no new dependencies.

---

## 19. Window Tiling Presets

**Status:** ✅ Done. Deskbrid exposes `windows.tile` as a helper over
`window_move_resize`, with monitor selection, named presets, optional padding,
CLI support, and Python client bindings.

### What's Missing

Future refinements: compositor-native tiling integrations where they give better
results than pixel geometry, smooth transitions, and richer layout presets.

### Implementation

High-level tile function over the existing `window_move_resize` backend method:

```rust
fn tile_window(backend, window_id, preset: TilePreset, monitor: Option<u32>) -> Result<()> {
    let info = backend.system_info()?;
    let mon = resolve_monitor(&info, monitor, window_id)?;
    let (x, y, w, h) = match preset {
        TilePreset::Left => (0, 0, mon.width / 2, mon.height),
        TilePreset::Right => (mon.width as i32 / 2, 0, mon.width / 2, mon.height),
        TilePreset::Top => (0, 0, mon.width, mon.height / 2),
        TilePreset::Bottom => (0, mon.height as i32 / 2, mon.width, mon.height / 2),
        TilePreset::TopLeft => (0, 0, mon.width / 2, mon.height / 2),
        TilePreset::TopRight => (mon.width as i32 / 2, 0, mon.width / 2, mon.height / 2),
        TilePreset::BottomLeft => (0, mon.height as i32 / 2, mon.width / 2, mon.height / 2),
        TilePreset::BottomRight => (mon.width as i32 / 2, mon.height as i32 / 2, mon.width / 2, mon.height / 2),
        TilePreset::Center => (mon.width as i32 / 4, mon.height as i32 / 4, mon.width / 2, mon.height / 2),
        TilePreset::Fill => (0, 0, mon.width, mon.height),
    };
    // Apply monitor offset
    backend.window_move_resize(window_id, x + mon.x, y + mon.y, w, h)
}
```

**DE-specific shortcuts (alternative to pixel math):**

| DE | Tiling shortcut |
|---|---|
| GNOME | `wmctrl -r $window -e 0,$x,$y,$w,$h` via keybinding simulation |
| Hyprland | `hyprctl dispatch movewindow l/r/u/d` + `hyprctl dispatch resizewindow` |
| KDE | KWin script via DBus: `setMinimize()` + `setMaximize()` |
| X11 | `xdotool windowsize $id $w $h` + `xdotool windowmove $id $x $y` |

For Hyprland, consider using native tiling dispatcher bindings
(`split:false`, `movefocus`, etc.) instead of pixel positioning since Hyprland is
a dynamic tiling WM.

### Protocol Actions

```rust
WindowsTile {
    window_id: String,
    preset: String,            // "left", "right", "top", "bottom",
                               // "top_left", "top_right", "bottom_left", "bottom_right",
                               // "center", "fill"
    monitor: Option<u32>,      // specific monitor, or None = current
    animate: Option<bool>,     // smooth transition (future)
}
```

### Effort

~150 lines. Purely a helper over existing `window_move_resize`, no backend changes needed.

---

## 20. Color Picker

**Status:** ✅ Done. Deskbrid exposes `color.pick` for sampling a pixel from an
existing image path or a fresh 1x1 screen capture, returning RGBA channels and a
hex string.

### What's Missing

HSL formatting and monitor-relative coordinate helpers are future refinements.

### Implementation

Trivial bolt-on to the existing screenshot pipeline:

```rust
fn sample_pixel(screenshot_path: &str, x: u32, y: u32) -> Result<Color> {
    // Option 1: ImageMagick (already a dep for many backends)
    let output = Command::new("convert")
        .args([screenshot_path, "-crop", &format!("1x1+{}+{}", x, y), "-depth", "8", "txt:-"])
        .output()?;
    // Parse: "0,0: (255, 0, 0)  #FF0000  red"

    // Option 2: `image` crate (already a Cargo.toml dep for pipewire feature)
    let img = image::open(screenshot_path)?;
    let pixel = img.get_pixel(x, y);
    Color { r: pixel[0], g: pixel[1], b: pixel[2], a: pixel[3] }
}
```

Deskbrid already has `image` as an optional dependency (for PipeWire screenshots).
The `image` crate approach is cleaner — no subprocess overhead.

### Protocol Actions

```rust
// Sample a pixel from a fresh screenshot
ColorPicker {
    x: u32,
    y: u32,
    monitor: Option<u32>,   // which monitor to capture first
    format: Option<String>, // "hex" (default), "rgb", "hsl"
}
// Returns: {
//   "hex": "#FF0000",
//   "rgb": {"r": 255, "g": 0, "b": 0},
//   "hsl": {"h": 0.0, "s": 1.0, "l": 0.5},
//   "position": {"x": 100, "y": 200, "monitor": 0}
// }

// Sample from an existing screenshot
ColorPickerFromFile {
    path: String,
    x: u32,
    y: u32,
}
```

### Effort

~80 lines. Trivial. Uses the `image` crate already in the dependency tree.

---

## 21. Desktop Settings (Read/Write Configuration)

### What's Missing

Deskbrid has no way to read or change desktop environment settings. Agents can't:
- Toggle dark mode
- Change accent color
- Adjust font size (accessibility)
- Enable/disable tap-to-click
- Remap keyboard shortcuts
- Change wallpaper (already planned via portal in section 10)

### Implementation

Each DE has its own settings backend:

**GNOME — dconf (via gsettings CLI or DBus):**

```rust
// Read
let out = Command::new("gsettings")
    .args(["get", "org.gnome.desktop.interface", "color-scheme"])
    .output()?;

// Write
Command::new("gsettings")
    .args(["set", "org.gnome.desktop.interface", "color-scheme", "'prefer-dark'"])
    .status()?;
```

**KDE — kconfig (via `kwriteconfig5`):**

```rust
Command::new("kwriteconfig5")
    .args(["--file", "kdeglobals", "--group", "General", "--key", "ColorScheme", "BreezeDark"])
    .status()?;
```

**Hyprland — config file:**

```rust
// Hyprland settings are read from ~/.config/hypr/hyprland.conf
// Changes need to be applied via `hyprctl reload` or keyword dispatcher
Command::new("hyprctl").args(["keyword", "general:gaps_in", "5"]).status()?;
```

**X11 General — xsettings:**

```rust
// xsettings DBus protocol for GTK settings
// Or just write to dconf/gsettings which works on X11 too
```

**Portal-based (cross-DE):**
The XDG Settings portal (`org.freedesktop.portal.Settings`) provides read-only
access to a subset of settings (dark mode, font, accent color). Write is portal
version-dependent.

### Protocol Actions

```rust
// Read a setting — DE-agnostic abstract names
SettingsGet {
    key: String,
    // Abstract keys (daemon maps to DE-specific):
    // "interface.color_scheme"  → "prefer-dark" / "prefer-light" / "default"
    // "interface.font_name"    → "Cantarell 11"
    // "interface.font_size"    → 11
    // "interface.accent_color" → "#3584e4"
    // "interface.cursor_size"  → 24
    // "interface.tap_to_click" → true/false
    // "interface.wallpaper"    → "file:///path/to/image.jpg"
    // "keyboard.layout"        → "us"
    // "power.screen_timeout"   → seconds (0 = never)
    // "power.suspend_timeout"  → seconds (0 = never)
}

// Write a setting
SettingsSet {
    key: String,
    value: serde_json::Value,  // typed value matching the key
    persist: Option<bool>,     // default: true (save to disk)
}
```

**Returns:**

```json
{
  "key": "interface.color_scheme",
  "value": "prefer-dark",
  "backend": "gsettings",
  "path": "org.gnome.desktop.interface color-scheme",
  "writable": true
}
```

### Effort

~300 lines. Mostly gsettings/DBus calls, plus a key-mapping layer from abstract names
to DE-specific backends.

---

## 22. Keyboard Layout Management

### What's Missing

Deskbrid can inject keystrokes but has no awareness of the active keyboard layout.
An agent can't:
- Know whether it's typing in QWERTY, AZERTY, or Dvorak
- Switch layouts programmatically
- List available layouts
- Type special characters that require non-current layouts

### Implementation

**GNOME:**

```rust
// List layouts
let out = Command::new("gsettings")
    .args(["get", "org.gnome.desktop.input-sources", "sources"])
    .output()?;
// Returns: [("xkb", "us"), ("xkb", "de")]

// Get active layout index
let out = Command::new("gsettings")
    .args(["get", "org.gnome.desktop.input-sources", "current"])
    .output()?;
// Returns: uint32 0

// Switch layout
Command::new("gsettings")
    .args(["set", "org.gnome.desktop.input-sources", "current", "1"])
    .status()?;

// Or via DBus (more reliable):
// org.gnome.Shell.Eval: imports.ui.status.keyboard.getCurrentInputSource()
```

**KDE:**

```rust
// List: read ~/.config/kxkbrc for LayoutList=us,de
// Switch: qdbus6 org.kde.keyboard /Layouts org.kde.KeyboardLayouts.setLayout 1
```

**Hyprland:**

```rust
// hyprctl devices -> active keymap
// hyprctl switchxkblayout <device> <index>
```

**X11:**

```rust
// setxkbmap us
// setxkbmap -query  → current layout
// setxkbmap -layout us,de  → available layouts
```

### Protocol Actions

```rust
// List available keyboard layouts
InputListLayouts,
// Returns: ["us (English US)", "de (German)", "fr (French AZERTY)", ...]

// Get current layout
InputGetLayout,
// Returns: {"index": 0, "short_name": "us", "full_name": "English (US)", "variant": ""}

// Switch to a layout by index or name
InputSetLayout {
    index: Option<u32>,    // by index
    name: Option<String>,  // by short name (e.g. "de", "fr")
    variant: Option<String>, // e.g. "dvorak", "colemak"
}

// Add/remove a layout
InputAddLayout {
    name: String,
    variant: Option<String>,
}
InputRemoveLayout {
    index: u32,
}
```

### Subscription Events

```rust
DeskbridEvent::KeyboardLayoutChanged {
    index: u32,
    short_name: String,
    full_name: String,
}
```

### Effort

~250 lines. Straightforward per-DE commands.

---

## 23. Session & User Management

### What's Missing

Deskbrid has `SystemPower` (shutdown/reboot/suspend) but no user session control.
Agents can't:
- Lock the screen (requires knowing session is locked)
- Switch to another user
- List logged-in users
- Know who is currently active on the machine

### Implementation

This overlaps with section 1 (systemd logind). The `zbus_systemd` crate's `login1`
module provides all the DBus interfaces needed:

```rust
use zbus_systemd::login1::ManagerProxy;

// Lock screen
let manager = ManagerProxy::new(&conn).await?;
manager.lock_session(session_id).await?;

// List sessions
let sessions = manager.list_sessions().await?;
// Each session has: session_id, uid, username, seat_id, display

// Switch to another user (VT switch)
manager.activate_session_on_seat(session_id, seat_id).await?;
```

For screen lock without logind (e.g., locking the current session):

```rust
// GNOME: gdbus call --session --dest org.gnome.ScreenSaver ...
// KDE: qdbus6 org.kde.screensaver ...
// Hyprland: hyprctl dispatch lockscreen
// X11: xscreensaver-command -lock
```

### Protocol Actions

```rust
// Session management
SessionLock,
SessionUnlock,  // if password is available (agent-managed)
SessionGetInfo,
// Returns: {
//   "active_session_id": "1",
//   "active_user": "jeremy",
//   "seat": "seat0",
//   "locked": false,
//   "type": "wayland",
//   "display": ":1",
//   "idle_seconds": 300
// }

// User management
SessionListUsers,
// Returns: [{"uid": 1000, "name": "jeremy", "display_name": "Jeremy"}, ...]

SessionListSessions,
// Returns: [{"id": "1", "uid": 1000, "user": "jeremy", "seat": "seat0", "state": "active"}, ...]

SessionSwitchUser {
    username: String,
}

// Per-user actions (if daemon runs as root via polkit)
UserLock {
    username: Option<String>,  // lock a specific user's session
}
```

### Subscription Events

```rust
DeskbridEvent::SessionLocked {
    username: String,
    session_id: String,
    timestamp: u64,
}
DeskbridEvent::SessionUnlocked {
    username: String,
    session_id: String,
    timestamp: u64,
}
```

### Effort

~150 lines via `zbus_systemd` login1 module + ~100 lines per-DE for screen lock
without logind.

---

## 24. Screenshot Diffing

**Status:** ✅ Done on `main`.

### What's Missing

Agents can take screenshots but can't compare two to detect what changed. Common
scenarios: wait for a page to finish loading, detect popups appearing, verify UI
actions had the expected visual effect, monitor for visual regressions.

### Implementation

Simple pixel-level diff with configurable tolerance using the `image` crate (already
an optional dependency):

```rust
fn diff_images(before, after, tolerance) -> Result<DiffResult> {
    // Compare pixel-by-pixel within tolerance
    // Return: percent_changed, diff_image, bounding_boxes of changed regions
}
```

### Protocol Actions

```rust
ScreenshotDiff {
    before_id: String,             // reference screenshot ID
    after_id: Option<String>,      // None = fresh screenshot now
    tolerance: Option<u8>,         // 0-255, default: 10
    bounding_boxes: bool,          // return change regions
    save_diff: bool,               // save highlighted diff image
}
// Returns: { "changed": true, "changed_pixels": 15420, "percent_changed": 0.74, ... }

ScreenshotSave { name: String },
ScreenshotLoad { name: String },
ScreenshotList,
```

### Subscription Events

```rust
DeskbridEvent::ScreenChanged { diff_percent, screenshot_id, timestamp }
```

### Effort

~200 lines. No new dependencies (`image` crate already in tree).

---

## 25. Action Recording & Replay (Macros)

### What's Missing

Agents can execute individual actions but can't record sequences for replay, loop
actions, share macros, or parameterize them with variables.

### Implementation

Intercept actions in the dispatch layer, serialize to `Vec<RecordedAction>`, store
as JSON files in `~/.local/share/deskbrid/macros/`.

**Replay modes:** fast (no delays), timed (preserve timing), stepped (await approval).

### Protocol Actions

```rust
MacroRecordStart { name: String, description: Option<String> },
MacroRecordStop,
// Returns: { "macro_name": "...", "actions_recorded": 24 }

MacroReplay {
    name: String,
    mode: Option<String>,      // "fast", "timed", "stepped"
    loop_count: Option<u32>,
    stop_on_error: Option<bool>,
    variables: Option<HashMap<String, serde_json::Value>>,
}

MacroList, MacroGet { name: String }, MacroDelete { name: String },
MacroExport { name: String }, MacroImport { name: String, data: String },
```

### Effort

~400 lines. Recording layer in dispatch, JSON file storage, replay loop.

---

## 26. Wait-for Conditions

**Status:** ✅ Done on `main`.

### What's Missing

There's a `wait` CLI command but no protocol-level action. Agents must poll
manually — wasteful and slow. Proper wait-for lets the daemon poll efficiently.

### Implementation

Daemon spawns a lightweight polling task per condition:

```rust
enum WaitCondition {
    WindowTitle(String, u64),
    WindowExists(String, u64),
    ClipboardContains(String, u64),
    ProcessExits(u32, u64),
    ScreenshotStable(u8, u64, u64),  // tolerance, stable_ms, timeout
    FileExists(String, u64),
    FileContent(String, String, u64), // path, pattern, timeout
    IdleSeconds(u64, u64),
    ...
}
```

Polling: 100ms start, back off to 1s for long waits.

### Protocol Actions

```rust
Wait {
    condition: String,             // "window_title", "clipboard_contains", etc.
    params: serde_json::Value,
    timeout_ms: u64,
    interval_ms: Option<u64>,      // poll interval (default: 200)
}
```

### Subscription Events

```rust
DeskbridEvent::WaitMatched { wait_id, condition, value, elapsed_ms }
```

### Effort

~300 lines. Polling loop + condition-specific checks.

---

## 27. Cron / Scheduled Actions

### What's Missing

No built-in scheduling. Agents can't run health checks every 5 minutes, take daily
screenshots, or do idle-time maintenance without external cron.

### Implementation

Embed a lightweight scheduler in the daemon. Store jobs in
`~/.local/share/deskbrid/cron.json`. Use the `cron` crate for schedule parsing.

```rust
pub struct ScheduledJob {
    id: String, schedule: CronSchedule, action: Action,
    enabled: bool, next_run: u64, max_runs: Option<u32>,
}
```

Schedule types: `*/5 * * * *` (cron), `interval:300` (seconds), `daily:09:00`,
`once:1747000000`, `idle:600` (after N seconds idle).

### Protocol Actions

```rust
CronCreate { name, schedule, action, enabled, max_runs },
CronList, CronGet, CronDelete, CronPause, CronResume,
CronRunNow { job_id: String },
```

### Subscription Events

```rust
DeskbridEvent::CronJobExecuted { job_id, name, result, duration_ms }
DeskbridEvent::CronJobCompleted { job_id, name, total_runs }
```

### Effort

~350 lines. `cron` crate, async tokio timer per job, JSON persistence.

---

## 28. D-Bus Raw Access (Escape Hatch)

### What's Missing

No matter how many actions Deskbrid wraps, there will always be a D-Bus interface
it doesn't cover. Agents need a direct D-Bus call escape hatch. Deskbrid already
has `zbus` — this exposes it.

### Implementation

```rust
DbusCall {
    bus: String,              // "session" or "system"
    destination: String,      // "org.freedesktop.DBus"
    path: String,             // "/org/freedesktop/DBus"
    interface: Option<String>,
    method: String,           // "ListNames"
    args: Vec<serde_json::Value>,
}
```

**Security:** Req`dbus.*` permission, polkit check for system bus, denylist for
dangerous interfaces.

### Protocol Actions

```rust
DbusCall { bus, destination, path, interface, method, args, timeout_ms },
DbusIntrospect { bus, destination, path },
DbusListServices { bus },
```

### Effort

~200 lines. JSON→zbus variant serialization + safety checks.

---

## 29. Secret / Keyring Access

### What's Missing

Agents need credentials (API keys, passwords, tokens). Currently must hardcode or
pass over the wire. Integrate with the system keyring.

### Implementation

**GNOME Keyring** via `secret-service` crate (freedesktop Secret Service API).  
**KDE KWallet** via DBus `org.kde.KWallet`.  
**Fallback:** AES-256-GCM encrypted file at `~/.config/deskbrid/keyring.json.aes`.

### Protocol Actions

```rust
SecretStore { service, key, value, attributes },
SecretGet { service, key },
SecretSearch { service, attributes },
SecretDelete { service, key },
SecretBackends,
```

### Dependencies

`secret-service` crate (Rust Secret Service bindings). Fallback: `aes-gcm` + `argon2`.

### Effort

~300 lines.

---

## 30. TCP Mode (Network Control)

### What's Missing

Deskbrid only listens on a Unix socket. Remote machines can't connect. Agents can't
control other computers on the network.

### Implementation

Add optional TCP listener alongside the Unix socket with TLS and token auth:

```bash
deskbrid daemon --tcp 0.0.0.0:7890              # plain TCP
deskbrid daemon --tcp 0.0.0.0:7890 --tls         # TLS
```

**Auth:** Self-signed TLS cert (generated on first run) + `Authorization: Bearer <token>`.

### Protocol Actions

```rust
DaemonConfigSet {
    tcp_bind, tls_enabled, tls_cert, tls_key, tcp_token,
},
DaemonConfigGet,
```

### Effort

~400 lines. TCP listener, TLS, token auth handshake.

---

## 31. Named Sessions (Multi-Agent Isolation)

### What's Missing

All clients share daemon state. Two agents see each other's subscriptions, share
clipboard history, and race on operations.

### Implementation

Per-session isolated state:

```rust
pub struct SessionState {
    id, name, peer_uid,
    subscriptions, hotkeys, watched_paths,
    terminal_sessions, clipboard_history, variables,
}
```

First connect message specifies session: `{"type": "connect", "session": "agent-alpha"}`.
Default session `"default"` for backward compatibility.

### Protocol Actions

```rust
SessionCreate { name, clone_from },
SessionDestroy { name },
SessionList,
SessionSwitch { name },
SessionVarSet { name, value }, SessionVarGet, SessionVarList,
```

### Effort

~250 lines.

---

## 32. Remote Screenshot Streaming

### What's Missing

Each screenshot requires a round trip. For remote control, agents need a
low-framerate stream to see what's happening in near real-time.

### Implementation

Stream JPEG-encoded frames over push events. Resize to streaming resolution
(e.g. 1280x720), encode at JPEG quality 70.

**Streaming modes:** full frames, diff frames (only changed regions).

### Protocol Actions

```rust
ScreencapStreamStart { interval_ms, quality, max_width, max_height, diff_mode, monitor },
ScreencapStreamStop,
```

### Subscription Events

```rust
DeskbridEvent::ScreencapFrame { data_base64, width, height, timestamp, frame_number }
```

### Effort

~300 lines. JPEG via `image` crate, streaming loop, event broadcasting.

---

## 33. Dry-Run Mode

**Status:** ✅ Done. Request envelopes can include `dry_run: true`. The daemon
performs permission checks, returns would-execute metadata, skips backend loading
and execution, and records the result in the audit log.

### What's Missing

Per-action detailed change previews are future work. The shipped version is a
safe dispatcher-level validation pass.

### Implementation

Add `dry_run: true` field to the standard envelope. On dry run:
- Read-only actions execute normally (no side effects)
- Write actions return what they would do without executing

```json
{"type": "windows.close", "id": "1", "window_id": "terminal", "dry_run": true}
```

```json
{"status": "ok", "data": {
  "dry_run": true, "would_execute": true,
  "would_change": ["window terminal-3 would be closed"],
  "permissions": {"allowed": true}
}}
```

### Effort

~80 lines. Check `dry_run` flag in dispatch layer, skip write execution.

---

## 34. Audit Log

**Status:** ✅ Done. Deskbrid records every dispatched action in a bounded in-memory
ring buffer and exposes `audit.log` / `audit.clear` through the protocol, CLI, and
Python client. Entries include action type, caller UID, sequence, status, duration,
and error message, but intentionally omit full action payloads to avoid copying
clipboard text, command arguments, or future secrets into the log.

### What's Missing

Persistent JSONL/journald export and subscription events are still future work.
The current shipped scope is the queryable in-memory audit trail.

### Implementation

Structured audit log recording every action:
- **Memory:** Ring buffer (last 1000 entries), queryable via protocol
- **JSONL:** Append to `~/.local/share/deskbrid/audit.jsonl`
- **Journald:** Systemd journal (when available)

### Protocol Actions

```rust
AuditLog { since, until, limit, offset, uid, action, status },
AuditLogExport { format, since, until },
AuditLogClear,
```

### Subscription Events

```rust
DeskbridEvent::AuditEntry { seq, uid, action, status, timestamp }
```

### Effort

~200 lines. Ring buffer + JSONL file writer.

---

## 35. Rate Limiting Per Client

**Status:** ✅ Done. Deskbrid applies a per-UID token bucket before permission
checks. Defaults are `DESKBRID_RATE_LIMIT_PER_SEC=30` and
`DESKBRID_RATE_LIMIT_BURST=120`; setting the rate to `0` disables limiting.
Limited requests return `RATE_LIMITED` with `retry_after_ms` and are recorded in
the audit log.

### What's Missing

Per-action category limits and `permissions.toml` overrides are future work. The
shipped version is a daemon-wide per-UID bucket.

### Implementation

Per-connection token bucket:

| Action Category | Rate | Burst |
|---|---|---|
| `windows.*` | 20/s | 10 |
| `input.*` | 50/s | 30 |
| `screenshot` | 2/s | 3 |
| `files.*` | 10/s | 5 |
| `process.*` | 5/s | 3 |
| * (default) | 30/s | 15 |

Configurable via `permissions.toml`:

```toml
[rate_limits]
default_rate = 30
default_burst = 15
[rate_limits.action]
screenshot = { rate = 2, burst = 3 }
```

Error: `{"error": {"code": "RATE_LIMITED", "retry_after_ms": 500}}`

### Protocol Actions

```rust
RateLimitGet,
```

### Effort

~200 lines. Token bucket per connection, configurable via permissions.toml.

---

## 36. Sandboxed Agent Profiles

**What's Missing:** Permissions.toml is global glob patterns. Agents can't have
named profiles with explicit allow/deny sets, action types, rate limit overrides,
and audit scoping — all bundled into a portable "profile" that can be applied to
a named session.

**Implementation:** Extend permissions.toml with profile blocks:

```toml
[profile.code-agent]
allow = ["windows.*", "input.*", "clipboard.*", "files.read", "files.write", "terminal.*", "process.start"]
deny = ["files.delete", "system.power", "bluetooth.*", "dbus.*"]
audit_level = "all"  # "all", "destructive", "none"
rate_limits = { default_rate = 30, screenshot = { rate = 2, burst = 3 } }
```

Named sessions (section 31) reference a profile on creation:
`SessionCreate { name: "agent-alpha", profile: "code-agent" }`.

**Protocol:** Same as existing permission actions. Profiles are server-side config.

**Effort:** ~200 lines. Profile parsing + session→profile binding.

---

## 37. Action Confirmation Mode

**What's Missing:** Destructive actions (file delete, system power, process kill)
execute immediately. No "are you sure?" guard for high-cost operations.

**Implementation:** Add a `confirm` flag to the envelope:

```json
{"type": "files.delete", "path": "/home/user/data", "confirm": true}
```

Daemon responds with:
```json
{"status": "action_requires_confirmation", "action": "files.delete", "id": "req-42", "details": "Delete /home/user/data (4 files, 2.3MB)"}
```

Client sends confirmation:
```json
{"type": "confirm", "id": "req-42"}
```

**Configurable in profile:** Which categories require confirmation. Default:
`confirm = ["files.delete", "system.power", "process.kill"]`.

**Effort:** ~150 lines. Pending confirmation queue in dispatch layer.

---

## 38. Canary Actions & Auto-Suspend

**What's Missing:** No way to detect if an agent is malfunctioning or compromised.
A buggy agent could hammer the daemon indefinitely.

**Implementation:** Periodically inject a "canary" action that the agent is expected
to respond to (respond in <2s with a specific nonce). If the agent misses N canaries
in a row:
1. Log the incident to audit
2. Suspend the agent's session
3. Notify the user (desktop notification)
4. Require explicit `SessionResume { name }` to reactivate

```rust
auto_suspend = {
    canary_interval_ms = 30000,  // check every 30s
    missed_threshold = 3,        // suspend after 3 missed
    suspend_actions = true,       // also suspend on suspicious action patterns
    suspicious_patterns = [
        ">10 windows.focus in 1s",   // focus spam
        ">5 files.delete in 10s",    // deletion spree
        "process.start with rm -rf", // dangerous command
    ]
}
```

**Effort:** ~250 lines. Canary timer, pattern detection, auto-suspend state.

---

## 39. User Presence Events

**What's Missing:** `SystemIdle` exists but it's a poll. No push events for user
presence state changes.

**Implementation:** Add a background task that monitors idle time via logind
(`org.freedesktop.login1.Manager` → `IdleHint` property) or XDG screensaver DBus.
Broadcast events on state changes.

### Protocol Actions & Events

```rust
// Query current presence
PresenceGet,
// Returns: { "state": "active", "idle_seconds": 0, "last_active": 1747000000, "locked": false }

// Subscription events (via existing subscribe/unsubscribe)
DeskbridEvent::PresenceActive { timestamp }
DeskbridEvent::PresenceIdle { idle_seconds, timestamp }
DeskbridEvent::PresenceReturned { idle_duration_secs, timestamp }
DeskbridEvent::PresenceLocked { timestamp }
DeskbridEvent::PresenceUnlocked { timestamp }

// Configure thresholds
PresenceConfig {
    idle_threshold_secs: Option<u64>,   // default: 300 (5 min)
    away_threshold_secs: Option<u64>,   // default: 900 (15 min)
}
```

**Effort:** ~150 lines. Monitor idle hint on logind + screensaver DBus signals.

---

## 40. Time-of-Day & Location Awareness

**What's Missing:** Agents have no concept of local time, timezone, or location
even though the daemon knows the system clock.

**Implementation:** Attach to every response automatically or via explicit action:

```rust
SystemTimeInfo,
// Returns: {
//   "local_time": "2026-05-20T14:30:00-04:00",
//   "unix_timestamp": 1747765800,
//   "timezone": "America/New_York",
//   "timezone_offset": -14400,  // seconds from UTC
//   "dst_active": true,
//   "uptime_seconds": 864000,
//   "boot_time": 1746900000,
//   "day_of_week": 3,           // 0=Sunday
//   "hour_of_day": 14,
//   "is_business_hours": true,  // Mon-Fri 9-17
//   "location": {               // from GeoClue/Geoclue or config
//     "timezone": "America/New_York",
//     "country_code": "US",
//     "region": "Indiana"
//   }
// }
```

**Optionally:** Auto-attach `local_time` and `timezone` to every response envelope
so agents always know the time without an extra round trip.

**Effort:** ~80 lines. `chrono` crate + system timezone file.

---

## 41. Element Detection via Screenshot (CV)

**What's Missing:** OCR extracts text (section 12), but agents can't find
"the blue button" or "the search icon" visually. Template matching or ML-based
detection finds UI elements by appearance.

**Implementation:** Two approaches:

**A) Template matching** (no ML, works for known UIs):
```rust
// Find all instances of a template image in a screenshot
let needle = image::open("template_save_button.png")?;
let haystack = screenshot::capture()?;
let matches = template_matching(&needle, &haystack, 0.8)?;
// Returns: [{x:100, y:200, confidence:0.95}, ...]
```

**B) ML-based** (for general UIs):
Either integrate a lightweight ONNX model or shell out to an external service.
This is a v2 feature — document the interface, don't implement yet.

### Protocol Actions

```rust
// Find UI element by visual template
VisionFindElement {
    template_path: String,       // path to template image on disk
    screenshot: Option<String>,   // specific screenshot or fresh capture
    min_confidence: Option<f64>,  // 0.0-1.0, default: 0.8
    max_results: Option<u32>,     // default: 5
}
// Returns: [{"x":100,"y":200,"width":50,"height":20,"confidence":0.95}, ...]

// Find element by text label (hybrid OCR + position)
VisionFindByText {
    text: String,
    screenshot: Option<String>,
}
// Returns: {"x":100,"y":200,"width":50,"height":20,"text":"Save","confidence":0.92}

// Detect UI state
VisionDetectState {
    screenshot: Option<String>,
    checks: Vec<StateCheck>,   // list of conditions to verify
}
// Returns: {"button_save_enabled": true, "dialog_open": false, "loading_spinner": false}
```

**Effort:** Template matching: ~200 lines (`image` crate, cross-correlation).
ML: significant (model file + inference runtime).

---

## 42. Screen Region Watching

**What's Missing:** Agents can take screenshots but can't say "watch this 200x200
area and tell me when it changes." Requires manual polling.

**Implementation:** Daemon captures a region at an interval, diffs against previous
frame, fires event when change detected.

```rust
RegionWatchCreate {
    name: String,
    monitor: Option<u32>,
    region: Region,
    interval_ms: u64,             // check every N ms
    change_threshold_pct: f64,    // fire if >N% of region changed (default: 1.0)
    notify_on_change: bool,       // fire event on each change
    notify_on_stable: bool,       // fire when region stops changing
    stable_duration_ms: u64,      // how long without change = stable
    auto_save: Option<String>,    // path to save changed frames
    max_changes: Option<u32>,     // auto-remove after N changes
}

RegionWatchUpdate { name: String, /* same params, partial */ },
RegionWatchRemove { name: String },
RegionWatchList,
```

### Subscription Events

```rust
DeskbridEvent::RegionChanged {
    name, changed_pct, bounding_boxes, screenshot_path, timestamp
}
DeskbridEvent::RegionStable {
    name, duration_ms, screenshot_path, timestamp
}
```

**Effort:** ~250 lines. Async loop per watch, pixel diff against previous frame.

---

## 43. Text Change Events (Watched Regions)

**What's Missing:** Agents can watch a region visually (section 42) but can't say
"tell me when the text in this 300x100 area changes." OCR + region watching =
text-aware region watching.

**Implementation:** Combine region watching (section 42) with OCR (section 12):

```rust
TextWatchCreate {
    name: String,
    monitor: Option<u32>,
    region: Region,
    interval_ms: u64,
    language: Option<String>,
    notify_on_change: bool,       // fire when extracted text differs
    notify_on_match: Option<String>, // fire when text contains substring
    notify_on_mismatch: Option<String>, // fire when text stops containing substring
    max_entries: Option<u32>,     // keep last N texts for history
}
```

### Subscription Events

```rust
DeskbridEvent::TextChanged {
    name, old_text, new_text, region, timestamp
}
DeskbridEvent::TextMatched {
    name, text, pattern, region, timestamp
}
```

**Effort:** ~200 lines. Composes region watch + OCR — both already documented.

---

## 44. Agent-to-Agent Messaging

**What's Missing:** Two agents connected to the same daemon can't communicate.
Each only talks to the daemon. If agent-alpha discovers something agent-beta needs,
it must send it through an external channel.

**Implementation:** Add an internal message bus. Agents send messages to other
agents by session name:

```rust
AgentMessage {
    to_session: String,           // recipient session name
    subject: String,              // message topic
    body: serde_json::Value,      // any JSON payload
    ttl_ms: Option<u64>,          // message expires after this
    reply_to: Option<String>,     // for request-response pattern
}

AgentBroadcast {
    subject: String,              // topic
    body: serde_json::Value,
    exclude_self: Option<bool>,   // default: true
}
```

### Subscription Events

```rust
DeskbridEvent::AgentMessage {
    from_session, subject, body, timestamp
}
```

**Effort:** ~150 lines. Message queue in daemon state, routed by session name.

---

## 45. Shared Blackboard (KV Store)

**What's Missing:** Agents need a coordination data store. No shared state means
each agent re-discovers things the other already knows. A blackboard lets agents
publish and consume facts.

**Implementation:** A key-value store scoped to the daemon. Keys are strings,
values are any JSON-serializable data.

```rust
BlackboardSet {
    key: String,
    value: serde_json::Value,
    ttl_secs: Option<u64>,       // auto-expire (session duration, or 0 for persistent)
    namespace: Option<String>,   // "default", "shared", or session-specific
    exclusive: Option<bool>,     // fail if key already exists (for locks)
}

BlackboardGet {
    key: String,
    namespace: Option<String>,
}

BlackboardDelete { key: String, namespace: Option<String> },

BlackboardSearch {
    prefix: Option<String>,      // find all keys starting with this
    namespace: Option<String>,
}

BlackboardList {
    namespace: Option<String>,
}
```

### Subscription Events

```rust
DeskbridEvent::BlackboardChanged {
    key, namespace, old_value, new_value, timestamp
}
DeskbridEvent::BlackboardDeleted {
    key, namespace, timestamp
}
```

**Effort:** ~200 lines. In-memory `HashMap<String, (Value, Instant)>` with TTL
sweeper.

---

## 46. Lock / Mutex Primitives

**What's Missing:** Two agents can race for the same resource (window focus, input
injection, keyboard). No coordination means conflicting operations.

**Implementation:** Distributed lock over the blackboard (section 45):

```rust
LockAcquire {
    resource: String,             // "input.keyboard", "window.focus"
    holder: String,               // session name (auto-filled if session exists)
    ttl_ms: u64,                  // max hold time (default: 5000)
    wait_ms: u64,                 // max wait to acquire (0 = fail-fast)
    force: bool,                  // steal lock from current holder
}

LockRelease {
    resource: String,
    token: String,                // lock token from acquire response
}

LockList,
// Returns: [{"resource":"input.keyboard","holder":"agent-alpha","acquired_at":...,"ttl_ms":5000}, ...]
```

**Lock semantics:** Backed by blackboard keys (`_lock:input.keyboard`). Automatic
release on session disconnect. Stale lock detection (TTL expired).

### Subscription Events

```rust
DeskbridEvent::LockAcquired { resource, holder, timestamp }
DeskbridEvent::LockReleased { resource, holder, timestamp }
DeskbridEvent::LockStolen { resource, old_holder, new_holder, timestamp }
DeskbridEvent::LockTimeout { resource, holder, timestamp }
```

**Effort:** ~200 lines. Blackboard-backed distributed lock.

---

## 47. Agent Registry

**What's Missing:** No way to discover what agents are connected, what they can do,
or what they're doing. The `clients` CLI shows connection count but nothing more.

**Implementation:** Each session registers on connect with metadata:

```rust
// Auto-registered on session connect
AgentRegister {
    name: String,                 // session name
    agent_type: Option<String>,   // "codex", "praxis", "hermes"
    capabilities: Option<Vec<String>>, // what this agent does
    metadata: Option<HashMap<String, String>>,
    heartbeat_interval_ms: Option<u64>, // for liveness tracking
}

AgentList,
// Returns: [{
//   "name": "agent-alpha", "connected_at": ..., "uid": 1000,
//   "capabilities": ["code", "terminal", "browser"],
//   "subscriptions": 5, "terminals": 2, "last_action": "windows.list",
//   "last_seen_ms_ago": 200, "locked_resources": []
// }, ...]

AgentGet { name: String },

AgentHeartbeat { name: String },  // automatic if heartbeat_interval set
```

### Subscription Events

```rust
DeskbridEvent::AgentConnected { name, agent_type, timestamp }
DeskbridEvent::AgentDisconnected { name, reason, uptime_secs, timestamp }
DeskbridEvent::AgentHeartbeatTimeout { name, timestamp }
```

**Effort:** ~150 lines. Session metadata + liveness tracking.

---

## 48. REPL Mode

**What's Missing:** Testing actions requires writing code or crafting JSON. No
interactive way to explore the daemon's capabilities.

**Implementation:** Built into the CLI:

```bash
deskbrid repl
# deskbrid> windows.list
# → [{id: "0x1", title: "Terminal", app_id: "gnome-terminal"}, ...]
# deskbrid> system.info
# → {desktop: "GNOME", version: "47", ...}
# deskbrid> !bash command       # escape to shell
# deskbrid> !!repeat 5          # repeat last action 5 times
# deskbrid> ?                   # show available commands
# deskbrid> help windows.list   # show action schema
```

**Features:** History (readline), tab completion, inline JSON, session management,
colorized output, pipe to `jq` filter, macro recording mode.

**Relevant crates:** `rustyline` (readline), `colored` (output).

**Effort:** ~300 lines. Readline loop + protocol client. Exists as `deskbrid repl`.

### Protocol (for remote REPL)

```rust
// Connect to daemon, send actions, receive responses in real-time
// Same protocol, just a human-friendly CLI wrapper.
```

---

## 49. Action Simulator (Replay Capture)

**What's Missing:** No way to test actions or replay sessions without affecting the
live desktop. A simulator captures real sessions and lets you replay them against
a mock backend.

**Implementation:** Two parts:

**A) Session capture:** Every action + response is logged with timestamps:
```bash
deskbrid daemon --capture ~/captures/capture_2026-05-20.jsonl
```

**B) Replay against mock backend:**
```bash
deskbrid simulate ~/captures/capture_2026-05-20.jsonl
# Replays all actions against a DesktopBackend mock
# Reports: "24/24 actions would have succeeded, 3 would have been denied"
```

**Mock backend:** Implement `DesktopBackend` trait with:
- No-op methods (log + return success)
- Configurable failure injection (make specific methods return errors)
- Validation mode (check permissions without executing)
- State tracking (track "what would the window list look like")

```rust
struct MockBackend {
    log: Vec<(String, String, String)>,  // (action, result, duration)
    fail_actions: HashSet<String>,
}
```

**Effort:** ~300 lines. Capture format + mock backend + replay loop.

---

## 50. Protocol Fuzzer

**What's Missing:** No automated way to test the daemon's robustness against
malformed or unexpected input.

**Implementation:** A built-in tool that sends random/generated actions and monitors
for crashes, hangs, or unexpected errors:

```bash
deskbrid fuzz --duration 60 --seed 42
# Tests: invalid JSON, missing fields, wrong types, extreme values,
#         rapid fire, concurrent connections, large payloads
```

**Fuzz strategies:**
1. **Mutation:** Take real actions, corrupt random bytes
2. **Generation:** Random valid JSON with random action types
3. **Boundary:** Empty strings, max values, negative numbers, NaN
4. **Concurrency:** N connections sending simultaneously
5. **Replay:** Real-world captured malformed inputs

**Integration:** Links against deskbrid as a library, connects to a test daemon
or mock backend.

**Effort:** ~400 lines. `arbitrary` crate for generation + tokio concurrency test.

---

## 51. OpenAPI / JSON Schema Export

**What's Missing:** Every action has strict parameter requirements, but there's no
machine-readable schema for code generation. Every client SDK must be hand-written.

**Implementation:** Auto-generate an OpenAPI 3.1 spec from the Action enum +
parameter types at compile time or via a CLI command:

```bash
deskbrid schema openapi
# → OpenAPI 3.1 JSON spec covering all 90+ actions

deskbrid schema json-schema
# → JSON Schema for the envelope format
```

**Crate:** `schemars` — generate JSON Schema from Rust types via derive macros.
Already used by many Rust projects for exactly this purpose.

```rust
#[derive(JsonSchema)]
pub struct Action {
    // ...
}
```

The schema includes:
- All action types with their parameter schemas
- Envelope format (type, id, seq, data, error)
- Response formats per action
- Authentication methods (Unix socket, TCP token)
- Rate limit headers

**Output includes:** OpenAPI 3.1 spec, JSON Schema, TypeScript types, Python
stubs — generated client code for any language via openapi-generator.

**Effort:** ~200 lines. `schemars` derive macros + CLI export command.

---

## 52. Semantic Screen Indexing

**What's Missing (THE SLEEPER HIT):** AT-SPI2 is great when it works, but it fails
on Electron apps, games, cross-platform toolkits, custom UI frameworks, web views,
and terminal UIs. OCR helps read text but agents still can't *find* anything.

**Semantic Screen Indexing** solves this: periodically screenshot + OCR + cache
element positions. Agents query "where is the Save button" and get coordinates back.
No AT-SPI needed. Works on any app, any toolkit. Pairs with wait-for conditions and
region watching to become a full perception layer.

### Implementation

**Phase 1 — Indexer (background):**

```rust
// Background task that periodically:
// 1. Takes a screenshot
// 2. Runs OCR with bounding boxes (positional data)
// 3. Caches: {text: "Save", x, y, w, h, confidence, screenshot_id, timestamp}
// 4. Groups: "Save" at (100,200) → likely the same button across frames

pub struct SemanticIndex {
    entries: Vec<IndexEntry>,
    snapshot_interval_ms: u64,     // how often to index (default: 2000)
    cleanup_old_ms: u64,           // discard entries older than (default: 60000)
    min_confidence: f64,           // minimum OCR confidence to index
    merge_overlap_pct: f64,        // merge entries that overlap by this much
}

pub struct IndexEntry {
    text: String,                  // OCR'd text
    x, y, w, h: u32,             // bounding box
    confidence: f64,
    screenshot_id: String,
    timestamp: u64,
    stable_count: u32,            // how many consecutive snapshots this element appeared
    last_seen: u64,               // when it was last present
}
```

**Phase 2 — Query API:**

```rust
// Agent asks: find the Save button
SemanticFind {
    query: String,                     // "Save", "Save As", "Open File"
    approximate: Option<bool>,         // allow fuzzy text match (default: true)
    min_confidence: Option<f64>,       // default: 0.6
    region: Option<Region>,            // restrict to a screen area
    closest_to: Option<(f64, f64)>,    // find nearest match to cursor
    stale_ok_ms: Option<u64>,         // accept entries up to N ms old
    min_stability: Option<u32>,       // require N consecutive sightings
    take_screenshot: Option<bool>,    // refresh index with fresh screenshot
}

// Returns:
{
    "query": "Save",
    "matches": [
        {
            "text": "Save",
            "x": 100, "y": 200, "width": 60, "height": 24,
            "confidence": 0.92,
            "stability": 5,          // seen in last 5 snapshots
            "last_seen_ms_ago": 200,
            "app_id": "firefox"       // if available from window context
        }
    ],
    "source_screenshot": "/tmp/deskbrid/semantic_12345.png",
    "index_age_ms": 500,
    "index_entries_total": 47
}

// Query with "click" hint — find + coordinate for clicking
SemanticClick {
    query: String,
    approximate: Option<bool>,
    min_confidence: Option<f64>,
    relative: Option<String>,         // "center" (default), "top_left", "bottom_right"
}
// Returns: {"text":"Save","x":130,"y":212,"width":60,"height":24,"confidence":0.92}
```

**Phase 3 — Subscribe to element lifecycle:**

```rust
// "Tell me when Save appears"
SemanticWatch {
    query: String,
    event: String,                   // "appears", "disappears", "moves", "changes_text"
    timeout_ms: Option<u64>,
    region: Option<Region>,
}

// Events
DeskbridEvent::SemanticElementAppeared { text, x, y, w, h, confidence, timestamp }
DeskbridEvent::SemanticElementDisappeared { text, x, y, w, h, timestamp }
DeskbridEvent::SemanticElementMoved { text, old_x, old_y, new_x, new_y, w, h, timestamp }
```

**Phase 4 — Pairing with wait-for conditions:**

Combines with section 26's `Wait` to create a full perception pipeline:

```json
// "Wait for the Save button to appear, then click it"
[
  {"type": "wait", "condition": "semantic_element", "params": {"text": "Save", "event": "appears"}, "timeout_ms": 10000},
  {"type": "semantic_click", "query": "Save"},
]
```

### Why This Wins

1. **Works everywhere:** Electron, Qt, GTK, games, terminals, web apps — if it renders
   text on screen, semantic indexing can find it.
2. **No DE dependency:** Same pipeline works on GNOME, KDE, Hyprland, X11, tiling WMs,
   even bare Xvfb headless sessions.
3. **No AT-SPI dependency:** Avoids the broken/incomplete accessibility trees that
   plague most Linux apps.
4. **Stability heuristic:** Elements seen across multiple snapshots are real UI elements,
   not transient rendering artifacts. Confidence grows with stability.
5. **Composable:** Pairs with wait-for (section 26), region watching (section 42),
   OCR (section 12), and element detection (section 41).

### Effort

- Phase 1 (Indexer): ~300 lines. Background OCR loop + in-memory cache.
- Phase 2 (Query): ~150 lines. Semantic search over the index.
- Phase 3 (Events): ~100 lines. Subscribe + diff index state.
- Phase 4 (Integration): ~50 lines. Wire into wait-for dispatch.

**Total:** ~600 lines. All building on existing OCR (tesseract + section 12) and
screenshot pipelines (already built). No new system dependencies.

The `image` crate (already optional dep) handles bounding box math. Tesseract
(section 12 dependency) handles text extraction. The only new code is the caching
+ query layer.

---

## 53. Wayland Protocols (Not Yet Wrapped)

**What's Missing:** Deskbrid uses DE-specific backends (Mutter DBus, hyprctl, KWin)
but doesn't use lower-level Wayland protocols directly. Several protocols enable
capabilities no single DE backend provides.

**wlr-layer-shell:** Control panels/overlays like waybar, eww, dunst. Lets agents
create on-screen HUDs, status overlays, or notification layers. Requires
`wayland-client` (already optional dep for COSMIC).

**xdg-activation-v1:** Standardized way to request window focus/token. Works across
any Wayland compositor that implements it. More portable than DE-specific focus.

**ext-image-capture-source-v1:** Screenshot individual app windows via PipeWire
instead of full screen. Compositor streams just the window's buffer. More efficient
and privacy-preserving than full-screen capture + crop.

**fractional-scale-v1:** Detect per-monitor fractional scaling (125%, 150%, 175%).
Currently Deskbrid gets scale from DE-specific APIs or xrandr. This protocol gives
accurate per-output scale factors on any compositor.

### Implementation

All of these use `wayland-client` (already optional dep) with the `wayland-protocols`
crate (already optional dep for COSMIC). Enable the relevant protocol XML and
bind to the compositor's globals.

**Effort:** ~200 lines per protocol. Wire into existing backend infrastructure.

---

## 54. Audio Control (PipeWire / PulseAudio D-Bus)

**What's Missing:** Deskbrid has `AudioListSinks` and `AudioSetSinkVolume` (sink
volume only). No per-app audio, no routing, no loopback, no mute state.

### Implementation

**Per-app volume via PipeWire:** PipeWire exposes a node graph via D-Bus
(`org.pipewire.PipeWire` or the `pw-cli`/`pw-dump` CLI). Each audio stream is a
node with controllable volume/mute:

```rust
// List all audio nodes (sinks, sources, streams)
let nodes = Command::new("pw-dump").output()?;
// Parse JSON, filter by type
// Control: pw-cli set-param <id> volume { value: 0.5 }

// PulseAudio-compatible D-Bus (via pipewire-pulse):
// org.pipewire.pulseaudio.* — or PulseAudio's native protocol
```

**Audio loopback:** Create a virtual sink that mirrors system audio. Agents can
"listen" to what the system is playing (for diagnostics, transcription, or
reaction-based automation):

```rust
// Create loopback module
pactl load-module module-loopback latency_msec=20
// Capture from the monitor source
parec --device=<monitor_source> --format=s16le --rate=44100
```

**Application audio isolation:** For screen recording with clean audio:

```rust
// Move a specific app's streams to a dedicated sink
let app_streams = find_streams_by_app("firefox");
for stream in app_streams {
    pactl move-sink-input <stream_id> <isolated_sink>
}
```

### Protocol Actions

```rust
AudioListNodes,                          // all audio nodes (sinks, sources, streams)
AudioNodeVolume { node_id: u32, volume: f64 },
AudioNodeMute { node_id: u32, mute: bool },
AudioNodeRoute { node_id: u32, target_sink_id: u32 },  // move stream to sink
AudioNodeInfo { node_id: u32 },

AudioCreateLoopback,
AudioDestroyLoopback { loopback_id: u32 },
AudioCaptureStart { source_id: u32, format: String },   // WAV, raw PCM
AudioCaptureStop,
// Returns transcribed text or audio file path
```

**Effort:** ~400 lines. PipeWire `pw-dump`/`pw-cli` wrappers + PulseAudio pactl.

---

## 55. GPU Power States

**What's Missing:** Desktop bridging should include GPU awareness. Agents can't
know GPU model, driver, utilization, or power state.

### Implementation

**NVIDIA:**
```rust
let info = Command::new("nvidia-smi")
    .args(["--query-gpu=index,name,temperature.gpu,utilization.gpu,power.draw,power.limit",
           "--format=csv,noheader"])
    .output()?;
// Parse CSV: "0, NVIDIA RTX 4090, 65, 45%, 250W, 450W"

// Set power limits (requires root/polkit):
// nvidia-smi -pl 300
// nvidia-smi -pm 1  (persistence mode)

// Switch graphics mode (laptops):
// prime-select query → "nvidia", "intel", "on-demand"
// supergfxctl --mode integrated/hybrid/nvidia
```

**AMD:**
```rust
// /sys/class/drm/card*/device/power_dpm_force_performance_level
// Values: "low", "high", "auto", "manual"
fs::write("/sys/class/drm/card1/device/power_dpm_force_performance_level", "low")?;

// /sys/class/drm/card*/device/gpu_busy_percent
let usage = fs::read_to_string("/sys/class/drm/card1/device/gpu_busy_percent")?;

// Power cap: /sys/class/drm/card*/device/power_cap
let power_cap = fs::read_to_string("/sys/class/drm/card1/device/power1_cap")?;
```

### Protocol Actions

```rust
GpuInfo,    // model, driver, memory, temperature, utilization, power
GpuSetPowerLimit { watts: u32 },
GpuSetPerformanceLevel { level: String },  // "low", "high", "auto"
GpuSetGraphicsMode { mode: String },       // "integrated", "hybrid", "nvidia"
```

**Effort:** ~200 lines. sysfs reads + nvidia-smi wrapper.

---

## 56. Battery Charge Threshold Management

**What's Missing:** Laptop battery longevity requires charge limits (80% for daily
use, 100% for travel). Deskbrid has `SystemBattery` (current percentage) but no
threshold control.

### Implementation

Vendor-specific sysfs paths:

```rust
// Lenovo (thinkpad_acpi):
// /sys/class/power_supply/BAT0/charge_control_start_threshold  → 0-100
// /sys/class/power_supply/BAT0/charge_control_end_threshold    → 0-100
fs::write("/sys/class/power_supply/BAT0/charge_control_end_threshold", "80")?;

// ASUS:
// /sys/class/power_supply/BAT0/charge_control_end_threshold
// (same interface, different driver)

// System76 (split between EC and sysfs):
// system76-power charge-thresholds --list
// system76-power charge-thresholds --profile <profile>
```

**Detection:** Probe known sysfs paths. If none exist, return unsupported.

### Protocol Actions

```rust
BatteryThresholdGet,
// Returns: {"start": 0, "end": 80, "supported": true, "vendor": "Lenovo"}

BatteryThresholdSet {
    start: Option<u32>,   // charge start threshold (0-100)
    end: u32,             // charge end threshold (0-100)
    profile: Option<String>, // "daily", "travel", "full" — vendor-specific presets
}
```

**Effort:** ~100 lines. sysfs probe + write.

---

## 57. Power Profiles Daemon

**What's Missing:** Desktop environments have power profiles (performance/balanced/
power-saver). Deskbrid can't switch them.

### Implementation

**GNOME's power-profiles-daemon (DBus):**

```rust
// org.freedesktop.UPower.PowerProfiles
// Profiles: "performance", "balanced", "power-saver"

// List available
let reply = conn.call_method(
    Some("org.freedesktop.UPower.PowerProfiles"),
    "/org/freedesktop/UPower/PowerProfiles",
    Some("org.freedesktop.DBus.Properties"),
    "Get",
    &("org.freedesktop.UPower.PowerProfiles", "Profiles"),
).await?;

// Switch
conn.call_method(
    Some("org.freedesktop.UPower.PowerProfiles"),
    "/org/freedesktop/UPower/PowerProfiles",
    Some("org.freedesktop.UPower.PowerProfiles"),
    "SetProfile",
    &("performance",),
).await?;
```

### Protocol Actions

```rust
PowerProfileList,
// Returns: ["performance", "balanced", "power-saver"]

PowerProfileGet,
// Returns: {"active": "balanced", "available": ["performance", "balanced", "power-saver"]}

PowerProfileSet { profile: String },
```

**Effort:** ~100 lines. zbus calls to power-profiles-daemon.

---

## 58. USB Device Power Control

**What's Missing:** Agents can't reset unresponsive USB devices, toggle port power,
or enumerate USB topology.

### Implementation

```rust
// List USB devices
// lsusb, /sys/bus/usb/devices/

// Power off a USB port:
// echo '0' > /sys/bus/usb/devices/<bus>/power/control (or "on" to enable)
// Or use `uhubctl` for per-port power control via USB hub controller

// Reset a device:
// usbreset <vendor>:<product> (from usbutils)
// Or: echo 0 > /sys/bus/usb/devices/<dev>/authorized && echo 1 > /sys/bus/usb/devices/<dev>/authorized
```

### Protocol Actions

```rust
UsbList,
// Returns: [{"bus": "001", "device": "003", "vendor": "046d", "product": "c52b",
//            "name": "Logitech USB Receiver", "power": "on", "power_watts": 2.5}, ...]

UsbPowerToggle { address: String, state: String },  // "on", "off"
UsbReset { address: String },
UsbPowerPort { hub: String, port: u32, state: String },  // per-port via uhubctl
```

**Effort:** ~200 lines. sysfs + `uhubctl` wrapper.

---

## 59. Input Device Configuration

**What's Missing:** Deskbrid injects input but can't configure input devices.
Mouse DPI, touchpad gestures, keyboard repeat rate, etc.

### Implementation

```rust
// Mouse DPI via libratbag/ratbagd (DBus):
// org.freedesktop.ratbag1 — list devices, profiles, resolutions
// ratbagctl <device> resolution set <dpi>

// Touchpad gestures via libinput (CLI or sysfs):
// libinput list-devices
// gsettings set org.gnome.desktop.peripherals.touchpad natural-scroll true
// gsettings set org.gnome.desktop.peripherals.touchpad tap-to-click true
// gsettings set org.gnome.desktop.peripherals.touchpad speed 0.0  // -1.0 to 1.0

// Keyboard repeat rate:
// gsettings set org.gnome.desktop.peripherals.keyboard delay 500
// gsettings set org.gnome.desktop.peripherals.keyboard repeat-interval 30
// xset r rate 200 30  (X11)

// Hyprland equivalents via hyprctl:
// hyprctl keyword input:touchpad:natural_scroll true
// hyprctl keyword input:repeat_rate 50
// hyprctl keyword input:repeat_delay 300
```

### Protocol Actions

```rust
InputDeviceList,
// Returns: [{"name":"Logitech G Pro","type":"mouse","vendor":"046d","product":"c084"}, ...]

InputDeviceGetConfig { device_id: String },

InputDeviceSetDpi {
    device_id: String,
    dpi: u32,
    profile: Option<u32>,  // which DPI profile slot (for multi-DPI mice)
},

InputTouchpadSetConfig {
    natural_scroll: Option<bool>,
    tap_to_click: Option<bool>,
    speed: Option<f64>,        // -1.0 to 1.0
    disable_while_typing: Option<bool>,
},

InputKeyboardSetRepeat {
    delay_ms: Option<u32>,      // delay before repeat starts
    rate: Option<u32>,          // repeats per second
},
```

**Effort:** ~300 lines. ratbagd DBus + gsettings/sysfs.

---

## 60. Monitor DDC/CI

**What's Missing:** Monitor brightness/contrast currently requires DE support
(xrandr, wlr-randr, Mutter). DDC/CI adjusts monitor settings directly over the
display cable — works on any monitor, any OS.

### Implementation

```rust
// ddccontrol (CLI):
// ddccontrol -r 0x10 -w 50 dev:/dev/i2c-N  // brightness
// ddccontrol -r 0x12 -w 50 dev:/dev/i2c-N  // contrast

// ddcutil (better maintained):
// ddcutil detect                                  → list monitors
// ddcutil getvcp 10                                → read brightness
// ddcutil setvcp 10 80                             → set brightness
// ddcutil capabilities dev:/dev/i2c-4              → VCP features
```

**Key VCP features codes:**
- `0x10`: Brightness
- `0x12`: Contrast
- `0x60`: Input source (HDMI1, DP, USB-C)
- `0xDC`: Power mode (on/off/sleep)
- `0x62`: Audio volume (for monitors with speakers)

### Protocol Actions

```rust
MonitorDDCList,
// Returns: [{"i2c_bus": "/dev/i2c-4", "model": "DELL U2723QE", "edid": "..."}, ...]

MonitorDDCGetVcp { bus: String, vcp_code: u16 },
MonitorDDCSetVcp { bus: String, vcp_code: u16, value: u16 },

MonitorDDCBrightness { bus: String, percent: f64 },
MonitorDDCContrast { bus: String, percent: f64 },
MonitorDDCInput { bus: String, input: String },  // "hdmi1", "dp", "usb-c"
MonitorDDCPower { bus: String, state: String },   // "on", "off", "sleep"
```

**Effort:** ~200 lines. `ddcutil` CLI wrapper.

---

## 61. Notification History & Action Buttons

**What's Missing:** Deskbrid can `NotificationSend` and `NotificationClose` but
can't read existing notifications or respond to action buttons.

### Implementation

**GNOME Notifications DBus:**

```rust
// org.freedesktop.Notifications:
// - GetServerInformation(…)
// - GetCapabilities(…) → "actions", "body", "action-icons"
// - CloseNotification(id)

// Reading notifications: GNOME stores them in
// ~/.local/share/gnome-shell/notifications/ (JSON)

// Receiving notification signals on org.freedesktop.Notifications:
// - NotificationClosed(id, reason)
// - ActionInvoked(id, action_key)
```

**Intercepting:**

```rust
// Register as a notification listener on the session bus
// Forward all notifications to subscribed clients
// Store last N notifications in a ring buffer (like clipboard history)
```

### Protocol Actions

```rust
NotificationHistory {
    limit: Option<u32>,
    app_name: Option<String>,       // filter by app
    since: Option<u64>,
}
// Returns: [{"id": 42, "app_name": "firefox", "title": "Download complete", "body": "...",
//            "urgency": "normal", "timestamp": ..., "actions": ["Open", "Show in Folder"]}, ...]

NotificationAction {
    notification_id: u32,
    action_key: String,             // "default", "Open", "Dismiss"
}

NotificationClearHistory,

NotificationWatch,                  // subscribe to new notifications
```

### Subscription Events

```rust
DeskbridEvent::NotificationReceived {
    id, app_name, title, body, urgency, actions, timestamp
}
DeskbridEvent::NotificationActed {
    id, action_key, timestamp
}
```

**Effort:** ~200 lines. DBus signal listener + history ring buffer.

---

## 62. NetworkManager D-Bus

**What's Missing:** Deskbrid uses nmcli for WiFi. NetworkManager's D-Bus API gives
richer control: connection profiles, signal strength, hotspots, ethernet, VPN.

### Implementation

```rust
// Create a WiFi hotspot (agent infrastructure):
// org.freedesktop.NetworkManager.AddAndActivateConnection
// with "802-11-wireless" settings: mode="hotspot", ssid, psk

// Read signal strength of connected network:
// org.freedesktop.NetworkManager.AccessPoint — Properties: Strength (uint8 0-100)

// List saved connection profiles:
// org.freedesktop.NetworkManager.Settings.ListConnections

// Enable/disable WWAN (mobile broadband), WiFi, ethernet:
// org.freedesktop.NetworkManager — WirelessEnabled, WwanEnabled
```

### Protocol Actions (additions to existing `Network*`)

```rust
NetworkConnectionList,
// Returns: [{"id": "MyWiFi", "type": "802-11-wireless", "ssid": "MyWiFi",
//            "signal": 85, "security": "WPA2", "ip": "192.168.1.42", 
//            "gateway": "192.168.1.1", "dns": ["8.8.8.8"]}, ...]

NetworkConnectionProfiles,
// Returns: [{"uuid": "...", "id": "Work VPN", "type": "vpn", "autoconnect": true}, ...]

NetworkCreateHotspot { ssid: String, password: Option<String> },
NetworkStopHotspot,

NetworkWifiEnable { enabled: bool },
NetworkWwanEnable { enabled: bool },

NetworkDnsSet { dns: Vec<String> },
NetworkDnsReset,

NetworkVpnConnect { profile_name: String },
NetworkVpnDisconnect,
```

**Effort:** ~250 lines. zbus calls to NetworkManager.

---

## 63. Tailscale / WireGuard Status

**What's Missing:** Agents running on a machine with Tailscale or WireGuard have no
way to know the VPN status, peer connectivity, or IPs.

### Implementation

**Tailscale:**

```rust
// tailscale status --json
let status = Command::new("tailscale")
    .args(["status", "--json"])
    .output()?;
// Parse: Self → Online, DERP region, IPs
// Parse: Peers → Online status, IPs, last seen, relay vs direct

// tailscale ping <host>  → check direct connectivity

// tailscale up --accept-dns=false  → configure
```

**WireGuard:**

```rust
// wg show → interface, public key, endpoint, allowed IPs, transfer
// wg showconf <iface> → full config
// /sys/class/net/wg0/  → link status
```

### Protocol Actions

```rust
NetworkVpnStatus,
// Returns: {
//   "tailscale": {
//     "online": true, "ip": "100.x.x.x", "derp": "us-east",
//     "peers": [{"name": "nas", "ip": "100.x.x.y", "online": true, "direct": true, "last_seen": "2s ago"}]
//   },
//   "wireguard": {
//     "interfaces": [{"name": "wg0", "public_key": "...", "peers": 3, "transfer_rx": "1.2GB", "transfer_tx": "300MB"}]
//   }
// }

NetworkVpnPeerInfo { peer: String },
NetworkVpnConnect, NetworkVpnDisconnect,  // for WireGuard
```

**Effort:** ~150 lines. Parse `tailscale status --json` + `wg show`.

---

## 64. mDNS Advertisement (Deskbrid Instance Discovery)

**What's Missing:** Multiple Deskbrid instances on the same LAN (TCP mode, section
30) have no way to discover each other.

### Implementation

Use Avahi (`org.freedesktop.Avahi` over DBus) to advertise and discover Deskbrid
instances:

```rust
// Advertise this instance:
// Service type: "_deskbrid._tcp"
// TXT records: version, hostname, session count, capabilities flags

// Discover other instances:
// Browse _deskbrid._tcp, resolve found services

// Alternative: Simple mdns-sd via CLI
// avahi-publish -s "Jeremy's Desktop" _deskbrid._tcp 7890
// avahi-browse _deskbrid._tcp
```

### Protocol Actions

```rust
DiscoveryAdvertise {
    name: String,                   // friendly name
    port: u32,                      // TCP port (from section 30)
    tls: bool,                      // whether TLS is enabled
}

DiscoveryStop,

DiscoveryList,
// Returns: [{"name": "Jeremy's Desktop", "hostname": "jerebook.local",
//            "address": "192.168.1.42", "port": 7890, "tls": true,
//            "version": "0.6.0", "last_seen_ms_ago": 1000}, ...]
```

**Effort:** ~150 lines. Avahi DBus calls.

---

## 65. Distrobox / Toolbox Integration

**What's Missing:** Desktop agents on Atomic/Fedora Silverblue/Universal Blue can't
interact with distrobox containers where dev tools actually live.

### Implementation

```rust
// List containers
// distrobox list
// → "ubuntu-dev" | "fedora-toolbox-42" | ...

// Enter container, run command:
// distrobox enter ubuntu-dev -- <command>
// toolbox run --container fedora-toolbox-42 <command>

// Copy files in/out:
// distrobox-host-exec cp /tmp/result.txt /var/home/user/result.txt
// toolbox cp <container>:/source /destination

// Init a new container:
// distrobox create --name ubuntu-dev --image ubuntu:24.04
```

### Protocol Actions

```rust
ContainerList,
// Returns: [{"name":"ubuntu-dev","image":"ubuntu:24.04","status":"running","created":"2d ago"}, ...]

ContainerExec {
    name: String,
    command: Vec<String>,
    cwd: Option<String>,
    env: Option<HashMap<String, String>>,
    timeout_ms: Option<u64>,
}

ContainerCreate {
    name: String,
    image: String,
    init_hooks: Option<Vec<String>>,   // commands to run on first init
}

ContainerRemove { name: String },
ContainerCopy { name: String, source: String, destination: String },
```

**Effort:** ~200 lines. CLI wrappers around distrobox/toolbox.

---

## 66. Docker / Podman Container Awareness

**What's Missing:** If a target app runs inside a container, Deskbrid should know
and adapt. Agents also need to manage containers for dev environments.

### Implementation

```rust
// Auto-detect podman/docker availability
// Which runtime: check $CONTAINER_HOST, $DOCKER_HOST, podman --version, docker --version

// List containers:
// podman ps --all --format json
// docker ps --all --format json

// Exec into a container:
// podman exec -it <name> <command>
// docker exec -it <name> <command>

// Check if a PID is inside a container:
// /proc/<PID>/cgroup → check for "docker" or "podman" in path
// Or: cat /proc/<PID>/mountinfo | grep overlay

// Detect if WE are inside a container:
// /.dockerenv file, /proc/1/cgroup
```

### Protocol Actions

```rust
ContainerRuntimeInfo,
// Returns: {"available": ["podman", "docker"], "active": "podman",
//           "version": "5.0.0", "running_containers": 2}

ContainerList {
    all: Option<bool>,       // include stopped
    runtime: Option<String>, // "podman" or "docker"
}

ContainerExec {
    name: String,
    command: Vec<String>,
    runtime: Option<String>,
}

ContainerLogs {
    name: String,
    tail: Option<u32>,
    runtime: Option<String>,
}

ContainerStart { name: String, runtime: Option<String> },
ContainerStop { name: String, runtime: Option<String> },
ContainerRestart { name: String, runtime: Option<String> },
```

**Effort:** ~200 lines. CLI wrappers for podman/docker.

---

## 68. mTLS for TCP Mode

**What's Missing:** Section 30 (TCP mode) describes TLS + bearer token auth. No
mutual TLS — the server verifies the client's certificate. Stronger auth model.

**Implementation:** On first daemon start, generate a CA cert + server cert. Each
client gets a signed client cert (via `deskbrid tcp authorize <name>`). Connections
require the client cert:

```bash
deskbrid daemon --tcp 0.0.0.0:7890 --tls --mtls
deskbrid tcp authorize --name agent-alpha --out agent-alpha.pem
# agent connects with client.pem
```

**Effort:** ~150 lines. Cert generation + `rustls` mTLS config.

---

## 69. Landlock + Seccomp for Spawned Processes

**What's Missing:** Spawned subprocesses (`ProcessStart`) inherit Deskbrid's full
filesystem access. A compromised agent can read/write anything.

**Implementation:**

**Landlock** (Linux 5.13+): Restrict filesystem access for child processes:

```rust
// Before spawning: apply Landlock rules via the landlock crate
// Allow: ~/project/* (read/write), /tmp/* (read/write)
// Allow: /usr/bin/*, /usr/lib/* (read/execute)
// Deny everything else
landlock_restrict(process_pid, &[
    Access("/home/user/project", ReadWrite),
    Access("/tmp", ReadWrite),
    Access("/usr/bin", ReadExecute),
])?;
```

**Seccomp** (for `ProcessStart` with `sandbox: true`): Block dangerous syscalls:

```rust
// Allow: read, write, openat, close, mmap, exit_group, etc.
// Block: ptrace, bpf, mount, reboot, kexec_load, etc.
// Default action: kill process on violation
seccomp_apply(SeccompFilter::new(
    vec![
        Allow(syscall::read),
        Allow(syscall::write),
        // ...
    ],
    ScmpAction::Kill,
)?;
```

**Crates:** `landlock` (Rust bindings), `seccompiler` or `libseccomp-sys`.

**Protocol:** Add `sandbox: Option<SandboxConfig>` to `ProcessStart`:

```rust
ProcessStart {
    command, workdir, env,
    sandbox: Option<SandboxConfig>,
}
// SandboxConfig { landlock_paths: [...], seccomp_filter: "default" | "strict" | "none" }
```

**Effort:** ~300 lines. Landlock + seccomp integration with spawned process setup.

---

## 70. Immutable Permissions

**What's Missing:** permissions.toml is reloaded on daemon start but can be
modified at runtime by anyone with filesystem write access. A compromised agent
that escalates to the user's UID can rewrite their own permissions.

**Implementation:** Add a `readonly: true` flag to permissions.toml. When set,
the daemon refuses to reload or modify the file during operation:

```toml
readonly = true
immutable_signature = "sha256-<hash>"  # optional: fail if file doesn't match hash

[default]
deny = ["*"]

[uid:1000]
allow = ["windows.*", "input.*", "clipboard.*"]
```

**Optional hardening:** Ship a `.permissions.toml.sig` alongside the config.
Daemon verifies the signature on startup. Signature generated by a separate
admin tool the user runs after editing.

```bash
deskbrid permissions sign         # signs current permissions.toml
deskbrid permissions verify       # check signature, reject if invalid
deskbrid permissions reload       # explicit reload (requires admin token)
```

**Effort:** ~150 lines. Readonly flag + optional signature verification.

---

## 71. Action Timeouts with Kill Guarantees

**Status:** ✅ Done. Request envelopes can include `timeout_ms`, and the daemon
also applies `DESKBRID_ACTION_TIMEOUT_MS` as a default action timeout (60s by
default, `0` disables). `wait.for` uses its requested wait timeout plus a small
grace period when no request-level timeout override is provided.

**What's Missing:** Some actions can hang indefinitely (screenshot on a locked
session, process.wait on a stuck process, terminal.read on a frozen PTY). No
timeout mechanism.

**Implementation:** Every action execution is wrapped with a timeout:

```rust
async fn execute_with_timeout(action: Action, backend, timeout_ms: u64) -> Result<Value> {
    let result = tokio::time::timeout(
        Duration::from_millis(timeout_ms),
        execute_action(action, backend),
    ).await;

    match result {
        Ok(Ok(data)) => Ok(data),
        Ok(Err(e)) => Err(e),
        Err(_elapsed) => {
            // Force-kill: send SIGKILL to any spawned process
            // Cancel any in-flight DBus call
            // Release any locks held by this action
            force_cleanup(&action)?;
            Err(anyhow!("action timed out after {}ms", timeout_ms))
        }
    }
}
```

**Default timeouts per action category:**

| Category | Timeout | Rationale |
|---|---|---|
| `windows.*` | 5s | Fast operations |
| `input.*` | 10s | Typing large text |
| `screenshot` | 15s | ScreenCast setup |
| `files.*` | 30s | Large file reads/writes |
| `process.*` | 60s | Process wait can be long |
| `terminal.*` | 60s | Terminal commands can be slow |
| `*` | 30s | Default |

**Configurable** on a per-action basis:

```json
{"type": "process.start", "command": ["sleep", "300"], "timeout_ms": 5000}
```

**Kill guarantee:** Timeout handler sends SIGKILL (not SIGTERM) to ensure
process death. Cleans up PTYs, temporary files, locks.

**Effort:** ~200 lines. Timeout wrapper in dispatch + force cleanup.

---

## 72. Audit Trail Signing

**What's Missing:** Audit log (section 34) entries are plain JSONL. Anyone with
write access to the file can forge entries. No way to prove an audit trail hasn't
been tampered with.

**Implementation:** Append a cryptographic signature to each audit entry:

```rust
pub struct SignedAuditEntry {
    entry: AuditEntry,           // the raw log data
    signature: String,           // base64-encoded Ed25519 signature
    signing_key_fingerprint: String,
}

// Signing:
// 1. Daemon generates Ed25519 keypair on first run
// 2. Public key fingerprint stored in audit header
// 3. Each entry: SHA-256(prev_signature || entry_json) → signed with private key
// 4. Chain: each entry's signature depends on the previous entry

// Verification:
// deskbrid audit verify [--file audit.jsonl]
// Reports: "All 1,542 entries valid. Signature chain intact."
```

**Crate:** `ed25519-dalek` or `signature` + `ed25519`.

**Admin tools:**

```bash
deskbrid audit verify              # verify signature chain
deskbrid audit export --format jsonl  # export without signatures for analysis
deskbrid audit key-info            # show public key fingerprint
```

**Effort:** ~200 lines. Ed25519 signing in audit log writer + verify command.

---

## 73. Protocol Test Suite

**What's Missing:** 90+ actions, no integration tests that verify each one works
against a real backend (mock or live).

**Implementation:** A test harness that connects to the daemon (or mock) and
exercises every action:

```bash
deskbrid test                     # run all integration tests
deskbrid test --action windows.*  # run specific category
deskbrid test --list              # list all available tests
```

```rust
// Each action gets a test case:
test_cases! {
    ("windows.list", || async {
        let resp = send_action(Action::WindowsList).await;
        assert_eq!(resp.status, "ok");
        assert!(resp.data.as_array().unwrap().len() > 0);
    }),
    ("input.keyboard.type", || async {
        let resp = send_action(Action::InputKeyboardType { text: "hello".into() }).await;
        assert_eq!(resp.status, "ok");
    }),
    // Negative tests:
    ("windows.close.invalid", || async {
        let resp = send_action(Action::WindowsClose("invalid".into())).await;
        assert_eq!(resp.status, "error");
        assert_eq!(resp.error.code, "INTERNAL_ERROR");
    }),
}
```

**Test modes:**
- **dry-run:** Against mock backend, no desktop needed
- **live:** Against real daemon (user confirms)
- **fuzz:** Random action permutations (section 50)
- **regression:** Replay captured audit logs and verify expected results

**Effort:** ~400 lines. Test harness + parameterized test cases.

---

## 74. Action Benchmarking

**What's Missing:** No data on how long actions take. Agents can't make informed
decisions about which approach is faster (e.g., a11y click vs semantic click vs
mouse click).

**Implementation:** Record latency in the dispatch layer:

```rust
pub struct ActionMetrics {
    action_type: String,
    count: u64,
    total_duration_ms: u64,
    min_ms: u64,
    max_ms: u64,
    p50_ms: u64,
    p95_ms: u64,
    p99_ms: u64,
    last_updated: u64,
}
```

**Tracking:** Sliding window of last 1000 executions per action type. Updated on
every dispatch call.

### Protocol Actions

```rust
BenchmarkGet {
    action_type: Option<String>,  // specific action or all
    since: Option<u64>,
}
// Returns: {"actions": [{"action": "windows.list", "count": 150, "avg_ms": 12, "p95_ms": 25, ...}, ...]}

BenchmarkReset,                     // clear metrics
BenchmarkCompare {                  // compare two approaches
    action_a: String,
    action_b: String,
    iterations: u32,
}
// Runs both actions N times, returns comparison stats
```

**Effort:** ~150 lines. Metrics collector in dispatch + sliding window stats.

---

## 75. Version Negotiation

**What's Missing:** The `connected` message includes `"protocol": "deskbrid-v2"`
but there's no negotiation. A v0.5 client connecting to v0.6 daemon might send
actions the daemon doesn't understand.

**Implementation:** Add protocol version handshake on connect:

```json
// Client sends on connect:
{"type": "hello", "protocol_versions": ["deskbrid-v2", "deskbrid-v1"], "features": ["screencast", "terminal"]}

// Daemon responds:
{"type": "welcome", "version": "0.6.0", "protocol": "deskbrid-v2",
 "server_features": ["windows.*", "input.*", "terminal.*", "screencast"],
 "min_client_version": "0.5.0",
 "protocol_version": 2}
```

**What the daemon does:**
1. Accepts client's requested protocol version
2. Falls back to lowest common denominator if higher version not fully supported
3. Returns `426 Upgrade Required` if client version is too old
4. Returns the intersection of requested and available features

**Effort:** ~100 lines. Version table + connection handshake logic.

---

## 76. Action Queue with Ordering

**What's Missing:** Agents fire actions as fast as they can. Race conditions:
windows.focus followed by input.type might execute in any order depending on
tokio scheduling.

**Implementation:** Per-session FIFO action queue. Client sends actions with an
optional sequence number. Daemon processes them in order:

```json
[
  {"type": "windows.focus", "window_id": "terminal", "id": "1", "seq": 1},
  {"type": "input.keyboard", "text": "ls\n", "id": "2", "seq": 2},
  {"type": "terminal.read", "terminal_id": "t-1", "id": "3", "seq": 3}
]
```

**Behavior:**
- Actions with same `seq` value execute in send order
- Actions without `seq` execute immediately (best-effort)
- Daemon can declare a "transaction": `seq_start` to `seq_end` as atomic batch
- If any action in a batch fails, remaining batch actions are skipped

**Parallel execution** (for independent actions):
```json
{"type": "windows.list", "id": "4", "parallel_group": "query"}
{"type": "clipboard.read", "id": "5", "parallel_group": "query"}
```
Actions in the same `parallel_group` run concurrently. Different groups block.

**Effort:** ~250 lines. Sequence queue with tokio channels per session.

---

## 77. Retry with Exponential Backoff

**What's Missing:** Transient failures (DBus timeout, compositor busy, screenshot
portal slow) cause action failures. Agent must manually retry.

**Implementation:** Add retry config to the envelope:

```json
{"type": "screenshot", "id": "1", "retry": {"max_attempts": 3, "backoff_ms": 500, "max_backoff_ms": 5000}}
```

**Backoff calculation:**
```rust
sleep = min(backoff_ms * 2^attempt, max_backoff_ms) + random_jitter(100ms)
// attempt 0: 500ms
// attempt 1: 1000ms
// attempt 2: 2000ms
```

**Which actions should retry:** Read-only actions (list, get, read) are safe to
retry. Write actions (delete, write, power) should only retry if the daemon can
prove they didn't execute (idempotency token).

**Idempotency key:** Client sends `idempotency_key` in the envelope. Daemon checks
if it already processed this key. If yes, returns cached response. Prevents
double-execution on retry.

**Effort:** ~150 lines. Retry loop + idempotency cache.

---

## 78. Health Webhook

**What's Missing:** External monitoring can't detect Deskbrid daemon state. If the
daemon crashes, no one knows until an agent tries to connect.

**Implementation:** On daemon start, register a webhook URL. POST JSON payloads
on lifecycle events:

```bash
deskbrid daemon --health-webhook https://monitor.example.com/deskbrid/alerts
```

**Payloads:**

```json
// On start (boot or restart):
{"event": "daemon.start", "version": "0.6.0", "hostname": "jerebook",
 "uptime": 0, "backend": "GNOME", "tcp": false, "pids": [1234]}

// On crash (from watchdog script):
{"event": "daemon.crash", "version": "0.6.0", "hostname": "jerebook",
 "last_uptime_secs": 86400, "exit_code": -6, "oom_score": 42}

// On healthy interval (configurable heartbeat):
{"event": "daemon.heartbeat", "version": "0.6.0", "uptime": 86400,
 "clients": 2, "actions_served": 15420, "memory_mb": 42, "cpu_pct": 1.2}

// On degradation:
{"event": "daemon.degraded", "reason": "backend_load_failed",
 "details": "GNOME Shell extension not reachable"}
```

**Watchdog mode:** A companion process (`deskbrid-watchdog`) that monitors the
daemon PID. If daemon exits unexpectedly, watchdog sends the crash webhook and
optionally restarts the daemon.

**Effort:** ~200 lines. HTTP client via `reqwest` (already a dep) + watchdog script.

---

## 79. Session Persistence (Survive Logout)

**What's Missing:** Deskbrid runs as a user service. On logout, systemd kills it.
Agents lose state, terminal sessions die, clipboard history is lost.

**Implementation:** Two-part approach:

**A) systemd user lingering:** Enable `loginctl enable-linger $USER`. The daemon
runs as a `--user` systemd service that survives logout.

```bash
# Install user service
systemctl --user enable deskbrid
loginctl enable-linger $(whoami)
```

**B) Service handoff:** When a new login session starts, the daemon is already
running from the linger session. New agents connect to the existing daemon.
Old terminal sessions remain alive.

**C) GPG/SSH agent forwarding:** For agents that need SSH keys or GPG after
logout, configure `systemd --user` with the right environment:

```ini
[Service]
Environment=SSH_AUTH_SOCK=%t/ssh-agent.socket
Environment=GPG_SSH_AUTH_SOCK=%t/gpg-agent.sock
```

### Protocol Actions

```rust
DaemonPersistenceGet,
// Returns: {"linger_enabled": true, "service": "user", "boot_start": false,
//           "session_id": "c1", "since_logout": false, "uptime_secs": 86400}

DaemonPersistenceEnable {
    linger: Option<bool>,         // enable lingering
    boot_start: Option<bool>,     // start on system boot, not just login
}
```

**Effort:** ~100 lines. systemd unit file + lingering detection.

---

## 80. Unified Search

**What's Missing:** Agents must query multiple endpoints to find something:
- `windows.list` to find a window
- `files.search` to find a file
- `clipboard.history` to find a copied snippet
- `app.list` (section 16) to find an app
- `audit.log` to find past actions

**Unified Search** indexes all of these into a single queryable store.

### Implementation

In-memory full-text search index, updated on every relevant action:

```rust
pub struct UnifiedIndex {
    windows: Vec<IndexedWindow>,
    files: Vec<IndexedFile>,
    clips: Vec<IndexedClip>,
    apps: Vec<IndexedApp>,
    audit: Vec<IndexedAudit>,
}

impl UnifiedIndex {
    fn search(&self, query: &str, categories: &[Category]) -> Vec<SearchResult> {
        // Tokenize query
        // Score each indexed item by: text relevance, recency, category weight
        // Return top 20 results
    }
}
```

### Protocol Actions

```rust
// Single search across everything
Search {
    query: String,
    categories: Option<Vec<String>>,  // "windows", "files", "clipboard", "apps", "audit", "all"
    limit: Option<u32>,               // per-category limit
    fuzzy: Option<bool>,              // allow fuzzy matching
}
// Returns:
{
  "query": "terminal",
  "results": {
    "windows": [{"id": "0xdeadbeef", "title": "Terminal", "app_id": "gnome-terminal", "score": 0.95}],
    "files": [{"path": "/home/user/projects/terminal/src/main.rs", "score": 0.7}],
    "clipboard": [{"text": "apt install terminal", "copied_at": 1747000000, "score": 0.6}],
    "apps": [{"name": "Terminal", "exec": "kgx", "score": 0.8}],
    "audit": [{"action": "windows.focus", "params": "terminal", "timestamp": 1747000000, "score": 0.5}]
  },
  "total_results": 4,
  "elapsed_ms": 15
}
```

**Effort:** ~300 lines. In-memory index + text scoring. Updates on dispatched actions.

---

## 82. Plugin System

**What's Missing:** Deskbrid is monolithic Rust. Adding a custom action means
forking and recompiling. No extension mechanism.

**Implementation:** WASM plugin runtime. Agents or users write plugins in any
language that compiles to WASM (Rust, Go, TinyGo, AssemblyScript). Plugins
register new action types and are sandboxed by the WASM runtime.

```rust
// Plugin interface (WASI-compatible)
pub trait DeskbridPlugin {
    fn name(&self) -> &str;
    fn actions(&self) -> Vec<PluginActionDef>;
    fn execute(&self, action: &str, params: Value) -> Result<Value>;
}

// Plugin discovery: ~/.local/share/deskbrid/plugins/*.wasm
// Each plugin exports its manifest: name, version, actions, permissions
```

**Crate:** `wasmtime` (WASM runtime, sandboxed by default). `wasmtime` provides
fuel metering (CPU limits), memory limits, and syscall filtering.

**Permission model:** Plugins declare required permissions in their manifest:
```toml
[plugin.window-manager]
version = "0.1.0"
requires = ["windows.*", "input.keyboard"]
sandbox = { fs = ["read:/tmp"], net = false }
```

**Effort:** ~800 lines. WASM runtime integration + plugin lifecycle management.

---

## 83. Event-Driven Triggers (Rules Engine)

**What's Missing:** Cron covers time-based scheduling. But agents can't say:
"when clipboard changes, run action X" or "when window Y closes, do Z."

**Implementation:** A lightweight rules engine that listens to subscription events
and fires actions when conditions match:

```rust
pub struct Rule {
    id: String,
    name: String,
    trigger: EventTrigger,    // what event to watch for
    condition: Option<Condition>,  // optional filter
    action: Action,            // what to execute
    enabled: bool,
    max_fires: Option<u32>,   // run N times then auto-disable
    cooldown_ms: Option<u64>, // don't re-fire within this window
}

pub enum EventTrigger {
    ClipboardChanged,
    WindowOpened { app_id: Option<String> },
    WindowClosed { app_id: Option<String> },
    WindowFocused { app_id: Option<String> },
    SessionLocked,
    SessionUnlocked,
    IdleStarted,
    IdleEnded,
    FileChanged { path: String },
    TimeRange { start_hour: u8, end_hour: u8, days: Vec<u8> },
    PresenceChanged { to: String },  // "active", "idle", "away"
}
```

### Protocol Actions

```rust
RuleCreate { name, trigger, condition, action, enabled, max_fires, cooldown_ms },
RuleList, RuleGet { rule_id }, RuleDelete { rule_id },
RulePause { rule_id }, RuleResume { rule_id },
```

**Effort:** ~300 lines. Event matcher + action dispatcher tied to subscription bus.

---

## 84. Persistence Layer (SQLite)

**What's Missing:** Clipboard history, audit log, blackboard, semantic index,
cron jobs, macros — all in-memory. Lost on daemon restart.

**Implementation:** Add SQLite via `rusqlite` crate. Contextual data persists
across restarts:

```rust
// Tables:
// clipboard_history (id, text, source, copied_at)
// audit_log (id, seq, uid, action, params, status, duration_ms, timestamp)
// blackboard (key, value_json, namespace, ttl, created_at, updated_at)
// semantic_index (id, text, x, y, w, h, confidence, screenshot_id, timestamp)
// cron_jobs (id, name, schedule, action_json, enabled, next_run, run_count)
// macros (name, actions_json, version, created_at)
// rules (id, name, trigger_json, action_json, enabled, max_fires)
// sessions (name, profile, created_at, last_seen)
```

**Migration strategy:** Embed schema version in SQLite user_version. Auto-migrate
on daemon start. Ship migrations inline in the binary.

**Performance:** WAL mode for concurrent reads. Periodic `PRAGMA optimize`.
Background vacuum.

**Effort:** ~400 lines. Schema definitions + migration + DAO layer.

---

## 85. MCP Server Mode

**What's Missing:** Deskbrid speaks its own JSON protocol. Any MCP-compatible
client (Claude Code, Codex CLI, Cursor) can't use it as a tool server without
a custom bridge.

**Implementation:** Run an MCP transport alongside the Unix socket. MCP tools
map to Deskbrid actions:

```json
// MCP tool definition for windows.list
{
  "name": "deskbrid_windows_list",
  "description": "List all open windows on the desktop",
  "inputSchema": { "type": "object", "properties": {} }
}

// MCP tool for window focus
{
  "name": "deskbrid_window_focus",
  "description": "Focus a window by ID or title",
  "inputSchema": {
    "type": "object",
    "properties": {
      "window_id": { "type": "string", "description": "Window ID or title" }
    }
  }
}
```

**Startup:**

```bash
deskbrid daemon --mcp              # stdio mode (for Claude Code, Codex)
deskbrid daemon --mcp-port 7891     # SSE mode (for remote MCP clients)
```

**Capability mapping:** 90+ actions → 90+ MCP tools. Categories map to tool
groups. Each tool gets auto-generated descriptions from action metadata.

**Effort:** ~300 lines. MCP transport adapter wrapping the dispatch layer.

---

## 86. Declarative Workflows / State Machines

**What's Missing:** Recording/replay (section 25) is flat. Cron (section 27)
is time-only. Event triggers (section 83) are single-step. No declarative
state machine where "step 1 → wait for X → step 2 → if Y → step 3" is a
single deployable unit.

**Implementation:** A YAML/JSON workflow definition with states, transitions,
and actions:

```yaml
name: "install-docker"
steps:
  - id: check
    action: process.run
    params: { command: ["which", "docker"] }
    on_success: { transition: "already-installed" }
    on_error: { transition: "install" }

  - id: install
    action: terminal.create
    params: { shell: "/bin/bash" }
    on_success: { transition: "wait-for-apt" }

  - id: wait-for-apt
    wait: { condition: "terminal.read", match: "Do you want to continue?" }
    on_match: { action: terminal.write, params: { input: "Y\n" }, transition: "done" }
    on_timeout: { transition: "failed" }

  - id: done
    action: notification.send
    params: { title: "Docker installed", body: "Ready to use." }

  - id: failed
    action: notification.send
    params: { title: "Install failed", body: "Check logs." }
```

**Storage:** Same as macros (JSON/YAML files in `~/.local/share/deskbrid/workflows/`).

**Runtime:** Lightweight state machine executor. Each step is an action, wait, or
condition branch. Supports loops, retries, parallel branches, and timeout.

**Effort:** ~500 lines. YAML/JSON parser + state machine executor.

---

## 87. Built-in Web Dashboard

**What's Missing:** CLI REPL (section 48) and the Unix socket are the only
interfaces. No visual way to monitor, test, or manage the daemon.

**Implementation:** A lightweight local web server serving a single-page React
(or plain HTML/JS) dashboard:

```bash
deskbrid daemon --web 127.0.0.1:8080
```

**Dashboard features:**
- **Actions explorer:** Browse all 90+ actions, see schemas, test them live
- **Live monitor:** Real-time event stream (windows opening, clipboard changes)
- **Screenshot viewer:** Browse screenshot timeline, diff images
- **Session manager:** See connected agents, their sessions, kill misbehaving ones
- **Permission editor:** Visual permissions.toml editor with validation
- **Audit log browser:** Search and filter past actions
- **Metrics dashboard:** Latency charts per action type, connection history
- **Cron jobs:** Visual scheduler, CRUD for scheduled actions

**Technology:** Serve static HTML/JS from the binary (embed via `include_dir!`).
Use the existing Unix socket to communicate with the daemon (or a dedicated
internal connection). Zero external dependencies.

**Effort:** ~600 lines for the server + ~300 lines of embedded HTML/JS.

---

## 88. Screenshot Timeline

**What's Missing:** Individual screenshots are isolated. No way to browse "what
was on screen 5 minutes ago" or scrub through a visual history.

**Implementation:** Extend the screenshot pipeline to auto-save and index
every screenshot:

```rust
pub struct ScreenshotTimeline {
    screenshots: Vec<ScreenshotEntry>,
    max_entries: u32,           // default: 1000
    retention_hours: u64,       // auto-clean screenshots older than (default: 24)
    thumbnails: bool,           // generate small previews for fast browsing
    auto_capture: bool,         // automatically screenshot on significant events
}

pub struct ScreenshotEntry {
    id: String,
    path: String,
    thumbnail_path: Option<String>,
    timestamp: u64,
    context: Option<String>,    // "user initiated", "auto: window focus change", "auto: clipboard"
    width: u32,
    height: u32,
    format: String,
}
```

**Culling:** Old screenshots cleaned on a background interval. User sets
retention policy. Auto-captured screenshots deprioritized over manual ones.

### Protocol Actions

```rust
ScreenshotTimelineGet {
    since: Option<u64>,
    until: Option<u64>,
    limit: Option<u32>,
}
// Returns: [{"id": "ss-001", "path": "...", "thumbnail": "...", "timestamp": ..., "context": "user"}, ...]

ScreenshotTimelineBrowse {
    interval_s: u64,             // show one screenshot per N seconds
    since: Option<u64>,
    until: Option<u64>,
}

ScreenshotTimelineDelete { older_than: u64 },
ScreenshotTimelineConfig { max_entries, retention_hours, auto_capture },
```

**Effort:** ~200 lines. Append-only log + cleanup task.

---

## 89. Graceful Degradation Profiles

**What's Missing:** When a feature isn't available, the action fails. No automatic
fallback chain. If AT-SPI isn't available, agents should automatically use OCR +
semantic index. If PipeWire isn't available, fall back to PulseAudio.

**Implementation:** Predefined degradation profiles that tell the daemon how to
handle unavailable features:

```rust
pub struct DegradationProfile {
    name: String,
    rules: Vec<DegradationRule>,
}

pub struct DegradationRule {
    primary: String,              // "a11y.click_element"
    fallbacks: Vec<String>,       // ["semantic_click", "ocr+coords", "mouse.move+click"]
    report: bool,                 // notify the agent which fallback was used
    latency_budget_ms: u64,       // if primary exceeds this, use fallback
}
```

**Built-in profiles:**

```toml
[profile."vision"]
a11y.click_element = { fallbacks = ["semantic_click", "ocr+coords", "mouse.click"], report = true }
a11y.get_text = { fallbacks = ["ocr.get_text"], report = true }

[profile."audio"]
audio.capture = { fallbacks = ["pactl.record"], report = true }

[profile."input"]
input.keyboard = { fallbacks = ["ydotool.keyboard", "xdotool.keyboard", "uinput.keyboard"], report = true }

[profile."screenshot"]
screenshot = { fallbacks = ["grim.screenshot", "import.screenshot", "portal.screenshot"], report = true }
```

**Auto-detection:** The daemon probes all backends on startup and builds the
degradation profile automatically. Reports available and degraded paths in
`SystemCapabilities`.

**Effort:** ~250 lines. Fallback chain executor + capability probe.

---

## 90. Self-Healing Fallback Chains

**What's Missing:** Graceful degradation (section 89) is static — it picks a
fallback and uses it. Self-healing means the daemon *learns* which approach works
best and adjusts dynamically.

**Implementation:** On each action execution, record which backend path succeeded:

```rust
pub struct FallbackMetrics {
    primary: String,
    attempts: u32,
    successes: u32,
    avg_latency_ms: f64,
    success_rate: f64,
    last_success: u64,
}

// After N failures on the primary, the daemon:
// 1. Switches to the next fallback in the chain
// 2. Logs the degradation to audit
// 3. Periodically re-tests the primary (when system is idle)
// 4. If primary recovers, switches back automatically
// 5. Reports the active path in SystemCapabilities
```

**Example flow:**
```
t=0:  a11y.click works → continue using a11y
t=10: a11y.click fails (AT-SPI daemon crashed) → try semantic_click
t=10: semantic_click succeeds → use semantic_click for all clicks
t=60: idle test of a11y.click → still down
t=300: idle test of a11y.click → works → switch back to a11y
```

**Protocol response enrichment:**

```json
{
  "status": "ok",
  "data": { "focused": "terminal-3" },
  "_degraded": {
    "primary": "a11y.focus",
    "fallback_used": "semantic_click",
    "reason": "AT-SPI unresponsive (attempts: 3, last_error: connection refused)",
    "auto_recovered": false
  }
}
```

**Effort:** ~300 lines. Metrics tracker + dynamic fallback executor + recovery
tester.

---

## 91. Desktop Application Management

### apps.defaults

**What's Missing:** No way to query or set default applications. Currently
agents can launch apps by window ID or by guessing the command, but can't
discover which app handles `*.pdf` or what terminal is configured.

**Implementation:** Wrap `xdg-mime`, `gio mime`, and DE-specific backends.

```rust
// Protocol actions
AppsDefaultsQuery { mime_type: String },  // "text/html" → "firefox.desktop"
AppsDefaultsSet { mime_type: String, desktop_file: String },
AppsDefaultsList,                           // all registered MIME→app mappings
AppsDefaultsListCategories { category: String },  // "x-scheme-handler/http"

// Per-DE backends
// XDG base:   xdg-mime query default text/html
// GNOME:      gio mime text/html firefox.desktop
// KDE:        kde-config --filetypes (or kreadconfig5)
// Generic:    parse ~/.config/mimeapps.list
```

**Effort:** ~150 lines. Thin wrappers around xdg-mime/gio.

### apps.recent

**What's Missing:** Agents can't see what files the user recently worked on.
No programmatic access to GTK/KDE recent document stores.

**Implementation:**

```rust
AppsRecentList { limit: Option<u32>, filter: Option<String> },
AppsRecentOpen { uri: String },
AppsRecentClear,

// Backends:
// GTK:  parse ~/.local/share/recently-used.xbel (XML)
// KDE:  parse ~/.local/share/kactivitymanagerd/ or kate-specific recent file lists
//     or: use gio recent --list
```

**Effort:** ~100 lines. XBEL parser + wrappers.

### apps.autostart

**What's Missing:** Agents can't manage what starts on user login. No
programmatic access to XDG autostart.

**Implementation:**

```rust
AppsAutostartList,        // files in ~/.config/autostart/, /etc/xdg/autostart/
AppsAutostartEnable { desktop_file: String },  // copy/symlink to autostart dir
AppsAutostartDisable { desktop_file: String }, // remove from autostart dir
AppsAutostartCreate { name, command, comment, icon, delay }, // write .desktop
```

Each autostart entry is a `.desktop` file in `~/.config/autostart/`. Enable =
copy, disable = remove, create = write with `X-GNOME-Autostart-enabled=true`.

**Effort:** ~100 lines. File CRUD in autostart directories.

### apps.launchers

**What's Missing:** App catalog (section 16) parses installed `.desktop` files
for discovery. It doesn't create, edit, or validate them.

**Implementation:**

```rust
AppsLauncherCreate {
    name: String,
    command: String,
    icon: Option<String>,
    categories: Vec<String>,
    terminal: bool,
    comment: Option<String>,
},
AppsLauncherEdit { path: String, updates: LauncherField },
AppsLauncherValidate { path: String },
```

Write `.desktop` files to `~/.local/share/applications/`. Validate parses the
file and checks: required fields present, icon exists, command executable
exists, Exec field is properly escaped.

**Effort:** ~150 lines. .desktop file writer + validator.

---

## 92. Compositor / Window Manager Rules

### windows.rules

**What's Missing:** Persistent compositor rules. Currently windows are
manipulated in real time (move, resize, focus). Rules survive restarts.

**Implementation:** Per-compositor backend for persistent window rules:

```rust
WindowsRuleCreate {
    app_id: String,                    // "firefox"
    properties: WindowRuleProperties,  // what to enforce
}

pub struct WindowRuleProperties {
    always_on_top: Option<bool>,
    always_on_visible_workspace: Option<bool>,
    maximized: Option<bool>,
    workspace: Option<String>,
    opacity: Option<f64>,
    no_focus: Option<bool>,
    border: Option<String>,
    monitor: Option<String>,
    x: Option<i32>,
    y: Option<i32>,
    w: Option<u32>,
    h: Option<u32>,
}

WindowsRuleList,
WindowsRuleDelete { rule_id: String },

// Backends:
// Hyprland:   hyprctl keyword windowrule + windowrulev2
// KDE:        kwin_scripting (dbus) to install window rules
// GNOME:      limited, but can use gsettings for some
// Wayland:    ext-foreign-toplevel protocol, compositor-specific
```

**Effort:** ~300 lines. Per-compositor rule generator + KWin script backend.

---

## 93. Workspace Lifecycle

### workspaces.create / rename / delete / reorder

**What's Missing:** Deskbrid can list, switch, and move windows between
workspaces. It can't create new workspaces, rename them, delete empty ones,
or reorder them.

**Implementation:**

```rust
WorkspaceCreate { name: Option<String> },       // compositor assigns ID
WorkspaceRename { id: String, name: String },
WorkspaceDelete { id: String },                  // errors if not empty
WorkspaceReorder { order: Vec<String> },         // array of workspace IDs

// Per-compositor:
// Hyprland:  hyprctl dispatch workspace new
// KDE:       dbus org.kde.KWin.VirtualDesktopManagement.createDesktop
// GNOME:     dynamic workspaces are auto-created/auto-removed, limited control
// X11:       n/a — virtual desktops are static
```

**Effort:** ~200 lines. Dispatch to compositor-specific commands + error handling.

---

## 94. Advanced File Metadata

### files.stat / files.metadata

**What's Missing:** File CRUD (section "already built") reads/writes/copies/moves
files. It doesn't report permissions, owner, group, timestamps, symlink targets,
xattrs, or ACLs.

**Implementation:**

```rust
FilesStat { path: String },
// Returns:
{
  "permissions": "rwxr-xr-x",
  "owner": "1000",
  "group": "1000",
  "size": 1423,
  "modified": "2026-05-20T12:00:00Z",
  "created": "2025-11-03T08:15:00Z",
  "accessed": "2026-05-20T11:45:00Z",
  "type": "file",  // or "dir", "symlink", "fifo", "socket"
  "symlink_target": null,
  "xattrs": { "user.foo": "..." },
  "acl": {
    "user": [{"name": "jeremy", "perms": "rwx"}, ...],
    "group": [{"name": "staff", "perms": "rx"}, ...],
    "other": "r-x"
  }
}
```

Uses `std::fs::metadata`, `std::os::unix::fs::MetadataExt`, and `xattr` crate
for extended attributes. ACL parsing via `acl` crate or `getfacl` command.

**Effort:** ~150 lines. Metadata reader + xattr + ACL parser.

### files.trash

**What's Missing:** XDG portal trash (roadmap, section 9) mentions moving files
to trash. No trash listing, restore, or empty.

**Implementation:**

```rust
FilesTrashList,           // list trashed files with original paths + deletion times
FilesTrashRestore { path: String },
FilesTrashEmpty,
```

Files in trash are under `~/.local/share/Trash/files/` with info in
`~/.local/share/Trash/info/` (FreeDesktop Trash specification *.trashinfo files).

**Effort:** ~100 lines. Trashinfo parser + file mover.

### files.archive

**What's Missing:** No zip/tar/tar.gz/tar.zst support. Agents that want to
package results or extract data have to shell out.

**Implementation:**

```rust
FilesArchiveCreate {
    archive_path: String,     // "output.tar.gz"
    source_paths: Vec<String>,
    format: String,            // "zip", "tar", "tar.gz", "tar.zst"
    compression_level: Option<u8>,
},
FilesArchiveExtract { archive_path: String, dest_dir: String },
FilesArchiveList { archive_path: String },
```

Use Rust crates: `zip`, `tar`, `flate2`, `zstd`. No shelling out to `tar` or
`unzip`. Structured error reporting with per-entry failure granularity.

**Effort:** ~200 lines. Archive crate wrappers.

---

## 95. Storage Monitoring

### storage.usage

**What's Missing:** Agents have no way to check disk space, find the biggest
directories, or get low-space alerts.

**Implementation:**

```rust
StorageUsage { path: Option<String> },
// Returns: { total, used, free, percent_used, mount_point, filesystem }
StorageUsageScan { path: String, max_depth: Option<u32> },
// Returns: sorted list of largest directories/files under path

// Backend: nix crate + std::fs for filesystem info
// Periodic events:
storage.low_space → fires when any mount crosses a threshold
storage.events.subscribe → "warning" at 90%, "critical" at 95%
```

**Effort:** ~150 lines. Filesystem stats + tree walker.

---

## 96. System Pressure / PSI

### system.pressure

**What's Missing:** Linux Pressure Stall Information (PSI) tells agents about
CPU, memory, and IO pressure. Agents can use this to decide whether to
proceed, back off, or retry.

**Implementation:**

```rust
SystemPressure,
// Returns:
{
  "cpu": {
    "some": { "avg10": 0.45, "avg60": 0.30, "avg300": 0.20, "total": 123456 },
    "full": { "avg10": 0.10, "avg60": 0.05, "avg300": 0.02, "total": 45678 }
  },
  "io": { "some": { "avg10": 2.1, ... }, "full": { ... } },
  "memory": { "some": { "avg10": 0.8, ... }, "full": { ... } },
  "swap_pressure": "none",       // or "low", "medium", "high"
  "oom_risk": "low"              // PSI memory full × duration heuristic
}
```

Backend: Read `/proc/pressure/{cpu,io,memory}`. Kernel ≥4.20 required. If
unavailable (no CONFIG_PSI), fall back to `/proc/stat` for CPU + `/proc/meminfo`
for memory. Emit events: `system.pressure.high`, `system.pressure.recovered`.

**Effort:** ~80 lines. Procfs reader + threshold calculator.

---

## 97. Network Firewall Management

### network.firewall

**What's Missing:** Deskbrid can manage network connections (nmcli, wifi)
but has no firewall awareness. Agents can't open/close ports or manage rules.

**Implementation:**

```rust
NetworkFirewallRuleCreate {
    name: String,
    direction: String,        // "in" | "out"
    protocol: String,         // "tcp" | "udp"
    port: u16,
    action: String,           // "allow" | "deny"
    source: Option<String>,   // IP or CIDR, default "any"
    destination: Option<String>,
    profile: Option<String>,  // named profile, scoped to Deskbrid's agent processes
},

NetworkFirewallRuleList,
NetworkFirewallRuleDelete { rule_id: String },
NetworkFirewallProfileCreate { name, rules },
NetworkFirewallProfileApply { name },
NetworkFirewallStatus,
```

Backend priority: `nftables` (nft CLI or `nftnl` crate), `firewalld` (D-Bus),
`ufw` (CLI), fallback to `iptables`. Detect which is available on startup.

**Scoping:** Named profiles let Deskbrid apply firewall rules specifically
for agent processes using cgroups + fwmark + nftables sets.

**Effort:** ~350 lines. Multi-backend firewall adapter.

---

## 98. Network Proxy Management

### network.proxy

**What's Missing:** No awareness of system proxy settings. Agents that make
HTTP requests need to respect the proxy.

**Implementation:**

```rust
NetworkProxyGet,
// Returns: { http: "http://proxy:8080", https: "...", socks: "...", no_proxy: ["*.local", "10.*"] }

NetworkProxySet {
    http: Option<String>,
    https: Option<String>,
    socks: Option<String>,
    no_proxy: Option<Vec<String>>,
    mode: Option<String>,    // "manual", "auto", "disabled"
},

NetworkProxyPac { url: Option<String> },  // proxy auto-config
```

Backend priority: GNOME (gsettings), KDE (kwriteconfig5), env (`$http_proxy`),
DE-agnostic (`/etc/environment` or `network proxy` CLI). On set, broadcast
`ProxyChanged` event.

**Effort:** ~150 lines. Per-DE proxy reader/writer.

---

## 99. Captive Portal Detection

### network.captive_portal

**What's Missing:** Agents can't distinguish "network down" from "captive
portal waiting for login."

**Implementation:**

```rust
NetworkCaptivePortalStatus,
// Returns: { state: "none" | "detected" | "logged_in", portal_url: Option<String>, detected_by: "connectivity" | "dns" | "http" }

// Detection methods:
// 1. DNS probe — resolve a known URL (connectivitycheck.gstatic.com)
// 2. HTTP probe — GET http://connectivitycheck.gstatic.com/generate_204
//    → 204 means connected, 302/200 means captive portal
// 3. NetworkManager — check NM's ConnectivityCheck state via D-Bus
```

**Effort:** ~100 lines. HTTP probe + NM D-Bus query.

---

## 100. Input Method Engine Control

### ime

**What's Missing:** Keyboard layout switching (section 15) handles layouts.
IME control handles actual input method engines like IBus, Fcitx5 —
Chinese/Japanese/Korean input, emoji pickers, composition state.

**Implementation:**

```rust
ImeListEngines,
// Returns: [{"name": "ibus-libpinyin", "language": "zh", "longname": "Chinese (Intelligent Pinyin)"}, ...]

ImeCurrentEngine,
// Returns current engine + composition mode

ImeSwitchEngine { engine_name: String },
ImeSetCompositionMode { mode: String },  // "direct", "composition", "latin"
ImeRestart,
```

Backend: D-Bus over IBus (`org.freedesktop.IBus`) or Fcitx5. Common
on all East Asian Linux desktops. Not installed on pure English setups,
so needs graceful degradation (status: "not_available").

**Effort:** ~200 lines. D-Bus IBus/Fcitx control.

---

## 101. Virtual Monitor Support

### monitor.virtual

**What's Missing:** Deskbrid controls real monitors. No way to create virtual
headless outputs for remote agents, screen recording with desktop context,
or headless testing.

**Implementation:**

```rust
MonitorVirtualCreate {
    name: String,
    width: u32,
    height: u32,
    refresh_rate: Option<u32>,
    scale: Option<f64>,
    format: Option<String>,  // "DRM_FORMAT_XRGB8888", etc.
},
MonitorVirtualList,
MonitorVirtualDestroy { name: String },
```

Backend: GNOME has `mutter.virtual-monitor` via D-Bus (works with screencast
streams). KDE has `KDECurseVirtualMonitor` via KWin scripting. Wayland has
`wlr-output-management` protocol extension. X11 has XRandR 1.5 dummy outputs.

**Effort:** ~300 lines. Per-DE virtual monitor backend.

---

## 102. Monitor Color Management

### monitor.color

**What's Missing:** No ICC profile reading, night light, gamma, HDR, or VRR
status.

**Implementation:**

```rust
MonitorColorGet { monitor_id: String },
// Returns: { icc_profile, night_light_active, gamma_r/g/b, hdr_enabled, vrr_enabled, color_temperature }

MonitorColorSetNightLight { monitor_id: Option<String>, enable: bool, temperature: Option<u32> },
MonitorColorSetGamma { monitor_id: String, r: f64, g: f64, b: f64 },
MonitorColorSetProfile { monitor_id: String, icc_path: String },
MonitorColorSetHdr { monitor_id: String, enable: bool },
```

Backend: GNOME has `gsettings` for night light, GNOME Color Manager D-Bus for
profiles. KDE has `kwriteconfig5` + kscreen. HDR awareness via `HDR_MODE`
connector property in DRM (kernel 6.12+).

**Effort:** ~250 lines. Per-DE color backend + DRM property reader.

---

## 103. Bluetooth Profile & Device Control

### bluetooth.profile / bluetooth.devices

**What's Missing:** Bluetooth pairing (section 55) covers connect/disconnect/
pair. It doesn't handle audio profiles (A2DP vs HFP), codec info, trust/block,
or device battery levels.

**Implementation:**

```rust
BluetoothProfileList { device_address: String },
// Returns: [{"uuid": "0000110b-0000-1000-8000-00805f9b34fb", "name": "A2DP Sink", "connected": true}, ...]

BluetoothProfileSet { device_address: String, profile: String },
// Switch from A2DP (high quality audio) to HFP (headset mic)

BluetoothDeviceInfo { device_address: String },
// Battery, signal strength, paired/trusted/blocked, connected profiles, vendor info

BluetoothTrust { device_address: String },
BluetoothBlock { device_address: String },
```

**Effort:** ~200 lines. BlueZ D-Bus profile switching + device info.

---

## 104. Print (CUPS) Control

### print

**What's Missing:** No printer awareness. Agents can't list printers, check
queues, or manage print jobs.

**Implementation:**

```rust
PrintListPrinters,
// Returns: [{"name": "Brother_HL-L2370DW", "location": "Office", "status": "idle", "default": true}]

PrintDefaultPrinter { printer: Option<String> },
PrintJobList,
PrintJobCancel { job_id: String },
PrintJobPause { job_id: String },
PrintJobResume { job_id: String },
```

Backend: D-Bus via `org.cups.cupsd` or CLI wrappers around `lpstat`, `lpadmin`,
`cancel`. CUPS is standard on all major distros.

**Effort:** ~150 lines. CUPS D-Bus + CLI wrappers.

---

## 105. Scanner (SANE) Support

### scan

**What's Missing:** No scanner support. Agents can't digitize documents.

**Implementation:**

```rust
ScanListScanners,
// Returns: [{"name": "brother3:net1;dev0", "vendor": "Brother", "model": "DCP-L2540DW", "connection": "net", "status": "idle"}]

Scan {
    device: String,
    resolution: Option<u32>,      // default 300 DPI
    color_mode: Option<String>,   // "color", "gray", "lineart"
    format: Option<String>,       // "png", "jpeg", "pdf"
    duplex: Option<bool>,
    source: Option<String>,       // "flatbed", "adf"
    batch: Option<bool>,          // if ADF, scan all pages
    page_size: Option<String>,    // "a4", "letter"
},
```

Backend: `scanimage` CLI (part of SANE). If `scanimage` is not installed,
return `"status": "not_available"`. SANE is widely available but not universal.

**Effort:** ~150 lines. scanimage CLI wrappers + output file handling.

---

## 106. Privacy Device Monitoring

### privacy.devices

**What's Missing:** Agents can't see what's using the camera, mic, or screen
sharing. No way to revoke unwanted access.

**Implementation:**

```rust
PrivacyDevicesList,
// Returns:
{
  "camera": [{"pid": 1234, "app": "zoom", "state": "active", "since": "12:00:00"}],
  "mic": [{"pid": 1234, "app": "zoom", "state": "active"}, {"pid": 5678, "app": "firefox", "state": "active"}],
  "screen_share": [{"pid": 1234, "app": "zoom", "state": "active"}]
}

PrivacyDeviceRevoke { app_pid: u32, device: String },
PrivacyDeviceSubscribeAll,     // receive events when any device goes active
```

Backend: GNOME has `org.gnome.Mutter.DisplayConfig` and PipeWire registry
for camera. KDE has KScreenDbus. PipeWire registry nodes tell you all active
streams. For camera specifically: `/sys/class/video4linux/` + lsof.

**Effort:** ~250 lines. PipeWire + procfs inspection.

---

## 107. XDG / Flatpak Portal Permissions

### portals.permissions

**What's Missing:** No visibility into which Flatpak apps have what portal
grants. Agents debugging why an app can't access files/camera/mic need
this.

**Implementation:**

```rust
PortalsPermissionsList { app_id: Option<String> },
// Returns: {"app_id": "org.mozilla.firefox", "permissions": {"camera": "granted", "location": "ask", "background": "granted"}}

PortalsPermissionsRevoke { app_id: String, permission: String },
PortalsPermissionsStore,  // read the portal permissions DB
```

Backend: Flatpak's permission store is at
`~/.local/share/flatpak/overrides/` and XML files in
`~/.local/share/xdg-desktop-portal/`. XDG Desktop Portal tracks per-app
permissions.

**Effort:** ~150 lines. Portal config file parser.

---

## 108. Do Not Disturb / Notification Policy

### notifications.dnd

**What's Missing:** Notification history (section 57) and actions (section 58).
No way to toggle DND, set quiet hours, or manage per-app notification policy.

**Implementation:**

```rust
NotificationsDndGet,
// Returns: { enabled: true, quiet_hours: { start: "22:00", end: "07:00" }, exceptions: ["phone", "alarms"] }

NotificationsDndSet { enable: bool },
NotificationsDndQuietHours { start: String, end: String },
NotificationsDndSetPerApp { app_id: String, policy: String },  // "allow", "silence", "block"
NotificationsDndGetPerApp { app_id: String },
```

Backend: GNOME → `gsettings get org.gnome.desktop.notifications show-banners`.
KDE → `knotifyrc` → `kwriteconfig5 --file knotifyrc ...`. Per-app policy via
GNOME notification settings schemas or KDE's notification categories.

**Effort:** ~150 lines. Per-DE DND backend.

---

## 109. SSH / GPG Agent Awareness

### auth.agents

**What's Missing:** Deskbrid can launch processes (`process.run`). No awareness
of whether SSH agent or GPG agent is running, what keys are loaded,
or if they're reachable.

**Note:** This provides *presence, health, and availability* — never load keys,
never read private key data, never expose key material.

**Implementation:**

```rust
AuthAgentGet {
    agent_type: String,     // "ssh" | "gpg" | "gpg-ssh"
},
// Returns:
{
  "available": true,
  "socket_path": "/run/user/1000/gnupg/S.gpg-agent.ssh",
  "identities_count": 2,
  "identities": [
    {"type": "ed25519", "fingerprint": "SHA256:...", "comment": "jeremy@coemedia"},
    {"type": "rsa", "fingerprint": "SHA256:...", "comment": "work-email@company.com"}
  ],
  "locked": false,
  "forwarding": { "enabled": true, "active_sessions": 1 }
}

AuthAgentForwardingStatus,    // SSH agent forwarding health
AuthAgentLock { agent_type: String },
AuthAgentUnlock,              // prompts user for passphrase
```

**Security:** Return fingerprints only (hashes, not keys). Never expose raw
key data. `gpg-connect-agent` for GPG. `ssh-add -l` for SSH identities.
Socket path detection via `$SSH_AUTH_SOCK` and `$GPG_AGENT_INFO`.

**Effort:** ~200 lines. Socket detection + identity enumeration (fingerprints
only).

---

## 111. Layout Profiles (Window Snapshots)

**What's Missing:** Agents manipulate windows in real time but can't save and
restore full desktop layouts — window positions, sizes, workspaces, minimized
states, active workspace. No "reset to dev layout" command.

**Implementation:** Named layout profiles that capture and restore full window
state:

```rust
LayoutProfileCapture {
    name: String,
    scope: Option<String>,    // "all" | "current_workspace" | "app:firefox"
}
// Returns: { profile_name: "dev-layout", window_count: 12, captured_at: "..." }

LayoutProfileRestore { name: String },
LayoutProfileList,
LayoutProfileDelete { name: String },
LayoutProfileDiff { name: String },
// Compare current desktop vs. a saved profile — list windows that moved/changed

// Storage: JSON files in ~/.local/share/deskbrid/layouts/{name}.json
// Format:
{
  "name": "dev-layout",
  "monitors": [{"name": "DP-1", "x": 0, "y": 0, "w": 1920, "h": 1080}],
  "active_workspace": "1",
  "workspaces": {
    "1": [
      {"app_id": "Alacritty", "title": "~/projects/api", "x": 0, "y": 0, "w": 960, "h": 540,
       "workspace": "1", "minimized": false, "monitor": "DP-1"}
    ]
  }
}
```

**Restore process:** For each window, match by app_id+title (fuzzy), then:
1. Move to correct monitor × workspace
2. Set position and size
3. Restore minimized state
4. Activate the saved workspace

**Effort:** ~300 lines. State capture + fuzzy matching + per-window restore.

---

## 112. Gamepad / Joystick Input Injection

**What's Missing:** Deskbrid injects keyboard and mouse events. No gamepad or
joystick support. Agents controlling games, simulators, or any application with
gamepad input can't interact.

**Implementation:** Create a virtual gamepad via uinput and inject events:

```rust
GamepadCreate { name: Option<String> },          // create virtual gamepad
GamepadDestroy { id: String },
GamepadButtonPress { id: String, button: String },  // "a", "b", "x", "y", "l1", "r1", "l2", "r2", "start", "select", "home"
GamepadButtonRelease { id: String, button: String },
GamepadAxisMove { id: String, axis: String, value: f64 },   // "left_stick_x", "left_stick_y", "right_stick_x", "right_stick_y", "l2_analog", "r2_analog"
GamepadDPad { id: String, direction: String },  // "up", "down", "left", "right"
GamepadList,                                      // list virtual gamepads
```

**Backend:** uinput — open `/dev/uinput`, write `input_event` structs. Same
pattern as the existing virtual keyboard/mouse but with `BUS_BLUETOOTH` and
`ABS_*` axes + `BTN_*` buttons. Maps to Xbox/PlayStation controller layout.

**Crate:** Use existing `uinput` dependency, or extend the current keyboard/mouse
uinput module.

**Effort:** ~200 lines. uinput virtual gamepad + event injection.

---

## 113. RGB / Peripheral Lighting (OpenRGB)

**What's Missing:** No control over keyboard, mouse, or case lighting. Agents
that need visual feedback via hardware lighting (flash red on error, pulse blue
on task completion) can't do it.

**Implementation:** Interface with OpenRGB SDK (network protocol) or openrgb CLI:

```rust
RgbListDevices,
// Returns: [{"name": "Razer Huntsman Elite", "type": "keyboard", "zones": ["logo", "keys", "bottom"]}]

RgbSetColor {
    device: String,
    zone: Option<String>,
    color: String,              // hex: "#ff0000"
    mode: Option<String>,       // "static", "breathing", "wave", "reactive"
    speed: Option<u8>,          // 0-100
    brightness: Option<u8>,
},

RgbSetProfile { device: String, profile_name: String },
RgbSaveProfile { device: String, name: String },
```

**Backend:** OpenRGB SDK via TCP (default port 6742). If OpenRGB isn't running,
fall back to `openrgb --device 0 --color ff0000`. For Chroma/Razer-specific:
OpenRazer kernel driver (`razerkbd` module).

**Effort:** ~200 lines. OpenRGB SDK protocol + CLI fallback.

---

## 114. Desktop Search (Tracker / Recoll)

**What's Missing:** Unified search (Section 80) searches open windows, files,
clipboard, and audit logs. It doesn't query Tracker SPARQL or Recoll Xapian
indexes — so agents can't find emails, documents, images, or chat history.

**Implementation:** Backend adapters for Tracker SPARQL (GNOME) and Recoll:

```rust
DesktopSearchQuery {
    query: String,                           // "financial report q3"
    backend: Option<String>,                 // "tracker", "recoll", "auto"
    limit: Option<u32>,
    scope: Option<Vec<String>>,              // restrict to "email", "documents", "images", "files"
}

// Returns:
[{
  "uri": "/home/jeremy/Documents/report.pdf",
  "title": "2026 Q3 Financial Report",
  "snippet": "...financial report...",
  "score": 0.87,
  "mime_type": "application/pdf",
  "backend": "tracker",
  "modified": "2026-05-15T10:30:00Z"
}]

DesktopSearchIndexStatus,
// Returns: { tracker: { running: true, indexed_files: 45231, last_update: "..." }, recoll: { available: false } }
```

**Backend:** Tracker — D-Bus over `org.freedesktop.Tracker3.Sparql`.
```bash
$ busctl call org.freedesktop.Tracker3.Sparql /org/freedesktop/Tracker3/Endpoint org.freedesktop.Tracker3.Sparql.Query "s" "SELECT nie:url(?u) WHERE { ?u a nfo:Document } LIMIT 5"
```
Recoll — CLI wrapper around `recoll -t -q "query"`.

**Effort:** ~250 lines. Tracker D-Bus SPARQL queries + Recoll CLI adapter.

---

## 115. Greeter / Display Manager Control

**What's Missing:** Section 23 covers `session.lock`, `session.unlock`,
`session.switch_user`. It doesn't control the greeter/display manager — GDM,
KDM, LightDM — for tasks like styling the lock screen, switching to a different
user at the DM level, or detecting which DM is running.

**Implementation:**

```rust
DisplayManagerStatus,
// Returns: { name: "gdm", version: "45.0", active: true, sessions: [...], greeter_running: true }

DisplayManagerListUsers,
// Returns: [{"username": "jeremy", "display_name": "Jeremy Coe", "session_type": "wayland", "uid": 1000, "logged_in": true, "seat": "seat0"}, ...]

DisplayManagerSwitchUser { username: String },
DisplayManagerRestart,           // restart display manager (disruptive for multi-seat)
DisplayManagerSetAutoLogin { username: Option<String> },
```

**Backend:** GDM has D-Bus on `/org/gnome/DisplayManager/Manager`. LightDM has
`lightdm` D-Bus interface. KDM uses KDE's KDM control protocol. Detect via
`cat /etc/systemd/system/display-manager.service -> /usr/lib/systemd/system/gdm.service`.

**Effort:** ~200 lines. Per-DM D-Bus interface wrappers.

---

## 116. Session Environment Variable Management

**What's Missing:** No way to read or modify environment variables in the
user's session. Agents that need to set `$EDITOR`, `$BROWSER`, or `$PATH`
have no programmatic way.

**Implementation:**

```rust
SessionEnvGet { name: String },          // single var
SessionEnvGetAll,                         // all env vars
SessionEnvSet { name: String, value: String, persistent: Option<bool> },
SessionEnvUnset { name: String, persistent: Option<bool> },
SessionEnvRestore,                        // reload from ~/.profile, ~/.config/environment.d/
```

**Backend:** Read/write from `/proc/self/environ` for current state. For persistent
changes: `~/.config/environment.d/*.conf` (systemd user-session), `~/.pam_environment`,
or `~/.profile`. Broadcast `EnvironmentChanged` event on change.

**Security:** Permission check — only actions that match `env.set.*` can modify
environment. Changes don't affect already-running processes (that's not possible
on Linux).

**Effort:** ~100 lines. Procfs reader + dotfile writer.

---

## 117. Prometheus / OpenTelemetry Export

**What's Missing:** Audit log (Section 34) and benchmarking (Section 37) record
data internally. No standard observability export. Agents and operators can't
monitor Deskbrid health in their existing dashboards.

**Implementation:** A `/metrics` HTTP endpoint and event exporter:

```rust
MetricsExportPrometheus,
// Returns: Prometheus text format metrics
// deskbrid_actions_total{action="windows.list",status="ok"} 1423
// deskbrid_actions_duration_ms{action="screenshot",p50="42",p95="156",p99="312"}
// deskbrid_clients{state="connected"} 3
// deskbrid_sessions{state="active"} 2
// deskbrid_audit_error_total{code="permission_denied"} 12

MetricsExportOtel {
    endpoint: String,                      // "http://localhost:4318/v1/traces"
    batch_size: Option<u32>,
    interval_s: Option<u64>,
},
// Export spans for each action execution with trace context propagation

MetricsExportConfig {
    prometheus_port: Option<u16>,           // default 9090, disabled if 0
    otel_endpoint: Option<String>,          // OpenTelemetry collector URL
    otel_service_name: String,
    attributes: Option<HashMap<String, String>>,
}

// Startup:
// deskbrid daemon --metrics-port 9090
```

**Structured logging integration:** Extend existing audit log to include
trace_id, span_id, and baggage. Clients pass `traceparent` headers in their
WebSocket/Unix socket requests, daemon propagates them through the action
lifecycle.

**Effort:** ~300 lines. Prometheus metrics registry + OTLP HTTP exporter.

---

## 118. Cross-Distro Package Management

**What's Missing:** Deskbrid can launch processes and run commands. No
structured, cross-distro abstraction for installing, removing, updating
software. Agents that need to install dependencies have to know the distro.

**Implementation:** Abstract package manager with backend detection:

```rust
PackageList { query: Option<String> },
// Returns: [{"name": "firefox", "version": "130.0", "source": "apt", "size": 142000}]

PackageInstall { names: Vec<String>, source: Option<String> },
PackageRemove { names: Vec<String> },
PackageUpdate { names: Option<Vec<String>> },   // omit for system update
PackageSearch { query: String },
PackageRepair { name: String },                  // dpkg --configure -a / rpm --rebuilddb

// Backends (auto-detected):
// apt:      apt-get install -y, dpkg-query -W
// dnf:      dnf install -y, rpm -qa
// pacman:   pacman -S, pacman -Q
// flatpak:  flatpak install, flatpak list
// snap:     snap install, snap list
```

**Progress events:** Long operations emit `package.install.progress` events
with percentage and package name.

**Scope:** Each backend is a separate permission category (`package.apt.*`,
`package.flatpak.*`). Agents can be restricted to only flatpak.

**Effort:** ~350 lines. Backend detection + CLI wrappers with progress parsing.

---

## 119. Nix / Guix Awareness

**What's Missing:** No awareness of Nix or Guix package environments. Agents
running on NixOS or using nix-shell need to know if they're in a Nix store
context.

**Implementation:**

```rust
NixStatus,
// Returns: { installed: true, in_nix_shell: false, nix_version: "2.24", store_path: "/nix/store", channels: ["nixpkgs"], current_profile: "/nix/var/nix/profiles/default" }

NixListPackages,
NixSearch { query: String, source: Option<String> },   // "nix", "nixpkgs"
NixRun { package: String, command: Vec<String> },       // nix run nixpkgs#hello
NixShell { packages: Vec<String> },                     // nix-shell -p

GuixStatus, GuixList, GuixSearch, GuixInstall,
```

**Effort:** ~100 lines. CLI wrappers + Nix store detection.

---

## 120. TPM / Hardware Security

**What's Missing:** No access to hardware security features — TPM PCR quotes,
attestation, hardware random number generator. Agents that need to prove they're
running on a specific machine or access HW RNG can't.

**Implementation:**

```rust
TpmStatus,
// Returns: { available: true, version: "2.0", manufacturer: "Intel", pcrs: ["0-23"], active_session: false }

TpmPcrRead { pcr: u16 },
// Returns: { pcr: 7, value: "0x3A458FE2...", bank: "sha256" }

TpmPcrQuote { pcr: Vec<u16>, nonce: String },
// Returns: signed quote, can be verified by a remote party

TpmAttestationKey,
// Returns: public key attestation — proves this machine's TPM signed it

HardwareRng { bytes: u32 },
// Returns: hardware random bytes (via /dev/hwrng or TPM2_GetRandom)
```

**Backend:** `tpm2-tools` CLI (`tpm2_pcrread`, `tpm2_quote`, `tpm2_getrandom`)
or the `tss-esapi` Rust crate. `/dev/hwrng` for hardware RNG if available.

**Security:** These actions expose measurement data, not secrets. PCR values
are inherently public. The attestation key is a public key that proves TPM
presence.

**Effort:** ~250 lines. tpm2 CLI wrappers + /dev/hwrng reader.

---

## 121. Headless Wayland Compositor (CI / Testing)

**What's Missing:** Deskbrid can only be tested against a real desktop. No
synthetic Wayland compositor for automated testing in CI or headless
environments.

**Implementation:** Launch a headless Weston or wlroots instance, connect
Deskbrid to it:

```rust
CompositorHeadlessStart {
    width: u32,
    height: u32,
    name: String,
    backend: String,            // "weston", "cage", "kwin_test"
    virtual_monitors: Option<u32>,
},
CompositorHeadlessStop { name: String },
CompositorHeadlessStatus,
```

**Backend:** `weston --backend=headless-backend.so --width=1920 --height=1080`
for Weston. `cage` for minimal wlroots. KWin has `KWIN_WAYLAND_TEST_MODE`.
PipeWire virtual sinks for screen capture testing.

**Integration:** The test compositor registers itself as `$WAYLAND_DISPLAY`.
Deskbrid auto-detects it. Tests can then run any Deskbrid action against the
synthetic desktop.

**Effort:** ~300 lines. Compositor process lifecycle + socket env management.

---

## 122. Mock Backend for Agent Testing

**What's Missing:** No way to run Deskbrid actions against a simulated desktop
without affecting the real one. Developers and agents need deterministic test
environments.

**Implementation:** A `--mock` mode that replaces all DE backends with
deterministic stubs:

```bash
deskbrid daemon --mock

# All actions return realistic but fake data:
# windows.list → returns 3 fake windows
# screenshot → returns a 1920x1080 gray image with mock UI
# keyboard.type → returns OK, no actual input
# a11y.tree → returns a fake but valid a11y tree with buttons/text fields
```

```rust
// Mock specification files (~/.config/deskbrid/mocks/):
// scenario.json — define fake windows, a11y trees, clipboard state, screenshots
// response_rules.json — define what each action returns

// Mock-specific actions:
MockScenarioLoad { path: String },
MockScenarioRun { name: String },        // play through a scenario deterministically
MockScenarioTeardown,
MockActionRecord { duration_s: u64 },    // record all dispatched actions for replay
MockActionVerify { expected: Vec<MockAction> },  // verify the agent took the right actions
```

**Mock backends override the normal dispatch:** Each real backend (GNOME, KDE,
Hyprland) has a `MockBackend` counterpart that returns fixed data. The daemon
operation is identical — only the backend drivers swap.

**Effort:** ~400 lines. Mock backend implementations + scenario format.

---

## 123. Plugin Hot-Reload

**What's Missing:** Plugin system (Section 82) loads `.wasm` files at startup.
No way to update plugins without restarting the daemon.

**Implementation:** Watch plugin directory for changes and hot-reload:

```rust
// On file change (inotify or fanotify):
// 1. Read the new .wasm
// 2. Validate: compile, check manifest, check permissions
// 3. If valid: instantiate new plugin, register its actions
// 4. If existing actions are mid-flight: let them complete, drain old plugin
// 5. Switch new plugin in atomically
// 6. Broadcast PluginUpdated { name, version_old, version_new }

// Error handling:
// If new plugin fails to compile: log error, keep old version
// If new plugin crashes on first action: revert to old version
// If plugin has breaking API changes: return error, don't reload
```

**Mechanism:** `inotify` on `~/.local/share/deskbrid/plugins/` for file
creation/modification. Each `.wasm` file's mtime is tracked. On change, compile
in a separate WASM `Store` (isolated), then swap the `Linker` entry atomically.

**Effort:** ~200 lines. File watcher + atomic plugin swap.

---

## 124. Graceful Restart & Config Live-Reload

**What's Missing:** Restarting the daemon drops all client connections. No way
to reload `permissions.toml` or `config.toml` without restarting.

**Implementation:** Two independent features:

```rust
// Config live-reload:
// Watch ~/.config/deskbrid/config.toml and permissions.toml for changes
// When changed: re-read, validate, apply new values
// For safe fields (rate limits, log level, metrics config): apply immediately
// For unsafe fields (socket path, backend config): flag for restart

DaemonReloadConfig,
// Returns: { reloaded: true, changes: {"permissions": "3 rules updated", "rate_limit": "from 100 to 200"} }

// Graceful restart:
DaemonRestart {
    delay_ms: Option<u64>,      // announce restart N ms before it happens
    drain_timeout_ms: Option<u64>,  // max time to wait for in-flight actions
}
// 1. Broadcast DaemonRestarting { reason, delay }
// 2. Stop accepting new connections
// 3. Wait for in-flight actions to complete (or timeout)
// 4. Persist state to SQLite
// 5. Exec into new binary (same pid, new process)
// 6. On startup: restore state, re-accept connections
// 7. Broadcast DaemonReady
```

**Persistent state file:** The daemon writes a state file (`~/.local/share/deskbrid/state.json`)
before restart: connection tokens, active sessions, in-flight action IDs. On
startup, re-hydrate from this file.

**Effort:** ~300 lines. File watcher + fd-passing exec restart.

---

## 125. Auto-Update with Rollback

**What's Missing:** No update mechanism. Users manually download new binaries.
No checks for updates, no safe rollback.

**Implementation:** Built-in update checker with rollback support:

```rust
UpdateCheck,
// Returns: { current: "v0.6.0", latest: "v0.7.0", published: "2026-06-01", critical: false, changelog_url: "..." }

UpdateDownload { version: Option<String> },
// Returns: { progress: 45, speed: "2.3 MB/s", estimated: "12s" }

UpdateApply,
// 1. Verify downloaded binary's GPG signature
// 2. Back up current binary to ~/.local/share/deskbrid/backups/v0.6.0
// 3. Swap binary
// 4. Trigger graceful restart (Section 124)
// 5. On crash within 60s: revert to backup automatically

UpdateRollback,
// Returns: { rollback_to: "v0.6.0", reason: "v0.7.0 crashed within startup timeout" }

UpdateHistory,
// Returns: [{version: "v0.7.0", installed: "2026-06-02", status: "current"}, {version: "v0.6.0", installed: "2026-05-01", status: "backup"}]
```

**Version source:** GitHub Releases API (`https://api.github.com/repos/coe0718/deskbrid/releases`).
Binary verification: GPG-signed checksums (minisign or signify).

**Channel support:**
```toml
[updates]
channel = "stable"           # "stable", "beta", "nightly"
auto_check = true
auto_update = false          # download but don't auto-apply
rollback_on_crash_seconds = 60
```

**Effort:** ~300 lines. GitHub API version checker + binary verifier + executor.

---

## 126. Shared Memory / Zero-Copy Buffer Passing

**What's Missing:** Screenshots, screen recordings, and large file transfers
are copied over the Unix socket. For multi-client scenarios or frequent
screenshots, this wastes both memory and time.

**Implementation:** Use `memfd` (Linux `memfd_create`) or Unix socket
ancillary data (`SCM_RIGHTS`) to pass file descriptors:

```rust
// When a client requests a screenshot:
// Option A: Return a memfd fd via SCM_RIGHTS
// Client reads the shared memory directly — no copy
// Option B: Return the path to a memfd-backed mmap'd buffer
// Client mmaps the same region

// Protocol:
BufferAllocate {
    size: u64,
    flags: Option<Vec<String>>,    // "seal_write", "seal_shrink"
}
// Returns: { buffer_id: "buf-a1b2c3", size: 4096000 }

BufferWrite { buffer_id: String, offset: u64, data: Vec<u8> },
BufferRead { buffer_id: String, offset: u64, length: u64 },
BufferFree { buffer_id: String },

// For screenshots:
Screenshot {
    direct_buffer: Option<bool>,  // if true, write into a shared buffer
}
// Returns: { buffer_id: "buf-a1b2c3", format: "png", size: 2400000 }
```

**Backend:** `memfd_create()` syscall + mmap. Rust's `memfd` crate or raw
`libc::memfd_create()`. File descriptors passed via Unix domain socket's
`SCM_RIGHTS` ancillary data.

**Security:** Sealed memfds (via `memfd_create(MFD_ALLOW_SEALING)` + `F_SEAL_SEAL`)
prevent clients from growing the buffer beyond the allocated size.

**Effort:** ~200 lines. Memfd allocation + SCM_RIGHTS fd passing.

---

## 127. Locale / Timezone Change Events

**What's Missing:** Locale or timezone changes silently break agent assumptions
about time formatting, language, or date calculations. No event for these changes.

**Implementation:** Watch for locale/timezone changes and broadcast events:

```rust
SystemLocaleGet,
// Returns: { lang: "en_US.UTF-8", lc_time: "en_DK.UTF-8", lc_numeric: "en_US.UTF-8" }

SystemLocaleSet {
    lang: Option<String>,
    lc_time: Option<String>,
    lc_numeric: Option<String>,
    persistent: Option<bool>,
},

SystemTimezoneGet,
// Returns: { timezone: "America/New_York", utc_offset: -4, dst_active: true, dst_name: "EDT" }

SystemTimezoneSet { timezone: String },
```

**Events emitted:**
- `locale.changed` — fired when any LC_* value changes
- `timezone.changed` — fired when timezone changes (deamons automatically update)

**Backend:** Read `/etc/localtime` (or `timedatectl`) for timezone. `locale`
command for locale. `localectl set-locale` for setting. DBus signal
`org.freedesktop.locale1` for change detection.

**Effort:** ~80 lines. File watcher + DBus signal listener.

---

## 128. btrfs / zfs Snapshot Integration

**What's Missing:** No filesystem-level snapshot support. Agents that want to
"save state before risky operation, rollback if it fails" have no way to do this
at the filesystem level.

**Implementation:** Backend detection + snapshot operations:

```rust
SnapshotCreate {
    path: String,                  // mount point or subvolume
    name: Option<String>,
    recursive: Option<bool>,
},
// Returns: { snapshot_id: "...", path: ".../.snapshots/..." }

SnapshotList { path: Option<String> },
// Returns: [{id: "...", name: "pre-update", created: "...", size: "...", fs: "btrfs"}]

SnapshotRollback { id: String },
SnapshotDelete { id: String },
SnapshotClone { id: String, target_path: String },
```

**Backend detection:**
- btrfs: `btrfs subvolume snapshot -r`, `btrfs subvolume list`, `btrfs subvolume delete`
- zfs: `zfs snapshot`, `zfs list -t snapshot`, `zfs rollback`, `zfs destroy`
- LVM: `lvcreate --snapshot`, `lvremove`, `lvconvert --merge`
- None: return `"code": "NOT_SUPPORTED"` with message explaining requirement

**Effort:** ~200 lines. btrfs/zfs CLI wrapper + filesystem type detection.

---

## 129. Priority Roadmap

### Tier 1 — Highest Agent Leverage (build next)

| Feature | Status | Effort | Impact | Reason |
|---|---|---|---|---|
| **Audit log** | ✅ Done | Low (~200 lines, ring buffer) | High | Trail for debugging and security as agent actions get more powerful |
| **Action timeouts** | ✅ Done | Low (~150 lines, timeout wrapper) | High | Prevents hung commands and long-running backend calls from wedging workflows |
| **Dry-run mode** | ✅ Done | Trivial (~80 lines, dispatch flag) | Medium | Validates sequences before executing them |
| **Rate limiting** | ✅ Done | Low (~200 lines, token bucket) | Medium | Prevents runaway agents from saturating the daemon |
| **Capabilities reporting** | ✅ Done | Low (caps crate) | Medium | Tells agents what they can and cannot do on this machine |
| **Confinement detection** | ✅ Done | Low (env checks only) | High | Prevents confusing failures in Flatpak/Snap/sandboxed environments |
| **Clipboard history** | ✅ Done | Low (~200 lines, ring buffer) | Medium | Retrieves and searches old clipboard entries |
| **App catalog (.desktop)** | ✅ Done | Low (~200 lines, ini parser) | Medium | Helps agents discover installed launchable applications |
| **MPRIS media control** | ✅ Done | Low (~300 lines, zbus calls) | Medium | Pauses audio before recording and exposes current media context |
| **Color picker** | ✅ Done | Trivial (~80 lines, `image` crate already dep) | Medium | Pixel sampling for visual verification |
| **Window tiling presets** | ✅ Done | Low (~150 lines, helper over existing) | Medium | Tiles windows without forcing agents to compute pixel coordinates |
| **Drag & drop** | ✅ Done | Very low (~100 lines, 4 backends) | Medium | Helps with file managers, design tools, and browser upload zones |
| **sysfs brightness/backlight** | ✅ Done | Very low (std::fs only) | Medium | Works across all DEs, no new deps |
| **sysfs thermal/CPU** | 🧭 Planned | Very low (std::fs only) | Medium | Useful for monitoring and simple read-only hardware insight |

### Tier 2 — Moderate Value, Moderate Effort

| Feature | Effort | Impact | Reason |
|---|---|---|---|
| **Keyboard layout mgmt** | Low (~250 lines, per-DE commands) | Medium | Know/switch keyboard layouts for correct typing |
| **Desktop settings** | Medium (~300 lines, gsettings/kconfig) | Medium | Toggle dark mode, change font, accessibility |
| **Session/user mgmt** | Medium (~250 lines via login1 + per-DE) | Medium | Lock screen, list users, session info |
| **Named sessions** | Low (~250 lines, session map) | Medium | Multi-agent isolation on the same daemon |
| **Action recording/replay** | Medium (~400 lines, capture + file store) | Medium | Record macros, replay sequences, automation |
| **Cron scheduler** | Medium (~350 lines, `cron` crate) | Medium | Scheduled actions without external cron |
| **D-Bus raw access** | Low (~200 lines, zbus + JSON conversion) | Medium | Escape hatch for unwrapped interfaces |
| **Secret/keyring** | Medium (~300 lines, `secret-service` crate) | Medium | Secure credential storage for agents |
| **cgroups sandbox** | Medium (cgroups-rs) | Medium | Protect daemon from runaway processes |
| **udev events** | Medium (udev crate + event thread) | Medium | React to hardware changes |

### Tier 3 — Specialized / Future

| Feature | Effort | Impact | Reason |
|---|---|---|---|
| **Screen recording (finish)** | Medium (400-600 lines, PipeWire frames) | Medium | Marketing demos, agent recording playback |
| **TCP mode (network control)** | Medium (~400 lines, TLS + token auth) | Medium | Control remote machines over network |
| **Remote screenshot streaming** | Medium (~300 lines, JPEG streaming) | Medium | Live visual feedback for remote agents |
| **XDG Portal integration** | Medium | Medium | Official API for DE features |
| **Desktop Portal settings** | Low (zbus calls) | Medium | Read dark mode, accent, font, cursor |
| **DBus application menus** | High (~1000+ lines) | Low | Complex, niche use case |
| **fanotify** | Medium | Low-Medium | Overkill for most agent use cases |
| **eBPF** | High | Low | Only valuable for security monitoring |
| **GPIO** | Low (gpio-cdev) | Low | Edge case (SBCs like Raspberry Pi) |

---

## Crate Dependency Checklist

| Crate | Status | Version | Notes |
|---|---|---|---|
| `zbus` | ✅ Already in | 5.x | Already depends on this |
| `zbus_systemd` | ⏭️ Optional follow-up | 0.26.0 | Current implementation uses `systemd-inhibit`, `loginctl`, `systemctl`, and `journalctl`; native DBus can replace CLI wrappers later |
| `zbus_polkit` | ⏭️ Optional follow-up | 5.0.0 | Current implementation uses `pkcheck` and `deploy/org.deskbrid.policy`; native `AuthorityProxy` can replace CLI checks later |
| `caps` | ❌ Needs add | 0.5.6 | Pure Rust, no system deps |
| `cgroups-rs` | ❌ Needs add | latest | Native Rust v1/v2 |
| `udev` | ❌ Needs add | 0.9+ | Bindings to libudev |
| `procfs` | ❌ Needs add | 0.16+ | Pure Rust, no system deps |
| `notify` | ✅ Already has | 7.x | Already depends on this |
| `inotify` | (via notify) | — | Already covered |
| `fanotify` | ❌ Maybe | — | Only if high-value use case emerges |
| `selinux` | ❌ Maybe | latest | Only if SELinux detection needed |
| `leptess` | ❌ Optional | latest | Only for in-process OCR (vs CLI tesseract) |
| `portable-pty` | ❌ Needs add | latest | PTY multiplexer for interactive terminals |
| `cron` | ❌ Needs add | latest | Cron expression parsing for scheduler |
| `secret-service` | ❌ Needs add | latest | Freedesktop Secret Service API bindings |
| `aes-gcm` | ❌ Optional | latest | AES-256-GCM for fallback encrypted keyring |
| `argon2` | ❌ Optional | latest | Key derivation for fallback encrypted keyring |

---

## Permission Model Impact

The existing `permissions.toml` glob-matching system handles everything in the doc.
Each new action needs:

1. An `action_name()` mapping in `permissions.rs` (e.g., `ServiceStart { .. } => "service.start"`)
2. A category in `public_action_types()` (e.g., `"service.*"`, `"journal.*"`, `"device.*"`)
3. A `SystemCapabilities` entry showing what's available and any degraded mode

For polkit-backed actions, the dispatch layer should:
1. Check permission toml (existing)
2. If allowed, check polkit (new)
3. If polkit needs interaction, return `"code": "AUTHENTICATION_REQUIRED"` with the
   action_id so clients can trigger a polkit dialog

---

## Summary

Deskbrid at v0.6.0 has excellent DE-level control. Adding these mechanisms would
make it a true **OS-level agent runtime** — not just a desktop controller.

**Highest ROI features to implement first:**
1. **Terminal/PTY multiplexer** — the single biggest gap. Coding agents need interactive terminals.
2. **OCR screenshot fallback** — agents read any window, not just a11y-accessible ones.
3. `systemd inhibit` and `journal query` — agents prevent sleep and read logs.
4. `sysfs brightness/backlight/thermal` — zero deps, works everywhere.
5. `capabilities reporting` — tell agents what's available.
6. `confinement detection` — stop confusing failures in Flatpak/Snap.

These six give agents practical system control with minimal code changes. The terminal
multiplexer alone (~500-800 lines) unlocks more agent capability than everything else
combined.
