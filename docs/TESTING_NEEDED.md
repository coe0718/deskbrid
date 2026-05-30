# Testing Required

The following features were implemented but **require manual testing on a live desktop environment** before they can be considered production-ready.

## Untested Features

### Screen Recording (#17) — `src/backend/gnome/screenshot.rs`
- Screencast start/stop via Mutter ScreenCast D-Bus session
- PipeWire stream recording via `gst-launch-1.0`
- `ScreencastFrame` and `ScreencastStopped` events
- **Test on:** GNOME (Wayland), any DE with Mutter compositor

### Desktop Portal Integration (#10) — `src/daemon/portal.rs`
- XDG Screenshot portal via `org.freedesktop.portal.Screenshot`
- Portal request/response signal handling (30s timeout)
- ScreenCast portal stub (requires PipeWire stream handling for full support)
- **Test on:** Any Wayland DE with `xdg-desktop-portal` installed

### Audio Control (#54) — `src/daemon/execute_audio.rs`
- Volume get/set via `wpctl`/`pactl`
- Mute/unmute via `pactl`
- Sink/source listing and default sink/source via `pactl`
- **Test on:** Any system with PipeWire + WirePlumber (`pipewire-pulse`)

### Self-Update (#19 / roadmap #125) — `src/cmd/update/`
- GitHub release binary download and replacement
- Check-only mode (`deskbrid update --check`)
- Binary backup to `deskbrid.old`, install replacement, restart `deskbrid.service` if active
- **Test on:** Any Linux x86_64/aarch64 with internet access and an actual newer GitHub release

### D-Bus Raw Access (#28) — `src/daemon/execute_system.rs`
- Raw D-Bus method calls via `dbus-send` subprocess
- Session and system bus support
- JSON args parsing
- **Test on:** Any system with D-Bus (run `deskbrid dbus-call --service org.freedesktop.DBus --path /org/freedesktop/DBus --interface org.freedesktop.DBus --method ListNames`)

### Cron / Scheduled Actions (#27) — `src/daemon/schedule.rs`
- Schedule file at `~/.config/deskbrid/schedule.json`
- `deskbrid schedule list|add|remove` CLI
- 60-second polling daemon task executes scheduled actions
- **Test on:** Running daemon (add a schedule entry, wait 60s, check audit log)

### TCP Mode (#30) — `src/daemon/tcp.rs`
- TCP listener with bearer token auth
- Auto-generated token (logged at INFO level) or explicit `--tcp-token`
- Rust client via `DESKBRID_HOST`/`DESKBRID_PORT`/`DESKBRID_TCP_TOKEN` env vars
- Python client via `tcp_host`/`tcp_port`/`tcp_token` kwargs or same env vars
- Synthetic UID `0xFFFF_FFFE` for permission scoping
- **Test on:** Any machine (start daemon with `--tcp-port 127.0.0.1:7890`, connect with env vars)

## How to Test

```bash
# Build
cargo build --release

# Screen recording
./target/release/deskbrid screencast start --output /tmp/test.mkv
./target/release/deskbrid screencast stop

# Portal screenshot
./target/release/deskbrid portal screenshot --interactive

# Audio
./target/release/deskbrid audio sinks
./target/release/deskbrid audio sources
./target/release/deskbrid audio get-volume sink 0
./target/release/deskbrid audio set-volume sink 0 0.50
./target/release/deskbrid audio mute sink 0 true
./target/release/deskbrid audio set-default sink alsa_output.example

# Self-update
./target/release/deskbrid update --check

# TCP Mode
./target/release/deskbrid daemon --tcp-port 127.0.0.1:7890 &
DESKBRID_PORT=7890 DESKBRID_TCP_TOKEN=<token-from-logs> ./target/release/deskbrid status
```

### Action Recording & Replay (#25) — `src/daemon/macro_engine.rs`
- `deskbrid macro record <name>` — starts recording all dispatched actions
- `deskbrid macro stop` — saves to `~/.local/share/deskbrid/macros/<name>.json`
- `deskbrid macro replay <name>` — executes saved sequence
- Modes: fast (no delays), timed (preserved timing)
- Loop count and stop_on_error enforcement
- **Test on:** Running daemon (record a few actions, stop, replay, list, export, import)

```bash
# Recording
./target/release/deskbrid macro record test-macro
./target/release/deskbrid system info
./target/release/deskbrid windows list
./target/release/deskbrid macro stop

# Replay
./target/release/deskbrid macro replay test-macro
./target/release/deskbrid macro replay test-macro --mode timed --loop-count 3

# CRUD
./target/release/deskbrid macro list
./target/release/deskbrid macro get test-macro
./target/release/deskbrid macro export test-macro > /tmp/macro-export.json
./target/release/deskbrid macro import test-macro-2 "$(cat /tmp/macro-export.json)"
./target/release/deskbrid macro delete test-macro-2
```
