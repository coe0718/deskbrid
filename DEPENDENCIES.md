# Dependencies

Deskbrid auto-detects your desktop environment and loads the matching backend. Dependencies vary by backend — install only what your compositor needs.

## GNOME

| Dependency | Package | Purpose |
|---|---|---|
| GNOME Shell Extension | `extensions/deskbrid@deskbrid/` (in-repo) | Window listing, focus, workspace control |
| `grim` | `grim` | Wayland screenshots |
| `wl-paste` / `wl-copy` | `wl-clipboard` | Clipboard read/write |
| `ydotool` | `ydotool` | Mouse control (move, click, scroll) |

```bash
sudo apt install -y grim wl-clipboard ydotool

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

## X11

The X11 backend uses xdotool for input and most window operations, wmctrl for maximize, xclip for clipboard, and ImageMagick for screenshots — no ydotoold required since X11 grants direct XTest extension access.

| Dependency | Package | Purpose |
|---|---|---|
| `xdotool` | `xdotool` | Window focus/get/close/minimize/move/resize, keyboard input (type/key/combo), mouse (move/click/scroll), workspace switch |
| `wmctrl` | `wmctrl` | X11 window maximize |
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
