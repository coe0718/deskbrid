---
name: deskbrid
description: Linux desktop HAL for AI agents — keyboard, mouse, clipboard, screenshots, windows, workspaces, MCP server, AT-SPI2, browser CDP, file ops, MPRIS, systemd, and more. Backends for GNOME, KDE, Hyprland, COSMIC, and X11.
---

# Deskbrid Desktop Control (v0.7.0)

Deskbrid is a Unix socket daemon + MCP server that wraps GNOME Shell, KDE, Hyprland, COSMIC, DBus, NetworkManager, BlueZ, PipeWire, and Wayland utilities into a JSON protocol. Any agent or script can control the full desktop.

**v0.7.0 highlights (79 commits, +31.6K / −6.5K lines):** COSMIC backend shipped, AT-SPI2 rebuild + MCP server (20 tools), uinput AbsPointer, browser CDP control, file operations, MPRIS media, color picker, app catalog, clipboard history, screenshot OCR + diffing, terminal PTY sessions, systemd/polkit controls, layout profiles, activate-or-launch, monitor controls, 8-file refactor (gnome/kde/hyprland/cosmic/x11/cli/daemon/protocol split into modular dirs).

## Installation

The **one-shot install script** auto-detects distro + DE and installs everything:

```bash
bash <(curl -fsSL https://deskbrid.patchhive.dev/install.sh)
```

What it does: detects distro (apt/pacman/dnf/zypper/apk), detects DE (GNOME/KDE/Hyprland/X11), installs deps per DE, sets up `/dev/uinput` udev rules, configures `ydotoold` autostart, and downloads the binary to `/usr/local/bin/deskbrid`.

See the dedicated [`deskbrid-install`](../deskbrid-install/SKILL.md) skill for the full install playbook.

**Agent discovery files in repo root:**
- `CLAUDE.md` — read by Claude Code, OpenCode, Gemini CLI, and other shelldex.com agents
- `AGENTS.md` — Hermes generic agent convention file
- `site/` directory — landing page at `deskbrid.patchhive.dev`, install.sh hosted there

## Quick Start

```bash
deskbrid daemon                                   # start the daemon
deskbrid mcp                                      # MCP stdio server (AI coding tools)
deskbrid health                                   # check deps + backend
echo '{"type":"system.info","id":"1"}' | nc -U    \
  /run/user/1000/deskbrid.sock -w 2               # test the socket
```

## AT-SPI + MCP (shipped v0.6.0+)

Deskbrid ships an MCP (Model Context Protocol) stdio server and expanded AT-SPI2 accessibility tools. AI coding tools (Claude Code, Cursor, Codex) can control the desktop through Deskbrid's MCP interface.

See `references/mcp-atspi-protocol.md` for the full tool mapping, protocol actions, module structure, and migration path from computer-use-linux.

### AT-SPI Connection Caching

The AT-SPI2 bus connection (`connect_a11y()`) is cached via `tokio::sync::OnceCell`. The first call opens a DBus session + AT-SPI2 bus connection; subsequent calls clone the `Arc`-backed `Connection` for free. This prevents opening a new connection on every AT-SPI action — critical for deep tree traversals and MCP tool calls that hit `find_all()` repeatedly.

```rust
static A11Y_CONN: OnceCell<Connection> = OnceCell::const_new();

pub async fn connect_a11y() -> anyhow::Result<Connection> {
    let conn = A11Y_CONN.get_or_try_init(|| async { /* ... */ }).await?;
    Ok(conn.clone())
}
```

**Pitfall:** If `connect_a11y()` creates a new `Connection::session()` + `GetAddress` + `Builder::address().build()` on every call, it leaks DBus connections. Always cache — zbus `Connection` is cheap to clone.

## Compositor Compatibility

Deskbrid v0.3.0 auto-detects the running desktop environment and loads the appropriate backend. Detection order: `$XDG_CURRENT_DESKTOP` → process scan (`pgrep Hyprland`, `pgrep kwin_wayland`) → GNOME fallback.

| Compositor | Protocol | Status | Backend |
|------------|----------|--------|---------|
| **GNOME (Mutter)** | Wayland | ✅ Fully supported | Mutter RemoteDesktop DBus + GNOME Shell extension |
| **Hyprland** | Wayland | ✅ Supported (v0.3.0) | `hyprctl` JSON CLI for windows/workspaces/dispatch; `ydotool` for input; `grim` for screenshots |
| **KDE (KWin)** | Wayland | ✅ Supported (v0.4.1) | KWin D-Bus + Scripting API + `ydotool` (runs as user!) + `spectacle` + ImageMagick `convert -crop` for screenshots; see `references/kde-backend-implementation.md` |
| **COSMIC (cosmic-comp)** | Wayland | ✅ Supported (v0.7.0) | Wayland protocols via `cosmic-helper` binary — `zcosmic_toplevel_manager_v1` + `ext_foreign_toplevel_list_v1` + `zcosmic_workspace_handle_v1`. Helper is a short-lived CLI inside the session. Non-windowing ops (grim, ydotool, wl-copy/paste) work standard. |
| **Cinnamon** | X11 | 🔲 Partial (v0.7.0) | Shared X11 window listing via xdotool + sharing X11 backend helpers |
| **MATE** | X11 | 🔲 Partial (v0.7.0) | Shared X11 window listing |
| **Generic X11** | X11 | 🔲 Partial (v0.7.0) | Shared X11 window listing + xrandr geometry |
| **Sway** | Wayland | ❌ Not planned | wlroots-based, no RemoteDesktop equivalent |

### Hyprland Backend (v0.3.0)

Merged to `main` at commit `ece80a6`. Full `DesktopBackend` trait implementation at `src/backend/hyprland.rs` (~1,050 lines). Branch was deleted after merge.

### KDE Backend (v0.4.1 — Implemented, Reviewed)

**Status:** ✅ Fully implemented, Claude-reviewed, and tested. Source at `src/backend/kde.rs` (~860 lines). Detection code (`DesktopEnv::Kde`) and `pgrep kwin_wayland` check exist in `mod.rs`. Routes to `kde::KdeBackend::new()`. Tested live on turtle (KDE 6.6.4 Plasma) — windows list returns native windows, system info identifies KWin.

**Architecture:** Input, clipboard, screenshots, system, network, bluetooth, audio, and files are shared with Hyprland backend (same tools: ydotool, wl-clipboard, nmcli, bluetoothctl, pactl, find). Screenshots use `spectacle` + ImageMagick (KDE-specific, since `grim` needs wlr-screencopy). The KDE-specific portion is windows + workspaces, managed via KWin D-Bus and scripting API.

**Window management via KWin JS scripting:**
- **Window listing:** `workspace.windowList()` → JSON-serialize via `print(JSON.stringify({id, title, app_id, geometry, active, minimized, pid}))`
- **Window focus:** `workspace.activeClient = w` — property assignment on the workspace object
- **Window get by ID:** Loop `workspace.windowList()` matching `String(w.internalId)` or `String(w.resourceClass)`
- **Output capture:** JS `print()` goes to KWin's journal output. Deskbrid reads via `journalctl _COMM=kwin_wayland` with marker-based extraction. Script loaded via `dbus-send` `org.kde.kwin.Scripting.loadScript`, run via `org.kde.kwin.Script.run`, stopped via `org.kde.kwin.Script.stop`.

**Workspace management via D-Bus:**
- **Simple switching:** `qdbus org.kde.KWin /KWin org.kde.KWin.setCurrentDesktop <int>` — 1-based desktop number
- **Virtual desktop listing:** KWin scripting — `workspace.virtualDesktopManager.desktops[]` with `d.x11DesktopNumber` and `d.name`
- **Move window to workspace:** Script assigns `w.desktops = [manager.desktops[N]]`

