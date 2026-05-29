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

### Audio Control (#54) — `src/daemon/audio.rs`
- Volume get/set via `wpctl`
- Mute/unmute via `wpctl`
- Sink/source listing via `pactl`
- **Test on:** Any system with PipeWire + WirePlumber (`pipewire-pulse`)

### Self-Update (#19) — `src/daemon/update.rs`
- GitHub release binary download and replacement
- Check-only mode (`deskbrid update --check`)
- Binary self-replacement via temp file + rename
- **Test on:** Any Linux x86_64 with internet access

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
./target/release/deskbrid audio volume get
./target/release/deskbrid audio volume set --level 50
./target/release/deskbrid audio mute
./target/release/deskbrid audio list-sinks

# Self-update
./target/release/deskbrid update --check
```
