# Clipboard

Read and write clipboard contents, and manage clipboard history. Works across
GNOME, Hyprland, KDE, and X11.

## Actions

### clipboard.read

Read the current clipboard contents as plain text.

```bash
deskbrid clipboard.read
```

No parameters.

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "text": "Copied text here",
    "mime_type": "text/plain"
  }
}
```

### clipboard.write

Write text to the clipboard.

| Parameter   | Type    | Description              |
|-------------|---------|--------------------------|
| `text`      | string  | The text to copy         |
| `mime_type` | string? | MIME type (default: `text/plain`) |

```bash
deskbrid clipboard.write '{"text": "Hello, world!"}'
```

```json
{
  "type": "clipboard.write",
  "text": "Hello, world!",
  "mime_type": "text/plain"
}
```

### clipboard.history_list

List recent clipboard history entries.

| Parameter | Type   | Description               |
|-----------|--------|---------------------------|
| `limit`   | uint?  | Max entries (default: 20) |
| `offset`  | uint?  | Pagination offset         |

```bash
deskbrid clipboard.history_list '{"limit": 5, "offset": 0}'
```

```json
{
  "type": "clipboard.history_list",
  "limit": 5,
  "offset": 0
}
```

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {"id": 1, "text": "Hello, world!", "copied_at": "2024-01-15T10:00:00Z"},
    {"id": 2, "text": "Copied text here", "copied_at": "2024-01-15T09:59:00Z"}
  ]
}
```

### clipboard.history_clear

Clear the entire clipboard history.

```bash
deskbrid clipboard.history_clear
```

No parameters.

## Requirements

- **Wayland**: `wl-clipboard` (for `wl-paste` / `wl-copy`)
- **X11**: Shared X11 clipboard access (no extra dependency)
- **GNOME**: Remote clipboard requires the Shell extension

If `clipboard.read` returns empty, run `deskbrid system health` and check the
clipboard subsystem status.

## Rate Limits

Clipboard reads are subject to per-namespace token-bucket rate limits configured
in `permissions.toml`:

```toml
[rate_limits.clipboard]
rpm = 120
burst = 20
```

Exceeding the bucket returns a `RATE_LIMITED` response with `retry_after_ms`.

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# Read current clipboard
text = client.clipboard_read()
print(f"Current clipboard: {text}")

# Write to clipboard
client.clipboard_write(text="Hello, world!")

# Browse history
history = client.clipboard_history_list(limit=10)
for entry in history:
    print(f"  [{entry['id']}] {entry['text'][:40]}")

# Clear history
client.clipboard_history_clear()
```

## Notes

- History is persisted in SQLite across restarts.
- Some Wayland compositors restrict clipboard access between sandboxed apps.
- Clipboard history is a separate queue and does not overwrite the current
  clipboard selection.

## Current Status

**Stable** — core clipboard operations (read, write).
**Experimental** — clipboard history (list, clear).
