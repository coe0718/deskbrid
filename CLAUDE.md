# Deskbrid — Desktop Control Daemon

Deskbrid bridges AI agents to any Linux desktop over a Unix socket. Control windows, inject keystrokes, take screenshots, manage clipboards — on GNOME, KDE, Hyprland, or X11.

## Install on Any Machine

The one-shot install script auto-detects the desktop environment and installs everything:

```bash
bash <(curl -fsSL https://deskbrid.patchhive.dev/install.sh)
```

Or directly from the repo:
```bash
bash <(curl -fsSL https://raw.githubusercontent.com/coe0718/deskbrid/main/site/install.sh)
```

**What the script does:**
1. Detects distro (apt/pacman/dnf/zypper/apk) and desktop (GNOME/KDE/Hyprland/X11)
2. Installs missing dependencies — grim, wl-clipboard, ydotool, xdotool, etc. per DE
3. Sets up /dev/uinput permissions for Wayland (udev rule + input group)
4. Downloads the latest binary to /usr/local/bin/deskbrid
5. Configures ydotoold autostart (Hyprland/KDE)
6. Prints next steps

## Quick Manual Install

```bash
# Deps for your DE
#   GNOME:    grim wl-clipboard
#   Hyprland: grim wl-clipboard ydotool
#   KDE:      spectacle imagemagick ydotool qt6-tools
#   X11:      xdotool wmctrl xclip imagemagick

# Download binary
curl -LO https://github.com/coe0718/deskbrid/releases/download/v0.6.0/deskbrid
chmod +x deskbrid
sudo mv deskbrid /usr/local/bin/

# Start daemon
deskbrid daemon
```

## After Install

```bash
# Check health
deskbrid health

# Test the socket
echo '{"type":"system.info","id":"1"}' | nc -U /run/user/1000/deskbrid.sock -w 2

# Full docs
open https://deskbrid.patchhive.dev
```

## Project Structure

- `src/` — Rust source (gnome.rs, hyprland.rs, kde.rs, x11.rs backends)
- `site/` — landing page + install.sh
- `extensions/` — GNOME Shell extension
- `hermes/` — Hermes agent tool config
