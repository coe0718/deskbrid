# Input Control

Simulate keyboard and mouse input.

## Keyboard

### Type Text

```bash
deskbrid input keyboard type "Hello, world!"
```

Protocol:
```json
{"action": "input.keyboard", "action": "type", "text": "Hello, world!"}
```

### Press a Single Key

```bash
deskbrid input keyboard key Return
deskbrid input keyboard key Escape
deskbrid input keyboard key F5
```

Protocol:
```json
{"action": "input.keyboard", "action": "key", "key": "Return"}
```

### Send Key Combinations

```bash
deskbrid combo Ctrl_L+c          # Copy
deskbrid combo Ctrl_L+v          # Paste
deskbrid combo Super_L+Tab       # Alt-tab
deskbrid combo Ctrl_L+Shift_L+Left  # Select word left
```

Protocol:
```json
{"action": "input.keyboard", "action": "combo", "keys": ["Ctrl_L", "c"]}
```

### Available Key Names

Common keys:
- `Return`, `Enter` - Enter/Return
- `Escape`, `Esc` - Escape
- `Space` - Spacebar
- `Tab` - Tab
- `BackSpace` - Backspace
- `Delete` - Delete
- `Insert` - Insert
- `Home`, `End`, `Page_Up`, `Page_Down`
- `F1` through `F12`
- `Shift_L`, `Shift_R` - Shift
- `Control_L`, `Control_R` - Ctrl
- `Alt_L`, `Alt_R` - Alt
- `Super_L`, `Super_R` - Windows key

## Mouse

### Click

```bash
deskbrid mouse click --button left
deskbrid mouse click --button right
deskbrid mouse click --button middle
deskbrid mouse click --x 100 --y 200
```

Protocol:
```json
{"action": "input.mouse", "action": "click", "x": 100, "y": 200, "button": "left"}
```

### Move Mouse

```bash
deskbrid mouse move --x 500 --y 300
```

Protocol:
```json
{"action": "input.mouse", "action": "move", "x": 500, "y": 300}
```

### Scroll

```bash
deskbrid mouse scroll --dy 3       # Scroll up 3 lines
deskbrid mouse scroll --dx -5      # Scroll left
deskbrid mouse scroll --dy 10      # Scroll down 10 lines
```

Protocol:
```json
{"action": "input.mouse", "action": "scroll", "dx": 0, "dy": 3}
```

## Desktop-Specific Notes

### Wayland (GNOME, KDE, Hyprland)

Under Wayland, input simulation requires:

1. **ydotoold** for keyboard/mouse (X11-style injection)
2. **graceful fallback** - Deskbrid tries ydotool first, falls back to virtual input

Setup:
```bash
# Arch
sudo pacman -S ydotool
sudo systemctl enable --now ydotoold

# Debian (may need to build from source)
sudo apt install ydotool
```

### X11 (Cinnamon, MATE, XFCE)

Uses `xdotool` directly:

```bash
sudo apt install xdotool  # Debian
sudo pacman -S xdotool    # Arch
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# Navigate in a terminal
client.type_text("cd /home/user/project\n")
client.combo(["Ctrl_L", "c"])  # Copy

# Click a button at coordinates
client.mouse_click(x=500, y=300, button="left")

# Scroll down
client.mouse_scroll(dy=5)
```

## AI Agent Example

```json
→ {"action": "input.keyboard", "action": "type", "text": "npm test\n"}
← {"type": "response", "status": "ok"}

→ {"action": "input.keyboard", "action": "combo", "keys": ["Alt_L", "F4"]}
← {"type": "response", "status": "ok"}
```