**Key API discoveries (tested on KDE 6.6.4, Plasma 6):**
- Window `internalId` is a UUID string like `{4ef80e0d-6c35-4268-ba62-09ac91b9c918}` — NOT a numeric windowId
- `resourceClass` returns a QByteArray — use `.toString()` in JS
- `w.activate` is a boolean property, NOT a method. To focus: assign `workspace.activeClient = w`
- `windowList()` returns ALL windows including plasmashell panels, desktop, etc.
- Virtual desktops use UUIDs internally (e.g., `2e069c28-58a3-4cd5-b6c3-7a59360509ba`)
- Scripting object paths are `/Scripting/Script<N>` (changed from `/<N>`)
- `loadScript` returns `int32` (NOT uint32)
- Screenshots available via `org.kde.KWin.ScreenShot2` but uses pipe FD — simpler to use `spectacle` + ImageMagick `convert -crop`

**Async safety (fixed v0.4.1, Claude review):** `kwin_js()` originally used `std::fs::File::create` and `std::fs::remove_file` inside an async function — blocking the tokio runtime. Fixed to `tokio::fs::write` and `tokio::fs::remove_file`. Same for `detect_desktop()` which used blocking `std::process::Command::new("pgrep")` instead of `tokio::process::Command`. **Never use std::fs or std::process::Command inside async fns in this codebase.** See `references/kde-backend-implementation.md` for the full async safety section.

**`kwin_js()` helper (core of the KDE backend):**
1. Wraps JS with unique marker strings (e.g., `KWIN_DESKBRID_<nanotimestamp>`)
2. Writes to temp file `/tmp/deskbrid_kwin_<PID>.js` via `tokio::fs::write` (async-safe)
3. Loads via `dbus-send` → parses `int32 <N>` from response
4. Runs via `org.kde.kwin.Script.run`
5. **Polls journalctl in a loop** (max 10 × 100ms = 1s) — exits as soon as the marker appears. Fast on responsive systems (~100ms), still catches slow journald flushes (full 1s)
6. Reads `journalctl _COMM=kwin_wayland --since "30 seconds ago" -o cat`
7. Extracts lines between marker strings
8. Strips `"js: "` prefix from each line
9. Stops script and cleans up temp file via `tokio::fs::remove_file` (async-safe)

**Why polling instead of fixed sleep:** The old 500ms sleep wasted time on fast systems and was too short on loaded systems. The poll loop adapts to system speed.

**Pitfalls:**
- Script load MUST use `dbus-send --print-reply` to get return value; response type is `int32 <N>`
- Scripting object path must be `/Scripting/Script<N>`, NOT `/<N>` (mid-2024 path change)
- `journalctl` requires privilege — works on Arch/EndeavourOS with default journald config
- Windows with empty captions (plasmashell panels) will have `title: ""`
- **Workspace ID (v0.4.1 fix):** Originally hardcoded to `0` for all windows. Now populated via `w.desktops[0].x11DesktopNumber` in the KWin JS — both `windows_list` and `window_get` pass `ws` in JSON and Rust reads `w["ws"].as_i64().unwrap_or(0) as u32`.
- Window matching via `internalId` vs `resourceClass` — the UUID is the canonical key
- Temporal `--since` is imprecise; marker-based parsing is more reliable
- **Always run `cargo fmt` after `cargo clippy --fix`** (clippy scrambles formatting; verified on this codebase)

**Detection:** Auto-detected at startup via `$XDG_CURRENT_DESKTOP` containing "kde" or "plasma", fallback to `pgrep -x kwin_wayland` success.

See `references/kde-backend-implementation.md` for the full API enumeration, D-Bus method signatures, window property reference, workspace UUID system, and the verified scripting workflow.

### COSMIC Backend (v0.7.0 — Shipped)

**Status:** ✅ Merged to main, shipped in v0.7.0. Source at `src/cosmic/` (split from monolithic `cosmic.rs`). Helper binary at `src/bin/cosmic_helper.rs`.

**Architecture:** Unlike GNOME (Shell extension + DBus) and KDE (KWin DBus scripting), COSMIC exposes window and workspace management through **native Wayland protocols** only. Wayland client objects from the `wayland-client` crate are **not `Send + Sync`** -- they can't live inside the async `DesktopBackend` trait. Solution: a **helper binary** (`cosmic-helper`) pattern, same as `computer-use-linux-cosmic`.

**Helper binary architecture:**

```
deskbrid daemon  --spawns-->  cosmic-helper list-windows
                              cosmic-helper activate --window-id N
                              cosmic-helper probe
                              | (Wayland protocol connection within session)
                              v
                              cosmic-comp
```

The helper is a short-lived CLI that connects to the Wayland compositor, registers protocol listeners, collects window/workspace state, outputs JSON to stdout, and exits.

**Wayland protocols used:**

- `ext_foreign_toplevel_list_v1` -- window enumeration (list, focused)
- `zcosmic_toplevel_manager_v1` -- window control (activate, close, maximize, minimize, fullscreen, sticky, move to workspace)
- `zcosmic_toplevel_info_v1` -- window metadata (title, app_id, state, bounds, pid, workspace)
- `zcosmic_workspace_handle_v1` -- workspace control (activate, deactivate, rename, set tiling state)
- `wl_seat` -- input device (required for activate())

**Helper CLI:**

```
cosmic-helper probe                  JSON probe (capabilities)
cosmic-helper list-windows           JSON array of WindowInfo
cosmic-helper focused-window         JSON single WindowInfo
cosmic-helper activate --window-id N
cosmic-helper close --window-id N
cosmic-helper maximize --window-id N
cosmic-helper minimize --window-id N
cosmic-helper workspace-list         JSON array of workspace info
cosmic-helper workspace-activate --id N
cosmic-helper move-to-workspace --window-id N --workspace-id N
```

**Non-windowing (no helper needed):** grim, ydotool, wl-copy/paste, notify-send all work on COSMIC standardly.

**Detection:**

```rust
if desktop.to_lowercase().contains("cosmic") {
    return DesktopEnv::Cosmic;
}
if pgrep -x cosmic-comp succeeds {
    return DesktopEnv::Cosmic;
}
```

**Cargo.toml additions:**

```toml
[[bin]]
name = "cosmic-helper"
path = "src/bin/cosmic_helper.rs"
required-features = ["cosmic"]
```

With `cosmic-protocols`, `wayland-client`, `wayland-protocols` behind a `cosmic` feature flag.

**Reference implementation:** computer-use-linux-cosmic by Avi Fenesh (MIT, ~10K SLoC). Uses same `cosmic-protocols` crate. Already supports probe, list-windows, focused-window, activate-window. Deskbrid's helper will extend with close, maximize, minimize, workspace ops.

**Pitfalls:**
- Wayland protocol connection must happen within the compositor session -- can't call from SSH without proper env
- DesktopBackend trait requires `Send + Sync` -- Wayland client objects don't satisfy this
- Activation state needs short TTL (~5s) so focused-window returns just-activated window
- COSMIC does NOT expose a DBus API -- don't probe busctl for cosmic services
- COSMIC lacks wlr-gamma-control -- wlsunset/gammastep won't work
- Turtle test rig has COSMIC v1.0.13-1 selectable from SDDM

See `references/cosmic-backend-research.md` for full protocol API, implementation checklist, and computer-use-linux source analysis.

**Architecture:**
```\nsrc/backend/\n├── mod.rs          — DesktopBackend trait + detect_desktop() + backend factory\n├── gnome.rs        — GnomeBackend (v0.2.0, unchanged)\n├── hyprland.rs     — HyprBackend (v0.3.0)\n└── kde.rs          — KdeBackend (v0.4.0, new)\n```

