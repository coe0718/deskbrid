# Clipboard

Read and write clipboard content, plus clipboard history.

## Read Clipboard

```bash
deskbrid clipboard read
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "type": "text",
    "text": "Hello, world!"
  }
}
```

Protocol:
```json
{"type": "clipboard.read"}
```

Python:
```python
from deskbrid import Deskbrid

client = Deskbrid()
content = client.clipboard_read()
print(content.text)  # "Hello, world!"
```

## Write Clipboard

```bash
deskbrid clipboard write "Hello from Deskbrid!"
```

Protocol:
```json
{"type": "clipboard.write", "text": "Hello from Deskbrid!"}
```

Python:
```python
client.clipboard_write("New clipboard content")
```

## Clipboard History

Keep track of clipboard changes over time.

**Note:** History requires the GNOME Shell extension or Hyprland autostart hook.

### List History

```bash
deskbrid clipboard history
deskbrid clipboard history --limit 10
deskbrid clipboard history --query "error"
```

Protocol:
```json
{"type": "clipboard.history", "limit": 10, "query": "error"}
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "entries": [
      {
        "id": 1,
        "text": "fixed the bug",
        "timestamp": "2024-01-15T10:30:00Z",
        "source": "terminal"
      },
      {
        "id": 2,
        "text": "git commit -m 'fix'",
        "timestamp": "2024-01-15T10:25:00Z",
        "source": "browser"
      }
    ]
  }
}
```

### Clear History

```bash
deskbrid clipboard history clear
```

Protocol:
```json
{"type": "clipboard.history.clear"}
```

Python:
```python
# Get recent history
entries = client.clipboard_history(limit=20)

# Search history
error_entries = client.clipboard_history(query="error")

# Clear history
result = client.clipboard_history_clear()
```

## Desktop-Specific Setup

### GNOME

Clipboard history requires the GNOME Shell extension:

```bash
deskbrid setup  # Enables the extension automatically
```

### Hyprland

Add to your Hyprland config:

```
exec-once = systemctl --user start deskbrid-history
```

Or enable clipboard history listener:

```bash
systemctl --user enable --now deskbrid-history.service
```

## AI Agent Example

```json
→ {"type": "clipboard.read"}
← {"type": "response", "status": "ok", "data": {"text": "def hello():"}}

→ {"type": "clipboard.write", "text": "def hello():\n    print('world')"}
← {"type": "response", "status": "ok"}
```