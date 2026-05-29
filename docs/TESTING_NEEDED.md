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
```