**Tool mapping:**
- Windows/focus: `hyprctl clients -j` / `hyprctl dispatch focuswindow`
- Workspaces: `hyprctl workspaces -j` / `hyprctl dispatch workspace`
- Input: `ydotool type` / `ydotool key` / `ydotool click` (requires `ydotoold` running)
- Screenshots: `spectacle` (full screen) + ImageMagick `convert -crop` (window/region crops via geometry)
- Clipboard: `wl-paste`/`wl-copy` (same as GNOME)
- Notifications: `notify-send` + `makoctl dismiss`
- Network: `nmcli` (avoids DBus dependency)
- Bluetooth: `bluetoothctl`
- Audio: `pactl` — all three backends. Uses `pactl list sinks` (verbose, parses `Volume:` percentage + `Mute: yes/no` — NOT `pactl list short sinks` which lacks volume/mute fields) (PipeWire compat layer)
- Battery: `/sys/class/power_supply/BAT*/` (no UPower needed)
- System info: `hyprctl version` + workspace enumeration
- Power: `systemctl suspend/poweroff/reboot` + `loginctl lock-session` + KDE: `qdbus6 org.kde.ksmserver /KSMServer logout`
- Idle detection: `/dev/input/event*` modification timestamps
- File watching: `notify` crate (same as GNOME)

