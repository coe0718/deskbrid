# Hotkeys

Register and unregister global keyboard shortcuts. When a registered hotkey
is pressed, the daemon emits an event to all subscribed clients.

## Actions

### hotkeys.register

Register a global hotkey combination.

| Parameter   | Type     | Description                                 |
|-------------|----------|---------------------------------------------|
| `hotkey_id` | string   | Unique identifier for this hotkey binding   |
| `keys`      | string[] | Key combination (e.g., `["Ctrl", "Shift", "A"]`) |

```bash
deskbrid hotkeys.register '{"hotkey_id": "screenshot-tool", "keys": ["Ctrl", "Shift", "S"]}'
```

```json
{
  "type": "hotkeys.register",
  "hotkey_id": "screenshot-tool",
  "keys": ["Ctrl", "Shift", "S"]
}
```

When the hotkey is pressed, subscribers receive:

```json
{
  "type": "event",
  "event": "hotkey.pressed",
  "data": {
    "hotkey_id": "screenshot-tool",
    "keys": ["Ctrl", "Shift", "S"]
  }
}
```

### hotkeys.unregister

Unregister a previously registered hotkey.

| Parameter   | Type   | Description                               |
|-------------|--------|-------------------------------------------|
| `hotkey_id` | string | ID of the hotkey to unregister            |

```bash
deskbrid hotkeys.unregister '{"hotkey_id": "screenshot-tool"}'
```

```json
{"type": "hotkeys.unregister", "hotkey_id": "screenshot-tool"}
```

## Python Example

```python
from deskbrid import Deskbrid
client = Deskbrid()

# Register a hotkey
client.hotkeys_register(
    hotkey_id="screenshot-tool",
    keys=["Ctrl", "Shift", "S"],
)

# Subscribe to hotkey events
client.connection_subscribe(events=["hotkey.*"])

# Later, unregister
client.hotkeys_unregister(hotkey_id="screenshot-tool")
```

## Requirements

- Global hotkeys require a Wayland session with Portal support, or X11
  (XGrabKey)
- Some desktop environments may restrict certain key combinations
- Not all DEs support programmatic hotkey registration

## Current Status

**Stable** — hotkey registration and unregistration supported.
