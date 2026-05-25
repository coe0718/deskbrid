# Dependencies

Deskbrid auto-detects your desktop environment and loads the matching backend. Dependencies vary by backend — install only what your compositor needs.

## GNOME

| Dependency | Package | Purpose |
|---|---|---|
| GNOME Shell Extension | `extensions/deskbrid@deskbrid/` (in-repo) | Window listing, focus, workspace control |
| `grim` | `grim` | Screenshot fast path where available |
| `gst-launch-1.0` | `gstreamer1.0-tools` / `gstreamer` | Capture frames from Mutter PipeWire streams |
| `pipewiresrc` | `gstreamer1.0-pipewire` / `gst-plugin-pipewire` | GStreamer PipeWire source plugin |
| Python GI | `python3-gi` / `python-gobject` | XDG Desktop Portal screenshot fallback |
| `wl-paste` / `wl-copy` | `wl-clipboard` | Clipboard read/write |
| `ydotool` | `ydotool` | Mouse control (move, click, scroll) |

```bash
# Debian/Ubuntu
sudo apt install -y grim wl-clipboard ydotool python3-gi gstreamer1.0-tools gstreamer1.0-pipewire xdg-desktop-portal xdg-desktop-portal-gnome

# Arch
sudo pacman -S grim wl-clipboard ydotool python-gobject gstreamer gst-plugin-pipewire xdg-desktop-portal xdg-desktop-portal-gnome

# Install and enable the GNOME Shell extension
cp -r extensions/deskbrid@deskbrid ~/.local/share/gnome-shell/extensions/
gnome-extensions enable deskbrid@deskbrid
# Log out and back in, or restart GNOME Shell (Alt+F2, type 'r' on X11)
```

## Hyprland

| Dependency | Package | Purpose |
|---|---|---|
| `grim` | `grim` | Wayland screenshots |
| `wl-paste` / `wl-copy` | `wl-clipboard` | Clipboard read/write |
| `ydotool` + `ydotoold` | `ydotool` | Keyboard and mouse input injection |

```bash
# Arch
sudo pacman -S grim wl-clipboard ydotool

# Debian/Ubuntu — ydotool from source or backports
sudo apt install -y grim wl-clipboard
```

**/dev/uinput permissions:** ydotool needs write access to `/dev/uinput`. On most distros it's root-only by default:

```bash
echo 'KERNEL=="uinput", GROUP="input", MODE="0660"' | sudo tee /etc/udev/rules.d/99-input.rules
sudo udevadm control --reload-rules
sudo chmod 0660 /dev/uinput && sudo chgrp input /dev/uinput
sudo usermod -aG input $USER
# Log out and back in for group to take effect
```

Add to `~/.config/hypr/hyprland.conf`:
```
exec-once = ydotoold
```

## KDE

| Dependency | Package | Purpose |
|---|---|---|
| `spectacle` | `spectacle` | Wayland screenshots (full screen) |
| `convert` | `imagemagick` | Window/region screenshot cropping |
| `wl-paste` / `wl-copy` | `wl-clipboard` | Clipboard read/write |
| `ydotool` + `ydotoold` | `ydotool` | Keyboard and mouse input injection |

```bash
# Debian/Ubuntu
sudo apt install -y spectacle imagemagick wl-clipboard ydotool

# Arch
sudo pacman -S spectacle imagemagick wl-clipboard ydotool
```

**ydotoold:** Must run as user (not root). Add to KDE autostart:

```bash
mkdir -p ~/.config/autostart
cat > ~/.config/autostart/ydotoold.desktop << 'EOF'
[Desktop Entry]
Type=Application
Name=ydotoold
Exec=ydotoold
Terminal=false
X-KDE-autostart-phase=2
EOF
```

**/dev/uinput permissions:** Same as Hyprland — ydotool needs write access:

```bash
echo 'KERNEL=="uinput", GROUP="input", MODE="0660"' | sudo tee /etc/udev/rules.d/99-input.rules
sudo udevadm control --reload-rules
sudo chmod 0660 /dev/uinput && sudo chgrp input /dev/uinput
sudo usermod -aG input $USER
# Log out and back in for group to take effect
```

