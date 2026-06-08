# Clipboard

Read and write clipboard contents across GNOME, Hyprland, KDE, and X11. Clipboard
history is a separate queue and does not overwrite the current clipboard.

## Actions

- `clipboard.read`
- `clipboard.write`
- `clipboard.history`
- `clipboard.history.clear`

## Requirements

- Wayland: `wl-clipboard`
- X11: shared X11 clipboard access (no extra dependency)
- GNOME remote clipboard requires the Shell extension

If `clipboard.read` returns empty, run `deskbrid system health` and check the
clipboard subsystem status.

## Rate limits

Clipboard reads are subject to per-namespace token-bucket rate limits configured
in `permissions.toml`:

```toml
[rate_limits.clipboard]
rpm = 120
burst = 20
```

Exceeding the bucket returns a `RATE_LIMITED` response with `retry_after_ms`.

## Python example

```python
from deskbrid import Deskbrid
client = Deskbrid()

text = client.clipboard_read()
client.clipboard_write(text)
history = client.clipboard_history()
client.clipboard_history_clear()
```

## Notes

- History is persisted in SQLite across restarts.
- Some Wayland compositors restrict clipboard access between sandboxed apps.
