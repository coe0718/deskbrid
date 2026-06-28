# Installation

## System Requirements

- Linux (Wayland or X11)
- Rust edition 2024 toolchain (for building from source)
- A supported desktop environment (see README desktop table)

## One-liner Install (recommended)

```bash
bash <(curl -fsSL https://deskbrid.patchhive.dev/install.sh)
```

The installer detects your distro and desktop, installs dependencies, configures `uinput`, and installs the binary to `/usr/local/bin`.

## Manual Install

1. Download the latest release tarball:
```bash
ARCH=$(uname -m)
case "$ARCH" in x86_64) ARCH="x86_64-unknown-linux-gnu";; aarch64|arm64) ARCH="aarch64-unknown-linux-gnu";; esac
curl -LO "https://github.com/coe0718/deskbrid/releases/latest/download/deskbrid-${ARCH}.tar.gz"
curl -LO "https://github.com/coe0718/deskbrid/releases/latest/download/deskbrid-${ARCH}.tar.gz.sha256"
sha256sum -c "deskbrid-${ARCH}.tar.gz.sha256"
tar -xzf "deskbrid-${ARCH}.tar.gz"
sudo mv deskbrid /usr/local/bin/
chmod +x /usr/local/bin/deskbrid
```

2. Install desktop-specific dependencies.

3. Configure permissions:
```bash
deskbrid setup
```

## Desktop-Specific Dependencies

### GNOME
```bash
sudo apt install -y grim wl-clipboard python3-gi gstreamer1.0-tools gstreamer1.0-pipewire
deskbrid setup
```

### Hyprland / Standalone Wayland (Sway, Niri, Wayfire, Labwc)
```bash
sudo pacman -S grim wl-clipboard ydotool
echo 'KERNEL=="uinput", GROUP="input", MODE="0660"' | sudo tee /etc/udev/rules.d/99-input.rules
sudo usermod -aG input $USER
```

> ⚠️ Standalone Wayland compositors don't ship a notification daemon by default.
> Install **dunst**, **mako**, or **swaync** and autostart it, otherwise
> `notification.send` may fail or hang.

### KDE Plasma
```bash
sudo apt install spectacle imagemagick wl-clipboard ydotool
```

### X11 (Cinnamon, MATE, XFCE)
```bash
sudo apt install xdotool wmctrl xclip imagemagick  # Debian/Ubuntu
# or: sudo pacman -S xdotool wmctrl xclip imagemagick  # Arch
```

## Post-Install Checklist

1. Log out and back in (required for group changes).
2. Verify the daemon socket path:
```bash
ls -l /run/user/$(id -u)/deskbrid.sock
```
3. Start the daemon:
```bash
deskbrid daemon
```
4. Run a smoke test:
```bash
deskbrid system.info
deskbrid windows.list
```

## Updating

```bash
# Re-run the installer to update
bash <(curl -fsSL https://deskbrid.patchhive.dev/install.sh)
```

Or manually replace the binary with the new release and re-run
`deskbrid setup` if your desktop dependencies changed.

## Web Dashboard

Deskbrid ships a built-in web dashboard at **http://localhost:20129**.
It exposes system info, monitors, windows, network, audio, clipboard history,
and a live audit log of agent actions.

## Automatic Updates

Deskbrid can check for updates on the configured channel. To change the
default update channel or disable auto-update, see the `self_update`
action documentation in [`../API.md`](../API.md).

## Uninstall

```bash
# Remove binary
sudo rm /usr/local/bin/deskbrid

# Remove runtime files
rm -rf ~/.local/share/deskbrid

# Remove config files (optional, destructive)
rm -rf ~/.config/deskbrid
```
