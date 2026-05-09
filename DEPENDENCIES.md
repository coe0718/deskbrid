# Dependencies

Deskbrid relies on several system packages. Most are available via `apt`.

## Required

| Dependency | Package | Purpose |
|---|---|---|
| GNOME Shell Extension | `extensions/deskbrid@deskbrid/` (in-repo) | Window listing, focus, and state-change signals over DBus |
| `grim` | `grim` | Wayland screenshots |
| `wl-paste` / `wl-copy` | `wl-clipboard` | Clipboard read/write |
| `wtype` | `wtype` | Keyboard text injection |

Install the GNOME extension:
```bash
cp -r extensions/deskbrid@deskbrid ~/.local/share/gnome-shell/extensions/
gnome-extensions enable deskbrid@deskbrid
# Log out and back in, or restart GNOME Shell (Alt+F2, type 'r')
```

## Optional

| Dependency | Package | Purpose |
|---|---|---|
| `ydotool` | `ydotool` | Mouse control (move, click, scroll) |
| `pactl` | `pulseaudio-utils` or `pipewire-pulse` | Audio sink listing and volume |
| `nmcli` | `network-manager` | WiFi scanning and connection |

## Quick Install

```bash
sudo apt install -y grim wl-clipboard wtype ydotool
```
