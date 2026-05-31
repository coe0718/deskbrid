# Macros

Deskbrid's macro system allows you to record and replay sequences of actions. Macros can be recorded in two modes: fast (no delays) or timed (preserves original timing). Macros are stored as JSON files in `~/.local/share/deskbrid/macros/`.

## Overview

Macros capture sequences of deskbrid actions and can replay them later. Each macro has:
- A name for identification
- Creation timestamp
- Mode (fast or timed)
- A list of recorded actions with timestamps (in timed mode)

## Starting Recording

```bash
deskbrid macro.record.start { name: "my-macro" }
```

### Parameters

- `name`: Identifier for the macro (required)
- `mode`: Recording mode - "fast" or "timed" (defaults to "fast")

## Stopping Recording

```bash
deskbrid macro.record.stop
```

## Listing Macros

```bash
deskbrid macro.list
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "macros": [
      {
        "name": "my-macro",
        "mode": "timed",
        "created": "2026-05-30T10:00:00Z",
        "actions": 15,
        "duration_ms": 2500
      },
      {
        "name": "another-macro",
        "mode": "fast",
        "created": "2026-05-30T10:05:00Z",
        "actions": 8,
        "duration_ms": 0
      }
    ]
  }
}
```

## Getting a Macro

```bash
deskbrid macro.get { name: "my-macro" }
```

Response includes the full macro definition with all recorded actions.

## Replaying a Macro

```bash
deskbrid macro.replay { name: "my-macro" }
deskbrid macro.replay { name: "my-macro", mode: "fast" }  # Override mode
```

### Parameters

- `name`: Name of the macro to replay (required)
- `mode`: Replay mode - "fast" or "timed" (defaults to macro's recorded mode)

## Exporting a Macro

```bash
deskbrid macro.export { name: "my-macro" }
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "name": "my-macro",
    "mode": "timed",
    "created": "2026-05-30T10:00:00Z",
    "actions": [
      {
        "type": "windows.list",
        "timestamp": 0
      },
      {
        "type": "input.keyboard",
        "action": "type",
        "text": "Hello, world!",
        "timestamp": 150
      }
      // ... more actions
    ]
  }
}
```

## Importing a Macro

```bash
deskbrid macro.import { name: "imported-macro", data: { /* macro JSON from export */ } }
```

### Parameters

- `name`: Identifier for the imported macro
- `data`: The macro JSON object (from export)

## Deleting a Macro

```bash
deskbrid macro.delete { name: "my-macro" }
```

## Examples

### Recording a Sequence of Actions

```bash
# Start recording in timed mode (preserves delays)
deskbrid macro.record.start { name: "greeting", mode: "timed" }

# Perform actions that will be recorded
deskbrid windows focus --app code
deskbrid input keyboard type "Hello, world!\n"
deskbrid input mouse move --x 100 --y 200
deskbrid input mouse click --button left

# Stop recording
deskbrid macro.record.stop
```

### Replaying with Different Modes

```bash
# Replay preserving original timing
deskbrid macro.replay { name: "greeting" }

# Replay as fast as possible
deskbrid macro.replay { name: "greeting", mode: "fast" }
```

### Sharing Macros Between Systems

```bash
# Export macro to share
deskbrid macro.export { name: "greeting" }
# Save the JSON output to a file and transfer to another system

# Import on another system
deskbrid macro.import { 
  name: "greeting", 
  data: { /* the exported JSON */ } 
}
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# Start recording
client.macro_record_start(name="test-macro", mode="timed")

# Perform some actions
client.windows_list()
client.input_keyboard_type("Hello, world!\\n")

# Stop recording
client.macro_record_stop()

# List macros
macros = client.macro_list()
for macro in macros['macros']:
    print(f"{macro['name']}: {macro['actions']} actions in {macro['mode']} mode")

# Get a macro
macro = client.macro_get(name="test-macro")
print(f"Macro has {len(macro['data']['actions'])} actions")

# Replay the macro
client.macro_replay(name="test-macro")

# Export for sharing
exported = client.macro_export(name="test-macro")
# saved_json = json.dumps(exported['data'])

# Import (would need the exported JSON)
# client.macro_import(name="imported-macro", data=saved_json)

# Delete when done
client.macro_delete(name="test-macro")
```