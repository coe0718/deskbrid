# Input Control

Simulate keyboard and mouse input. Deskbrid v1.0.0 dispatches input actions
directly over the socket (`input.keyboard`, `input.mouse`) and falls back to
visible-mode injection on desktop environments that block invisible input.

## Keyboard

Type text:

```bash
deskbrid input.keyboard { action: "type", text: "Hello, world!" }
```

Press a single key:

```bash
deskbrid input.keyboard { action: "key", key: "Return" }
```

Send a combination:

```bash
deskbrid input.keyboard { action: "combo", keys: ["Ctrl_L", "c"] }
```

Common keys: `Return`, `Escape`, `Space`, `Tab`, `BackSpace`, `Delete`, `F1`-
`F12`, `Shift_L`, `Control_L`, `Alt_L`, `Super_L`.

## Mouse

Click:

```bash
deskbrid input.mouse { action: "click", x: 100, y: 200, button: "left" }
```

Move:

```bash
deskbrid input.mouse { action: "move", x: 500, y: 300 }
```

Scroll:

```bash
deskbrid input.mouse { action: "scroll", dx: 0, dy: 3 }
```

Visible input mode is covered in the safer path docs. On Wayland, this mode
uses `ydotoold` under the hood.

## Python example

```python
from deskbrid import Deskbrid
client = Deskbrid()

client.input_keyboard_type("cd /home/user/project\\n")
client.input_keyboard_combo(keys=["Ctrl_L", "c"])
client.input_mouse_click(x=500, y=300, button="left")
client.input_mouse_scroll(dx=0, dy=5)
```
