# Installation

Deskbrid is a single static binary with auto-detecting backends for multiple desktop environments.

## Pre-built Binary

Download the latest release from the [GitHub releases page](https://github.com/coe0718/deskbrid/releases):

```bash
# Pick your version — replace v0.6.0 with the latest tag
curl -LO https://github.com/coe0718/deskbrid/releases/download/v0.6.0/deskbrid
chmod +x deskbrid
sudo mv deskbrid /usr/local/bin/
```

## Build from Source

```bash
git clone https://github.com/coe0718/deskbrid
cd deskbrid
cargo build --release
sudo cp target/release/deskbrid /usr/local/bin/
```

## Desktop-Specific Setup

### GNOME

```bash
# System dependencies
sudo apt install -y grim wl-clipboard

# Install GNOME Shell extension
deskbrid setup

# Or manually:
cp -r extensions/deskbrid@deskbrid ~/.local/share/gnome-shell/extensions/
gnome-extensions enable deskbrid@deskbrid
```

If `gnome-extensions enable` fails on newer GNOME versions:

```bash
# Check your GNOME version
gnome-shell --version

# If your version isn't in metadata.json, either:
# 1. Update shell-version in metadata.json, or
# 2. Disable version validation temporarily:
gsettings set org.gnome.shell disable-extension-version-validation "true"
```

### Hyprland (and other standalone Wayland compositors)

```bash
# System dependencies (Arch)
sudo pacman -S grim wl-clipboard ydotool

# System dependencies (Debian - ydotool may need to be built from source)
sudo apt install grim wl-clipboard

# Fix /dev/uinput permissions (ydotool needs write access)
echo 'KERNEL=="uinput", GROUP="input", MODE="0660"' | sudo tee /etc/udev/rules.d/99-input.rules
sudo udevadm control --reload-rules
sudo chmod 0660 /dev/uinput && sudo chgrp input /dev/uinput
sudo usermod -aG input $USER  # log out and back in
```

Add to your Hyprland config (`~/.config/hypr/hyprland.conf`):

```
exec-once = ydotoold
```

> ⚠️ **Notification daemon required.** Standalone Wayland compositors (Hyprland, Sway, Niri, Wayfire, Labwc, COSMIC) don't ship a notification daemon. Deskbrid's `notify send` will time out after 30 seconds without one. Install one of:
> - **dunst** — lightweight, highly configurable (`sudo pacman -S dunst`)
> - **mako** — minimal, Wayland-native (`sudo pacman -S mako`)
> - **swaync** — Sway-style notification center (`sudo pacman -S swaync`)
>
> Then add it to your compositor's autostart (e.g. `exec-once = dunst` for Hyprland). Deskbrid's `system.health` check will warn if no notification daemon is detected.

### KDE Plasma

```bash
# System dependencies (Debian)
sudo apt install spectacle imagemagick wl-clipboard ydotool

# System dependencies (Arch)
sudo pacman -S spectacle imagemagick wl-clipboard ydotool
```

Add ydotoold to KDE autostart (`~/.config/autostart/ydotoold.desktop`):

```ini
[Desktop Entry]
Type=Application
Name=ydotoold
Exec=ydotoold
Terminal=false
X-KDE-autostart-phase=2
```

Fix `/dev/uinput` permissions (same as Hyprland):

```bash
echo 'KERNEL=="uinput", GROUP="input", MODE="0660"' | sudo tee /etc/udev/rules.d/99-input.rules
sudo udevadm control --reload-rules
sudo chmod 0660 /dev/uinput && sudo chgrp input /dev/uinput
sudo usermod -aG input $USER  # log out and back in
```

### X11 (Cinnamon, MATE, XFCE)

```bash
# System dependencies
sudo apt install xdotool wmctrl xclip scrot  # Debian
sudo pacman -S xdotool wmctrl xclip scrot    # Arch
```

## Starting the Daemon

```bash
# Start in background
deskbrid daemon &

# Or with verbose logging
deskbrid daemon --verbose &

# Or with MCP server on TCP port
deskbrid daemon --mcp-port 18796 &
```

## Verify Installation

```bash
# Check daemon status
deskbrid status

# List windows
deskbrid windows list

# Get system info
deskbrid system info
```

## MCP Server (for AI coding tools)

```bash
# Build with MCP support included by default
cargo build --release

# Start MCP server on stdio
deskbrid mcp
```

Add to your AI coding tool's MCP config:

```json
{
  "mcpServers": {
    "deskbrid": {
      "command": "deskbrid",
      "args": ["mcp"]
    }
  }
}
```

### MCP Configuration Examples

**Claude Desktop** (`~/.config/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "deskbrid": {
      "command": "/usr/local/bin/deskbrid",
      "args": ["mcp"],
      "env": {
        "PATH": "/usr/local/bin:/usr/bin:/bin"
      }
    }
  }
}
```

**Cursor** (`.cursor/mcp.json`):

```json
{
  "mcpServers": {
    "deskbrid": {
      "command": "deskbrid",
      "args": ["mcp"]
    }
  }
}
```