## Sway / Niri / Wayfire / Labwc

These wlroots-style backends use compositor CLIs for windows/workspaces and
shared Wayland tools for input, clipboard, screenshots, and monitor control.

| Backend | Extra dependency | Purpose |
|---|---|---|
| Sway | `swaymsg` | Windows, workspaces, monitors |
| Niri | `niri` | Windows and workspaces via `niri msg` |
| Wayfire | `wf-ipc` | Windows and workspaces |
| Labwc | `wlrctl` | Window listing/focus/close/maximize |
| Niri / Wayfire / Labwc | `wlr-randr` | Monitor mode, scale, rotation, enable/disable |

| Shared dependency | Package | Purpose |
|---|---|---|
| `grim` | `grim` | Wayland screenshots |
| `identify` | `imagemagick` | Screenshot dimensions |
| `wl-paste` / `wl-copy` | `wl-clipboard` | Clipboard read/write |
| `ydotool` + `ydotoold` | `ydotool` | Keyboard and mouse input injection |

```bash
# Debian/Ubuntu
sudo apt install -y grim imagemagick wl-clipboard ydotool wlr-randr

# Arch
sudo pacman -S grim imagemagick wl-clipboard ydotool wlr-randr
```

Install the compositor-specific CLI from your distro package for that compositor
(`sway`, `niri`, `wayfire`, or `wlrctl`). The same `/dev/uinput` permissions and
`ydotoold` setup from Hyprland/KDE apply.

## COSMIC

COSMIC support uses the in-repo `cosmic-helper` plus standard Wayland tooling.
Monitor control uses `cosmic-randr`; monitor discovery also tries `wlr-randr`
when available. Window move/resize is currently marked unsupported in
`system.capabilities`.

| Dependency | Package | Purpose |
|---|---|---|
| `cosmic-helper` | built from this repo | COSMIC window/workspace bridge where supported |
| `cosmic-randr` | COSMIC package | Monitor control |
| `wlr-randr` | `wlr-randr` | Monitor discovery fallback |
| `grim` | `grim` | Wayland screenshots |
| `wl-paste` / `wl-copy` | `wl-clipboard` | Clipboard read/write |
| `ydotool` + `ydotoold` | `ydotool` | Keyboard and mouse input injection |

## X11

The X11 backend uses xdotool for input and most window operations, wmctrl for window listing and maximize, xclip for clipboard, and ImageMagick for screenshots — no ydotoold required since X11 grants direct XTest extension access.

| Dependency | Package | Purpose |
|---|---|---|
| `xdotool` | `xdotool` | Window focus/get/close/minimize/move/resize, keyboard input (type/key/combo), mouse (move/click/scroll), workspace switch |
| `wmctrl` | `wmctrl` | X11 window listing and maximize |
| `xclip` | `xclip` | Clipboard read/write |
| `import` | `imagemagick` | Screenshot capture (fullscreen and region crop) |
| `notify-send` | `libnotify` | Desktop notifications |

```bash
# Debian/Ubuntu
sudo apt install -y xdotool wmctrl xclip imagemagick libnotify-bin

# Arch
sudo pacman -S xdotool wmctrl xclip imagemagick libnotify

# Fedora
sudo dnf install -y xdotool wmctrl xclip ImageMagick libnotify
```

X11 does **not** need ydotoold, udev rules, or any compositor-specific extension. It works immediately on any X11 desktop (Xfce, MATE, Cinnamon, i3, etc.).

## Optional (all backends)

| Dependency | Package | Purpose |
|---|---|---|
| `pactl` | `pulseaudio-utils` or `pipewire-pulse` | Audio sink listing and volume |
| `nmcli` | `network-manager` | WiFi scanning and connection |
| `bluetoothctl` | `bluez` | Bluetooth device management |
| `notify-send` | `libnotify` | Desktop notifications |
| `tesseract` | `tesseract-ocr` plus language packs such as `tesseract-ocr-eng` | Screenshot OCR |