**Test rig (\"turtle\"):** EndeavourOS on Haswell laptop (192.168.1.244, jeremy@), **4 DEs installed** — **COSMIC** (1.0.13, Rust-native), **Hyprland** (0.54.3), **KDE Plasma** (6.6.4 Wayland), **GNOME Shell** (46.3), all selectable at SDDM login. COSMIC installed via `sudo pacman -S cosmic`. Also has Cinnamon/MATE/X11 on the roadmap for cross-DE testing. Wired Ethernet (RTL810xE, 100Mbps) recommended over flaky RTL8188EE WiFi. Deskbrid binary at ~/deskbrid. Rebuild and scp after any Rust source changes, especially before cutting a release.

**Pitfalls:**
- **`ydotoold` must be running for input injection to work** (started via `exec-once = ydotoold` in hyprland.conf)
- **`/dev/uinput` permissions block on Arch/EndeavourOS:** By default, `/dev/uinput` is `crw-------` (root-only). ydotool needs write access. Fix: add a udev rule and reload:
  ```
  echo 'KERNEL=="uinput", GROUP="input", MODE="0660"' | sudo tee /etc/udev/rules.d/99-input.rules
  sudo chmod 0660 /dev/uinput && sudo chgrp input /dev/uinput
  ```
  The user must be in the `input` group. After the udev rule is in place, the permission survives reboots (though an immediate `chmod` is needed for the current boot).
- **ydotoold started from SSH may not work:** Even with correct permissions, `ydotoold` started from an SSH session may not inject into the Hyprland session properly. Prefer starting it via `hyprctl dispatch exec ydotoold` from within the session, or via `exec-once` in `hyprland.conf`.
- `hyprctl` requires `HYPRLAND_INSTANCE_SIGNATURE` env var — only available within the Hyprland session
- **SSH remote control requires THREE env vars:** `HYPRLAND_INSTANCE_SIGNATURE` (hyprctl), `WAYLAND_DISPLAY=wayland-1` (grim), and `XDG_RUNTIME_DIR=/run/user/1000` (socket resolution). All must be set on EVERY command (daemon + client). The daemon auto-detects these via `detect_hypr_instance()` on startup by scanning `/run/user/*/hypr/` for the newest instance directory by mtime. See `references/hyprland-test-rig.md` for discovery pattern and `references/ssh-daemon-env-debug.md` for systematic debugging when detection fails.
- **`hyprctl version` exits code 0 even when it fails.** When `HYPRLAND_INSTANCE_SIGNATURE` is unset, it prints the error to **stdout** (not stderr) and exits 0 — so you can't catch the failure with exit code checking. Always use `hyprctl -j version` (JSON mode) which returns a proper error on failure and `{"version":"0.54.3"}` on success. Note the JSON key is `"version"` (no "v" prefix); the response also contains `branch`, `commit`, `tag`, `dirty`, and other metadata.
- **Hyprland client JSON uses `at[x,y]` and `size[w,h]` arrays**, NOT flat fields. Field names: `address` (hex window ID), `at[0]` (x), `at[1]` (y), `size[0]` (width), `size[1]` (height), `class` (app_id), `title`, `workspace.id`, `focusHistoryID` (0=focused), `pid`, `mapped`, `floating`, `fullscreen`. The monitor JSON from `hyprctl -j monitors` uses flat `width`, `height`, `x`, `y`, `name`, `id`, `scale`, `focused`, `refreshRate`.
- **Instance auto-detection:** The `detect_hypr_instance()` free function reads `$XDG_RUNTIME_DIR/hypr/` (fallback: `/run/user/1000/hypr/`), lists subdirectories, sorts by mtime, and returns the newest instance's directory name as the signature. Also reads `.wayland_socket` symlink for `WAYLAND_DISPLAY` (falls back to `wayland-1`). This runs at daemon startup in `HyprBackend::new()` before any commands are issued.
- **Wayland env propagation:** Setting `env()` on spawned `Command` objects is preferred over modifying the daemon's own `std::env`. Set `XDG_RUNTIME_DIR`, `WAYLAND_DISPLAY`, and `HYPRLAND_INSTANCE_SIGNATURE` on every hyprctl, grim, wl-copy/paste, and notify-send command. Use the per-command `.env()` call, not `std::env::set_var()` (which is `unsafe` in edition 2024).
- **Empty-string env var trap:** `std::process::Command.env("KEY", "")` sets the env var to an *empty string*, which is functionally worse than unset. Always guard with `if !sig.is_empty()` before calling `.env()`.
- **"Text file busy" when redeploying:** If `scp` fails with "dest open: Failure" when deploying a new deskbrid binary, there's a daemon process still running that holds the old binary open. Kill ALL instances (including orphaned ones from previous SSH sessions): `ssh host "pkill -x deskbrid 2>/dev/null; sleep 1"` before attempting `scp` again. If `scp` still fails, use the pipe-through-SSH workaround: `cat target/release/deskbrid | ssh host "cat > /home/user/deskbrid && chmod +x /home/user/deskbrid"`.
- Mouse positioning uses absolute coordinates via `ydotool mousemove --absolute` (unlike GNOME which uses relative motion)
- **Always run `cargo fmt` after `cargo clippy --fix`** — clippy's auto-fix scrambles formatting. Without it, `cargo fmt --check` in CI fails. Run `cargo clippy --fix --allow-dirty && cargo fmt` as a single step (use `--allow-dirty` because `clippy --fix` stages changes).
- **`ydotool rec` is NOT a real ydotool command.** Don't use it. For scroll, use `ydotool mousemove --wheel <dx> <dy>`. The old code had `ydotool rec mousemove ...` which would fail silently (caught by Claude code review).
- **Shell pipes don't work in `Command::new("find")`.** Passing `|`, `head -n`, `2>/dev/null` as literal arguments to `find` doesn't work — they're not interpreted by a shell. Use Rust's `.take(n)` on the output lines instead. The old `files_search` had this bug in the Hyprland backend; the GNOME backend was already correct.
- **Hermes config file socket path must match actual daemon socket.** `hermes/deskbrid.toml` had `$XDG_RUNTIME_DIR/deskbrid/socket` (wrong — doesn't exist) instead of `$XDG_RUNTIME_DIR/deskbrid.sock` (correct — this is what the daemon actually binds). The daemon itself always uses the correct path; the bug was only in the Hermes tool config. Verify socket paths in config files independently.

See `references/mutter-remotedesktop-session-api.md` for the full Mutter RemoteDesktop DBus Session API (introspection XML, all method signatures, keysym table, connection-lifetime gotcha).
See `references/mutter-screencast-api.md` for the ScreenCast API exploration — RecordMonitor, RecordVirtual, and why absolute mouse positioning failed on GNOME 46.
See `templates/waybar-top-only.jsonc` and `templates/waybar-top-only.css` for a minimal waybar config (single top bar) for Hyprland test rigs — prevents the default two-bar layout.

## Quick Test

### One-command setup (v0.4.1+)

Auto-detects your DE and does the right thing:

```bash
deskbrid setup
# GNOME:   installs + enables Shell extension
# Hyprland: prints ydotool setup tips
# KDE:     prints ydotoold autostart tips
```

### Daemon Test

```bash
# Daemon must be running — check socket
ls /run/user/1000/deskbrid.sock

# Test connection
echo '{"type":"system.info","id":"1"}' | nc -U /run/user/1000/deskbrid.sock -w 2
```

### nc Connection Lifecycle

Every `nc` invocation opens a **fresh** Unix socket connection. The daemon sends a `connected` message on each new connection with `seq:0`. The per-connection `seq` counter resets — two sequential `nc` commands both start at `seq:0` → `seq:1`.

```bash
# Each command is a separate connection — seq resets
echo '{"type":"windows.focus","id":"focus","seq":1}' | nc -U $SOCKET -w 2
# Response: {"type":"connected","seq":0} then {"data":{"focused":"code"},"seq":1}

echo '{"type":"input.keyboard","action":"type","text":"hi","id":"type","seq":2}' | nc -U $SOCKET -w 2
# Response: {"type":"connected","seq":0} then {"data":{"typed":2},"seq":1}  ← seq:1, NOT seq:2
```

**`&&` chaining pitfall:** When chaining with `&&`, the second `nc` opens a new connection and gets its OWN `connected` message + response. Each `nc` sees only its own `seq:0` connected + `seq:1` action response. The `seq` in your JSON is meaningless for request/response matching across invocations.

**Timeout:** Always use `-w 3` or higher. Some actions take longer than default. `-w 2` is sufficient for most simple actions but may cut off responses. If you see `[Command timed out]` but the action visibly worked, it was a timeout on reading the response, not a failure to execute.

**Stale socket after daemon kill:** If you `pkill deskbrid` and restart, the socket file may persist. The new daemon overwrites it — no need to `rm`. But the old daemon's DBus session is gone; the new daemon must be started with the correct `DBUS_SESSION_BUS_ADDRESS`.

## GNOME Shell Extension

Path: `~/.local/share/gnome-shell/extensions/deskbrid@deskbrid/extension.js`

The extension provides window/workspace management over DBus. It's required for `windows.list`, `windows.focus`, `workspaces.list`, `workspaces.switch`, and `workspaces.move_window`.

### 🚨 Collateral Damage of GNOME Shell Restarts

**Restarting gnome-shell doesn't just break the extension — it nukes `gnome-keyring-daemon`, which takes the `gh` OAuth token with it.** If you or an agent need to push code after a gnome-shell restart, `gh auth status` will report "The token in default is invalid" and `git push` will fail with "No such device or address."

The token itself isn't expired — gnome-keyring-daemon was killed when gnome-shell died, and the token was only stored in its in-memory session. The fix is to use the `GH_TOKEN` env var instead of relying on the keyring (see `github-auth` skill for the full approach).

**TL;DR:** If you restart gnome-shell on a machine where agents push code, re-auth `gh` or better yet, wire `GH_TOKEN` in the agent's `.env` so it survives the restart.

### Reloading the Extension on GNOME 46 (Wayland)

**Critical:** `ReloadExtension` is deprecated and broken on GNOME 46+. `gnome-extensions enable/disable` toggles the gsettings key but does NOT reload the extension. The DBus Disable/Enable trick is UNRELIABLE — it may work once, then fail on subsequent attempts. When the extension enters INACTIVE state and DBus calls return `b true` but `State` remains `INACTIVE`, only a logout/login will recover it.

The DBus trick (try first, but don't count on it working twice):

```bash
busctl --user call org.gnome.Shell /org/gnome/Shell org.gnome.Shell.Extensions DisableExtension s "deskbrid@deskbrid"
sleep 1
busctl --user call org.gnome.Shell /org/gnome/Shell org.gnome.Shell.Extensions EnableExtension s "deskbrid@deskbrid"
sleep 2
gnome-extensions info deskbrid@deskbrid | grep State  # Should show ACTIVE, may show INACTIVE
```

**If State stays INACTIVE after multiple attempts — DO NOT GIVE UP. The version bump trick recovers it without logout:**

```bash
# GNOME Shell caches disabled state per version. Bump the version to force a fresh load.
cd ~/.local/share/gnome-shell/extensions/deskbrid@deskbrid
python3 -c "
import json
with open('metadata.json') as f:
    m = json.load(f)
m['version'] = m.get('version', 1) + 1
with open('metadata.json', 'w') as f:
    json.dump(m, f, indent=2)
print('Version bumped to', m['version'])
"
# Then do the DBus cycle
busctl --user call org.gnome.Shell /org/gnome/Shell org.gnome.Shell.Extensions DisableExtension s "deskbrid@deskbrid"
sleep 2
busctl --user call org.gnome.Shell /org/gnome/Shell org.gnome.Shell.Extensions EnableExtension s "deskbrid@deskbrid"
sleep 3
gnome-extensions info deskbrid@deskbrid | grep State  # Should show ACTIVE
```

**Why this works:** GNOME Shell 46 caches extension state per `(uuid, version)` tuple. After the 10-minute kill bug disables the extension, GNOME Shell marks that version as "error-prone" and refuses to reload it even when EnableExtension returns `b true`. Bumping the version number creates a new cache entry, bypassing the block.

**Only when version bump also fails:** logout/login is the final recovery path.

### Extension State Check

```bash
gnome-extensions info deskbrid@deskbrid | grep -E 'Enabled|State'
```

State values: `ACTIVE` (loaded, DBus available), `INACTIVE` (gsettings flag set but not loaded), `DISABLED` (user toggled off).

## Launching GUI Apps from Terminal

When launching GUI apps (VS Code, terminal, browser) from Hermes's terminal tool on Wayland, you MUST set these environment variables:

```bash
export DISPLAY=:0
export WAYLAND_DISPLAY=wayland-0
export DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/1000/bus"
export XDG_RUNTIME_DIR=/run/user/1000
export XAUTHORITY=/run/user/1000/.mutter-Xwaylandauth.XXXXXX  # <-- CRITICAL
```

The XAUTHORITY value is machine-specific (random suffix). To find it on GNOME Wayland:
```bash
ls /run/user/1000/.mutter-Xwaylandauth.*
```

**Without XAUTHORITY:** Electron/Chromium apps (VS Code, Discord, etc.) will crash with "Missing X server or $DISPLAY" and exit code 139 (SIGSEGV).

**Redaction pitfall:** Hermes's `redact_secrets` feature masks the XAUTHORITY path as if it were an API key because the random suffix looks token-like. If you see `/run/u...VO3` truncated, use `ls` directly to get the real path.

### Full launch template for GUI apps:
```bash
export DISPLAY=:0 DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/1000/bus" \
       WAYLAND_DISPLAY=wayland-0 XDG_RUNTIME_DIR=/run/user/1000 \
       XAUTHORITY=$(ls /run/user/1000/.mutter-Xwaylandauth.* 2>/dev/null | head -1) && \
  /usr/share/code/code --no-sandbox --new-window /path/to/project
```

```bash
# System
echo '{"type":"system.info","id":"1"}' | nc -U /run/user/1000/deskbrid.sock -w 2
echo '{"type":"system.idle","id":"2"}' | nc -U /run/user/1000/deskbrid.sock -w 2

# Windows
echo '{"type":"windows.list","id":"3"}' | nc -U /run/user/1000/deskbrid.sock -w 2
echo '{"type":"windows.focus","id":"4","window_id":"3"}' | nc -U /run/user/1000/deskbrid.sock -w 2

# Input
echo '{"type":"input.keyboard","id":"5","action":"type","text":"hello\n"}' | nc -U /run/user/1000/deskbrid.sock -w 2
echo '{"type":"input.keyboard","id":"6","action":"combo","keys":["ctrl","t"]}' | nc -U /run/user/1000/deskbrid.sock -w 2

# Clipboard
echo '{"type":"clipboard.read","id":"7"}' | nc -U /run/user/1000/deskbrid.sock -w 2

# Screenshot
echo '{"type":"screenshot","id":"8"}' | nc -U /run/user/1000/deskbrid.sock -w 5
```

## v0.2: Mutter RemoteDesktop Input Injection

**Status: Built and tested May 9, 2026.** Replaced wtype/ydotool with direct Mutter RemoteDesktop DBus calls through the compositor's real input pipeline. No virtual device permissions needed. No uinput. No ydotool daemon.

### Architecture

```
deskbrid daemon (zbus::Connection — persistent)
    ↓ CreateSession → Start
    ↓ org.gnome.Mutter.RemoteDesktop.Session
    ↓
NotifyKeyboardKeysym(u keysym, b state)   → real keystrokes through compositor
NotifyPointerButton(i button, b state)    → real mouse clicks
NotifyPointerMotionRelative(d dx, d dy)   → relative mouse motion
NotifyPointerAxisDiscrete(u axis, i steps) → scroll wheel
```

**No screen cast required** for keyboard, buttons, relative motion, or scroll. Only `NotifyPointerMotionAbsolute` needs a screen cast stream — use relative motion as fallback.

### Keysym Mapping

ASCII characters are mapped to XKB keysyms (e.g., 'a'→0x0061, 'A'→0x0061+Shift). See `references/mutter-remotedesktop-session-api.md` for the full mapping table. The `keysym` module in `src/backend/gnome.rs` handles:
- All printable ASCII (a-z, A-Z, 0-9, punctuation)
- Special keys (Return, Tab, Escape, arrows, F-keys)
- Modifier keys (Shift, Ctrl, Alt, Super)
- Shift-state tracking for uppercase and symbols

### Session Lifecycle

- Session created via `org.gnome.Mutter.RemoteDesktop.CreateSession` → returns object path like `/org/gnome/Mutter/RemoteDesktop/Session/u1`
- Must call `Start()` before any input methods work
- Session lives as long as the creating `zbus::Connection` — the daemon's persistent connection keeps it alive
- **"Session creation inhibited"** error means another remote desktop protocol (RDP) has the session — disable with `grdctl rdp disable`
- Only ONE session can exist at a time across all consumers (RDP, VNC, deskbrid)

### Pitfalls

- **`NotifyPointerMotionAbsolute` returns "No screen cast active":** Absolute positioning requires a screen cast stream (used by RDP/VNC clients). Deskbrid doesn't create one — use relative motion or the window-geometry+click pattern instead.
- **Keyboard OK ≠ input delivered:** The DBus call succeeding doesn't guarantee the keysym reached an application. Mutter may silently drop input if no remote desktop client is connected. **Test with visible effects** (typing text into a focused text field).
- **"Session creation inhibited":** RDP is likely enabled. Run `grdctl rdp disable` to free the session. Or enable it back after deskbrid is done.
- **One-session limit:** If gnome-remote-desktop (RDP/VNC) is running, deskbrid can't create its own session. They compete for the single Mutter RemoteDesktop slot.

## Pitfalls (System-Level)

### 🚨 NEVER restart systemd-logind with an active user session

`sudo systemctl restart systemd-logind` instantly destroys the user's desktop session. It kicks them to the login screen, breaks the display manager, and can leave the GPU in a bad state where SDDM won't restart cleanly. This is functionally equivalent to pulling the power cord.

**If you've already done this and the user is stuck at a black screen — reality is worse than theory:**

1. **Multiple `systemctl start sddm`/`systemctl restart sddm` calls pile up.** Each invocation blocks on a systemd transaction lock held by the previous (also-stuck) invocation. You'll see processes like `systemctl start sddm` in `ps aux` that never complete. Kill ALL of them:
   ```bash
   ps aux | grep 'systemctl.*sddm'  # check for piled-up processes
   sudo pkill -f 'systemctl.*sddm'  # kill them all at once
   sudo systemctl reset-failed sddm  # clear the failed state
   ```

2. **systemctl may hang even after killing.** `systemctl start sddm` can still block because systemd's service manager itself is waiting on something. The only reliable way to get SDDM back is to bypass systemd entirely and start it directly:
   ```bash
   sudo pkill -9 -x sddm
   sudo /usr/bin/sddm &  # starts on the next free VT, bypassing systemctl
   ```
   After this, the user may need to switch VTs (Ctrl+Alt+F3 usually — not F2, not F1).

3. **`pkill -9 -u jeremy` is unpredictable.** It kills the SSH session too, and may not break the DRM lock. Use it as a hail mary, not step 1.

4. **Check for stuck processes again after any kill attempt.** `ps aux | grep -E '[s]ddm|[s]ystemctl.*sddm'` — if any systemctl process is still S+ (sleeping, waiting on lock), kill it directly by PID.

5. **Check VT assignment.** `sudo fgconsole` tells you the active VT. `journalctl -u sddm --since '30 seconds ago'` tells you which VT SDDM started on. If they don't match, tell the user to switch VTs. It's common for SDDM to land on VT 3 while the user is stuck on VT 1.

6. **Ultimately, reboot is the only reliable recovery.** After the user has been kicked out, the GPU (especially Intel HD) can enter a state where atomic modesetting fails for any new display server. Once you've tried the above and the user is still staring at a black screen with a cursor:
   ```bash
   sudo reboot
   ```
   Don't waste 10 minutes trying to fix it incrementally — the GPU DRM state is hosed. Reboot and move on.

7. **After reboot**, KDE Wayland may fail with "Atomic modeset test failed! Permission denied" on Intel GPUs — see the GPU permissions fix below. On the next login, KDE will work.

### KDE Wayland — Intel HD Graphics "Atomic modeset test failed" (Black Screen)

On EndeavourOS/Arch with Intel HD 4000/4400/5000 GPUs, KDE Wayland can fail with:

```
Failed to find a working output layer configuration!
drmModeListLessees() failed: Permission denied
Atomic modeset test failed! Permission denied
```

**Fix: add user to the `video` group** (KDE Wayland needs DRM master capability):
```bash
sudo usermod -aG video jeremy
```
After adding to the group, the user needs to log out and back in for it to take effect. The session can then be restarted fresh from SDDM.

### Test Rig Sleep Prevention

EndeavourOS/Arch laptops default to suspending on lid close and idle timeout. For a headless test rig accessed via SSH, disable all sleep targets:

```bash
sudo systemctl mask sleep.target suspend.target hibernate.target hybrid-sleep.target
sudo sed -i 's/^#HandleLidSwitch=.*/HandleLidSwitch=ignore/' /etc/systemd/logind.conf
sudo sed -i 's/^#HandleLidSwitchExternalPower=.*/HandleLidSwitchExternalPower=ignore/' /etc/systemd/logind.conf
```

**⚠️ DO NOT restart systemd-logind** after changing HandleLidSwitch. It will kill the user's session (see the 🚨 pitfall above). Instead, have the user reboot the machine. The sleep.target mask takes effect immediately; the lid settings take effect on next boot without the logind restart catastrophe.

**KDE-specific:** KDE Plasma also has its own power management that can sleep the display or lock the screen even with sleep.target masked. The systemd logind masks stop the kernel from suspending, but KDE's powerdevil daemon can still turn off the display. To disable in KDE: System Settings → Power Management → Energy Saving → "When idle" set to "Do nothing", or from the CLI it's more involved. On a test rig running KDE, simply logging into a KDE session and disabling sleep in the UI is the pragmatic fix.

Also disable GNOME idle dimming and screensaver (if GNOME is also installed on the rig):
```bash
gsettings set org.gnome.desktop.session idle-delay 0
gsettings set org.gnome.desktop.screensaver idle-activation-enabled false
```

These are per-user gsettings and survive reboots. Apply them from within the user's session.

## Known Issues

### KDE (KWin) — ydotool WORKS (when ydotoold runs as user, not root)

**Contrary to earlier belief, ydotool DOES work on KDE** — the socket permission issue was the real blocker, not KWin blocking /dev/uinput. Verified working on KDE Plasma 6.6.4 (EndeavourOS): `ydotool type`, `ydotool key`, `ydotool click` all deliver input to the focused window.

**The trap:** If ydotoold was started with `sudo` (common when following Hyprland instructions), it creates a root-owned socket. The user process can't write to it:
```
$ ydotool type hello
failed to connect socket `/tmp/.ydotool_socket': Permission denied
$ ls -la /tmp/.ydotool_socket
srw------- 1 root root 0 ... /tmp/.ydotool_socket
```

**Fix:** Kill root ydotoold, start it as the user (who has `input` group access to /dev/uinput):
```bash
sudo pkill ydotoold
sudo rm -f /tmp/.ydotool_socket  # root-owned socket
ydotoold &  # runs as user, creates user-owned socket at $XDG_RUNTIME_DIR/.ydotool_socket
```

The socket created by user-run ydotoold lands at `$XDG_RUNTIME_DIR/.ydotool_socket` (typically `/run/user/1000/.ydotool_socket`), which ydotool finds automatically via the `$XDG_RUNTIME_DIR` env var — no `YDOTOOL_SOCKET` override needed.

**Auto-start on KDE login:**
```bash
mkdir -p ~/.config/autostart
cat > ~/.config/autostart/ydotoold.desktop << 'EOF'
[Desktop Entry]
Type=Application
Name=ydotoold
Exec=ydotoold
Terminal=false
NoDisplay=true
X-KDE-autostart-phase=2
EOF
```

### KDE — Screenshots Now Fixed (spectacle + ImageMagick)

**Problem:** KDE Plasma does NOT support the wlr-screencopy protocol that `grim` uses. On KDE:
```
Error: grim failed: compositor doesn't support the screen capture protocol
```

**Fix (v0.4.1):** Replaced `grim` with `spectacle -b -n -o <path>` for full-screen captures. For window/region screenshots: capture full screen via `spectacle`, then crop using ImageMagick `convert -crop <WxH+X+Y>`. Verified working on KDE Plasma 6.6.4:

| Type | Result | |
|------|--------|---|
| Full screen | ✅ 1366x768 correct PNG | |
| Window (Firefox 1366x722) | ✅ Cropped to 1366x722, bottom panel excluded | |
| Window (Console 652x456) | ✅ Cropped to correct offset (357,133) | |

**Dependencies:** `spectacle` (part of KDE Plasma, always available) + ImageMagick `convert`/`identify` (from `imagemagick` package on Arch/EndeavourOS).

**Implementation:** The KDE backend's `screenshot()` method now:
1. Window screenshots: calls `window_get()` for geometry → `spectacle` full screen → `convert -crop` with the geometry rect
2. Region screenshots: `spectacle` full screen → `convert -crop` with the region rect
3. Full screen: straight `spectacle -b -n -o <path>`
4. Dimensions extracted via ImageMagick `identify -format "%w %h"`

**Note:** `spectacle --version` crashes with core dump when run headlessly (tries to init GUI), but `spectacle -b -n` works fine in captured sessions — the crash is a `--version`-specific quirk.

### Versioning: Match the Release Train, not Cargo.toml

**Never jump versions because Cargo.toml says something different.** The Cargo.toml `version` field may drift from the actual release history. Always check `git tag --list 'v*' | sort -V` to see the last real release tag before choosing the next version. If the last tag was `v0.4.0` and Cargo.toml says `2.0.0`, the next tag should be `v0.4.1` (bugfix) or `v0.5.0` (new feature) — not `v2.0.0`.

Jeremy will correct you if you get this wrong. Trust the tag history, not the TOML.

### CI Trap: dead_code Warnings Become Errors with `-D warnings`

The CI runs with `RUSTFLAGS: -D warnings`, which means ANY warning (including `dead_code`) becomes a compile error. The most common pre-CI failure: an unused method or import.

**Fix before pushing:** Run the full CI pipeline locally:

```bash
RUSTFLAGS="-D warnings" cargo check && \
  cargo clippy --fix --allow-dirty && \
  cargo fmt && \
  cargo test
```

Using plain `cargo check` won't catch dead_code — it only prints a warning. CI will fail on push. Always prepend the flag.

### Binary Redeployment Workaround

When `scp` fails with "dest open: Failure" because the daemon is running and holding the old binary open, use the temp-filename trick instead of killing everything:

```bash
scp target/release/deskbrid user@host:~/deskbrid_new
ssh user@host "mv ~/deskbrid_new ~/deskbrid && chmod +x ~/deskbrid"
```

The `mv` atomically replaces the inode — the running daemon keeps its open handle to the old inode, but the new `scp` writes to a fresh one. Only kill the daemon if you need the new binary to take effect.

### deskbrid setup command (v0.4.1+)

`deskbrid setup` is a one-command setup that auto-detects the desktop environment and installs/configures what's needed:

- **GNOME**: Installs the embedded Shell extension (`extension.js` + `metadata.json` baked into the binary via `include_str!()`) to `~/.local/share/gnome-shell/extensions/deskbrid@deskbrid/` and enables it
- **Hyprland**: Prints ydotool setup tips (udev rules, `exec-once = ydotoold`)
- **KDE**: Prints ydotoold autostart tips (XDG autostart .desktop file)

The extension files are embedded at compile time in `src/setup.rs`:
```rust
pub const GNOME_EXTENSION_METADATA: &str = include_str!("../extensions/deskbrid@deskbrid/metadata.json");
pub const GNOME_EXTENSION_JS: &str = include_str!("../extensions/deskbrid@deskbrid/extension.js");
```

**Never manually extract extension files** — use `deskbrid setup` or the release zip asset.

### Release Quality Gate

**Do a full sweep before saying "done."** Jeremy will catch every missed item — docs not updated, stale files left in `~/projects/`, the Hermes project skill (`hermes/deskbrid.md`) not updated, EIS research artifacts not cleaned up. Before declaring a release complete:
1. `find ~/projects -name '*eis*' -o -name '*libei*'` — nuke stale research artifacts
2. Check `hermes/deskbrid.md` — update for any new backends/commands
3. Sweep ALL `.md` files for stale references (grepping for "planned", "not yet", old version strings)
4. **Check ALL `.md` files for pre-existing formatting rot before telling Jeremy it's done.** Line number prefixes (`   N|   N|` at start of every line) are invisible during dev but break markdown rendering. Use: `grep -cnP '^\s*\d+\|' docs/*.md` — any hits > 0 means the file needs a strip pass before it'll render properly. If you find rot, strip it before the release, not after Jeremy points it out.
5. Check that the git tag version matches the release train  
6. Verify CI passes on the actual commit (not just local `cargo check`)

**Always test ALL active backends before cutting a release.**

```
turtle KDE:   focus → type → screenshot → workspace switch → list windows
turtle Hyprland: focus → type → screenshot → workspace switch → list windows
coemedia GNOME: focus → type → screenshot → workspace switch → list windows
```

If a backend has a known issue (like a detection failure on a specific distro), the release notes MUST call it out explicitly. Do NOT ship a broken backend silently.

The extension was silently disabled ~10 minutes after enable with no error log. Root cause: GJS garbage collection sweeping the Extension instance because no GC root held a reference.

**Fix:** Add `_extensionInstance = this` in `enable()` and `_extensionInstance = null` in `disable()`. Also add `&& _dbusImpl` null guard in signal handler.

## Workspace Operations (v0.2.1+)

Workspace switch and move-window use the GNOME Shell extension's DBus methods — NOT `Eval`. On GNOME 46, `org.gnome.Shell.Eval` is locked down (returns `b false` silently). The extension exposes:

| Operation | Extension DBus Method | Args |
|-----------|----------------------|------|
| Switch workspace | `SwitchWorkspace` | `u index` |
| Move window | `MoveWindowToWorkspace` | `s app_id`, `u workspace_index` |

The Rust daemon calls these via `ext_call_parsed()` — a thin wrapper around `gdbus call`:

```rust
// Workspace switch
self.ext_call_parsed("SwitchWorkspace", &[&id.to_string()]).await?;

// Move window to workspace
self.ext_call_parsed("MoveWindowToWorkspace", &[&app_id, &workspace_id.to_string()]).await?;
```

**No Eval.** No fallback. If the extension isn't active, these fail — fix the extension, not the call site.

### Why Eval Died

GNOME 46+ restricts `org.gnome.Shell.Eval` to trusted callers. Even when `gsettings` says extensions are enabled, `Eval("javascript_code")` returns `(false, '')` — no error, no output. The extension DBus interface side-steps this entirely because it registers its own methods on the session bus.

## Demo Recording — Keyboard-Only Approach

When recording a deskbrid demo (for README GIF, YouTube, or social):

**DO NOT use the mouse.** On GNOME 46, absolute mouse positioning requires a ScreenCast session which is flaky (`RecordMonitor` returns "Unknown monitor," `RecordVirtual` returns no active stream). Relative mouse motion drifts after daemon restarts because the tracked position resets to (960,540).

**Use this keyboard-only sequence instead:**

```bash
# 1. Focus the target window by title substring (v0.2.1+ fix ensures correct window)
echo '{"type":"windows.focus","window_id":"MyProject","id":"focus","seq":1}' | nc -U /run/user/1000/deskbrid.sock -w 3

# 2. Wait for window to come to front
sleep 1

# 3. Type text (compositor-native, no virtual devices)
echo '{"type":"input.keyboard","action":"type","text":"your command here\n","id":"type","seq":1}' | nc -U /run/user/1000/deskbrid.sock -w 3

# 4. Maximize with keyboard shortcut (optional, for visual pop)
echo '{"type":"input.keyboard","action":"combo","keys":["Super","Up"],"id":"combo","seq":1}' | nc -U /run/user/1000/deskbrid.sock -w 3
```

**Why this works:** `windows.focus` uses the extension's DBus `FocusWindow` method (case-insensitive substring matching on title + app_id). Keyboard input goes through Mutter's `NotifyKeyboardKeysym` — no ScreenCast needed, no uinput permissions, no virtual devices. Window maximization uses standard GNOME shortcuts through the same compositor pipeline.

**For maximal demo impact:** Open VS Code with a project loaded, have the agent type a meaningful command (not "hello world"), and optionally maximize the window. The focus → type → maximize sequence takes ~5 seconds and demonstrates all three compositor-native capabilities without touching the mouse.

**Two separate `nc` invocations, not `&&` chaining.** `&&` works but the second `nc` opens a fresh connection with its own `seq` counter. It's cleaner to run them as separate commands.

See `references/demo-gif-production.md` for converting screen recordings to README-friendly GIFs.

Deskbrid migrated from Rust edition 2021 → 2024 (stable since 1.85). Changes required:

1. **`std::env::set_var` now unsafe** — can cause UB in multi-threaded contexts. Wrap in `unsafe { }` with a `// SAFETY:` comment.
2. **`collapsible_if` is deny-by-default** — nested `if let ... { if ... { } }` must collapse to `if let ... && ... { }` using let-chains (edition 2024 feature).
3. **`cargo fmt` rules changed** — `use` groups sort differently, `unsafe` blocks get their own indent. Run `cargo fmt` after bumping edition.

CI must include `cargo fmt --check` — edition 2024 formatting differs from 2021.

**Gotcha:** Hermes's patch tool lint will hallucinate `async fn not permitted in Rust 2015` errors on edition 2024 code. Ignore those — always verify with actual `cargo check`/`cargo clippy`. The patch tool doesn't parse `edition = "2024"` from Cargo.toml.

## Release Workflow

**🚨 CRITICAL: Check the existing tag/release scheme before tagging.** Cargo.toml may have been bumped to a version that doesn't match the release train (e.g., Cargo.toml says 2.0.0 but existing releases are v0.4.x). Always check `git tag --list 'v*' | sort -V` and `gh release list -L 5` before deciding the next version. The user expects semantic versioning — patch for bugfixes, minor for features, major only for real breaking changes. A version jump from v0.4.0 to v2.0.0 will be rejected.

**Merge to `main` before any release action.**** Feature branches do NOT ship. The sequence is: feature branch → merge to main → delete branch → tag/release. If you skip the merge and push tags from the branch, `main` stays behind and the "release" points to orphaned commits.

See `references/permissions-system.md` for the scoped per-UID permissions system (v0.5.0).
See `references/release-workflow.md` for the complete release checklist: pre-flight checks, CI gates, version bumping, docs audit, binary build, GitHub release creation, and verification. Covers the `git branch`/`tag` name collision pitfall (commits on `v0.2.0` branch don't reach `main`) and fixing stale GitHub releases.

### Release Assets

**Always include TWO assets in every GitHub release:**
1. **Compiled binary** (`deskbrid`) — `target/release/deskbrid`
2. **GNOME extension zip** (`deskbrid-gnome-extension-v<VERSION>.zip`) — zip of `extensions/deskbrid@deskbrid/` files

The extension is also embedded in the binary via `src/setup.rs` (`include_str!()`), but the zip lets manual installers grab it without building from source.

```bash
mkdir -p dist
zip -j dist/deskbrid-gnome-extension-v0.4.1.zip \
  extensions/deskbrid@deskbrid/metadata.json \
  extensions/deskbrid@deskbrid/extension.js

gh release create v0.4.1 \
  --title "v0.4.1 — Title" \
  --notes "Release notes..." \
  target/release/deskbrid \
  dist/deskbrid-gnome-extension-v0.4.1.zip
```

### CI Pipeline Order

The CI runs: `cargo check` → `cargo clippy -- -D warnings` → `cargo fmt --check` → `cargo test`. DO NOT push until ALL pass locally:

```bash
RUSTFLAGS="-D warnings" cargo check && \
  cargo clippy --fix --allow-dirty && \
  cargo fmt && \
  cargo test
```

**Trap:** `cargo clippy --fix` scrambles formatting. Always run `cargo fmt` immediately after. Pushing without formatting will fail the `fmt --check` CI step.

**Second trap:** plain `cargo check` doesn't fail on `dead_code` — it only prints a warning. The CI uses `RUSTFLAGS: -D warnings` which turns ALL warnings into errors. Always use `RUSTFLAGS="-D warnings" cargo check` locally to catch what CI will catch.

**Third trap — clippy --fix on new files:** If you created a NEW file (like `src/permissions.rs`) in the current session, `clippy --fix` can't fix it if the file isn't committed yet. Run `git add src/new_file.rs` BEFORE `cargo clippy --fix --allow-dirty`, or the fix won't persist through the commit. The `--allow-dirty` flag lets clippy modify staged files, but unstaged new files are invisible to it. Sequence: `git add -A && cargo clippy --fix --allow-dirty && cargo fmt && cargo test`.

**Fourth trap — CI Rust version may be newer than local:** The CI runner (`actions-rust-lang/setup-rust-toolchain@v1`) may pull a newer stable Rust than what's installed locally. A lint like `clippy::unnecessary_sort_by` that fires on Rust 1.95+ in CI may not fire on your local 1.92. Both run `cargo clippy -- -D warnings`, but the lints themselves evolve across Rust versions. If the CI fails but local passes with the same flags, the fix is likely still valid — apply it and push. Do NOT waste time trying to reproduce locally.

**Fifth trap — after fixing a CI failure, commit and push immediately.** Don't present the fix and wait for Jeremy to say "push." He expects you to own the full fix cycle: diagnose → reproduce → fix → verify → commit → push. If you verified the fix passes the exact CI flags and just stopped, you're not done. The next message from Jeremy will be "commit and push dick" and you'll deserve it.

## Daemon Management — Starting from Hermes Terminal Context

When starting the deskbrid daemon from Hermes's `terminal` tool (NOT from a GNOME desktop terminal), the daemon needs `DBUS_SESSION_BUS_ADDRESS` to find the GNOME Shell session bus. Without it, `zbus::Connection::session()` connects to a different (headless) bus, and the GNOME backend won't load:

```bash
# Get the GNOME session's DBus address from the running gnome-shell process
DBUS_SESSION_BUS_ADDRESS=$(tr '\0' '\n' < /proc/$(pgrep -x gnome-shell | head -1)/environ | grep DBUS_SESSION_BUS_ADDRESS | cut -d= -f2-)

# Start daemon with the right session
DBUS_SESSION_BUS_ADDRESS=$DBUS_SESSION_BUS_ADDRESS ./target/release/deskbrid daemon
```

**Common address:** `unix:path=/run/user/1000/bus`

**No-deskbrid-binary pitfall:** Hermes's terminal is hermetic — it may not have `deskbrid` on PATH even if it's installed on the host. Use the full path (`/home/coemedia/projects/deskbrid/target/release/deskbrid`).

**Don't kill the GNOME-session daemon.** If the user started the daemon from their GNOME terminal, it already has the right DBus session. Killing and restarting from Hermes without the env var breaks it. Check if the socket is already live with the right session before killing:

```bash
# Test if existing daemon has the right session
echo '{"type":"system.info","id":"1"}' | nc -U /run/user/1000/deskbrid.sock -w 2 | python3 -m json.tool
# If it returns "no backend loaded" — the daemon needs restarting with DBUS_SESSION_BUS_ADDRESS
# If it returns system info — it's fine, leave it alone
```

## Daemon Management

```bash
# Check status
systemctl --user status deskbrid
pgrep -la deskbrid

# Restart
systemctl --user restart deskbrid

# Manual run (for debugging)
~/projects/deskbrid/target/release/deskbrid daemon
```

## Pitfalls

- **`org.gnome.Shell.Eval` is dead on GNOME 46.** It returns `(false, '')` silently even when extensions are enabled. Do not use Eval for anything. All workspace operations MUST go through the extension's DBus interface (`SwitchWorkspace`, `MoveWindowToWorkspace`). If the extension is inactive, fix that — don't write an Eval fallback.
- **Wayland `Alt+F2` → `r` doesn't work.** GNOME Shell restart is X11-only. See `references/gnome46-extension-lifecycle.md` for the full extension lifecycle guide, debug commands, and the 10-minute GJS GC kill bug investigation.
- **Linux agentic control ≠ macOS parity.** For the full analysis of what's achievable and what's permanently out of reach, see `references/linux-macos-agent-parity.md`. The TL;DR: VNC/grdctl is the only reliable input path; ydotool/wtype are dead ends for window manipulation.
- **Extension file must be at `~/.local/share/gnome-shell/extensions/deskbrid@deskbrid/`** — repo copy at `extensions/deskbrid@deskbrid/` must be synced.
- **socket path:** `$XDG_RUNTIME_DIR/deskbrid.sock` (typically `/run/user/1000/deskbrid.sock`), NOT `~/.local/share/deskbrid/deskbrid.sock`.
- **nc timeout:** Always use `-w 2` or higher — some actions (screenshot) take longer.
- **v0.2+ does NOT use wtype/ydotool.** Input now goes through Mutter RemoteDesktop compositor pipeline (see v0.2 section above). If you're on an older build that still uses wtype/ydotool: keyboard is blocked by Mutter, mouse clicks don't guarantee window raise. **Upgrade to v0.2.**
- **Mouse uses relative motion, not absolute.** `NotifyPointerMotionAbsolute` requires a ScreenCast stream. Relative motion with position tracking via `last_mouse: Mutex<(f64,f64)>` is the workaround. First move from the default center (960,540) may be imprecise; subsequent moves are accurate. **Critical calibration issue:** after daemon restart, the tracked position resets to (960,540) regardless of actual cursor position. There is no `reset_position` protocol action — the only way to recalibrate is to move the mouse to a known screen position and restart the daemon. For precise window targeting, prefer keyboard shortcuts (Super+Up, Alt+F10) over mouse clicks.
- **`zvariant::Value` can't deserialize complex DBus types.** Mutter DisplayConfig's `GetCurrentState` returns deeply nested structures that fail with `SignatureMismatch`. Use `busctl` or `gdbus` for probing, not zbus Value deserialization.
- **`reply.body().deserialize()` lifetime:** The body borrows from the reply message. Hold in a local: `let reply_body = reply.body(); let val = reply_body.deserialize()?;`
- **`Window focus failure when multiple windows of same app are open:** `windows.focus` used to locally match the FIRST window by XID order, then pass that window's app_id+title to the extension — resulting in a second find-first that agreed on the wrong window. 

**Fix (v0.2.1):** The daemon now passes the user's target string directly to the extension's `FocusWindow(app_id, title, exact)` method for non-XID targets. The extension already does case-insensitive substring matching on both app_id AND title, so `deskbrid windows focus kinsafe` finds the window with \"kinsafe\" in its title regardless of XID order. XID targets (starting with `0x`) still use the local-lookup-then-pass path.

**Before fix:** With 5 VS Code windows, `focus kinsafe` picked whichever Code window appeared first in the window list. **After fix:** It finds the one with \"kinsafe\" in the title.
- **GNOME 46 Wayland has no semantic window control.** Unlike macOS (AppleScript + Accessibility API), Linux provides no reliable way to say "maximize window 3 of VS Code." AT-SPI exists for accessibility but is sparse for Electron apps. VNC + deskbrid geometry reads + keyboard shortcuts is the practical ceiling. See `references/linux-macos-agent-parity.md` for the full analysis.
- **DBus Disable/Enable trick is unreliable.** It may work once per GNOME Shell session cache. When it fails (State stays INACTIVE despite `b true` returns), use the version bump trick (see Reloading section) instead of resorting to logout/login. The version bump bypasses the cached disabled-state block.
- **Extension shell-version metadata can lie.** After a backend rewrite (e.g., Mutter RemoteDesktop), the extension may still claim support for GNOME 45 in `metadata.json` when it only works on 46+. Bump `shell-version` to match actual support — this is a docs audit item on every release (see `references/release-workflow.md`).
- **GUI app launch from Hermes terminal requires XAUTHORITY.** On GNOME Wayland, `DISPLAY=:0` alone is insufficient. Electron/Chromium apps (VS Code, Chrome) fail with SIGSEGV "Missing X server or $DISPLAY." See `references/gui-app-launch.md` for the full environment block, XAUTHORITY discovery methods, and background process patterns.
