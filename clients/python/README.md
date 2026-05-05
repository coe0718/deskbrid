# deskbrid Python Client

The Python client gives you a direct path into the Deskbrid daemon over its Unix socket. Use it when you want typed desktop actions, event subscriptions, and automatic decoding of protocol responses into Python dataclasses.

## Quick Start

### Install

```bash
pip install ./clients/python
```

### Connect and inspect the daemon

```python
from deskbrid import Deskbrid

client = Deskbrid()
try:
    info = client.info()
    print(info)
finally:
    client.close()
```

### Subscribe to events

```python
from deskbrid import Deskbrid

client = Deskbrid()

@client.on("window:focus")
def handle_focus(window):
    print(f"Focused: {window.app_id} :: {window.title}")

@client.on("clipboard")
def handle_clipboard(clip):
    print(f"Clipboard changed: {clip.text}")

client.listen()
```

## Sync vs Async Usage

Use `Deskbrid` when you want a blocking client from normal Python code. Use `AsyncDeskbrid` when you already have an asyncio app and want to await actions directly.

### Synchronous client

```python
from deskbrid import Deskbrid

client = Deskbrid()
try:
    client.type_text("hello from sync mode\n")
    print(client.list_windows())
finally:
    client.close()
```

### Asynchronous client

```python
import asyncio

from deskbrid import AsyncDeskbrid


async def main() -> None:
    client = AsyncDeskbrid()
    await client.connect()
    try:
        await client.type_text("hello from async mode\n")
        print(await client.list_windows())
    finally:
        await client.close()


asyncio.run(main())
```

## Actions

### `type_text`

```python
from deskbrid import Deskbrid

client = Deskbrid()
try:
    client.type_text("hello world\n")
finally:
    client.close()
```

### `send_keys`

```python
from deskbrid import Deskbrid

client = Deskbrid()
try:
    client.send_keys(["ctrl", "shift", "t"])
finally:
    client.close()
```

### `mouse_click`

```python
from deskbrid import Deskbrid

client = Deskbrid()
try:
    client.mouse_click(640, 360, button="left")
finally:
    client.close()
```

### `clipboard_read`

```python
from deskbrid import Deskbrid

client = Deskbrid()
try:
    clip = client.clipboard_read()
    print(clip.text)
    print(clip.mime_types)
finally:
    client.close()
```

### `clipboard_write`

```python
from deskbrid import Deskbrid

client = Deskbrid()
try:
    client.clipboard_write("deskbrid copied this")
finally:
    client.close()
```

### `screenshot`

Sync `Deskbrid.screenshot()` returns the saved path as a string.

```python
from deskbrid import Deskbrid

client = Deskbrid()
try:
    path = client.screenshot()
    print(path)
finally:
    client.close()
```

Async `AsyncDeskbrid.screenshot()` returns a `ScreenshotResult`.

```python
import asyncio

from deskbrid import AsyncDeskbrid


async def main() -> None:
    client = AsyncDeskbrid()
    await client.connect()
    try:
        shot = await client.screenshot()
        print(shot.path)
    finally:
        await client.close()


asyncio.run(main())
```

### `notify`

```python
from deskbrid import Deskbrid

client = Deskbrid()
try:
    notification_id = client.notify(
        "Deskbrid",
        "Build finished successfully",
        urgency="low",
    )
    print(notification_id)
finally:
    client.close()
```

### `list_windows`

```python
from deskbrid import Deskbrid

client = Deskbrid()
try:
    for window in client.list_windows():
        print(window.app_id, window.title, window.focused)
finally:
    client.close()
```

### `focus_window`

```python
from deskbrid import Deskbrid

client = Deskbrid()
try:
    client.focus_window(app_id="firefox")
    client.focus_window(title="Terminal", exact=False)
finally:
    client.close()
```

### `list_displays`

```python
from deskbrid import Deskbrid

client = Deskbrid()
try:
    for monitor in client.list_displays():
        print(monitor.id, monitor.width, monitor.height, monitor.scale)
finally:
    client.close()
```

### `info`

```python
from deskbrid import Deskbrid

client = Deskbrid()
try:
    info = client.info()
    print(info.deskbrid_version)
    print(info.desktop, info.session_type)
    print(info.capabilities)
finally:
    client.close()
```

## Event Subscriptions

The decorator form is the intended pattern for event handlers.

### Sync listener

```python
from deskbrid import Deskbrid

client = Deskbrid()

@client.on("window:focus")
def on_focus(window):
    print(f"Focus changed to: {window.title}")

@client.on("notifications")
def on_notification(event):
    print(f"Notification: {event.summary}")

client.listen()
```

### Async listener

```python
import asyncio

from deskbrid import AsyncDeskbrid


async def main() -> None:
    client = AsyncDeskbrid()

    @client.on("clipboard")
    async def on_clipboard(clip):
        print(f"Clipboard updated: {clip.text}")

    @client.on("window:close")
    def on_window_close(event):
        print(f"Closed PID {event.pid} from {event.app_id}")

    await client.listen()


asyncio.run(main())
```

Common event names:

| Event | Payload type |
|---|---|
| `window:focus` | `WindowInfo` |
| `window:open` | `WindowInfo` |
| `window:close` | `WindowClosedEvent` |
| `clipboard` | `ClipboardContent` |
| `notifications` | `NotificationEvent` |
| `idle` | `IdleEvent` |
| `audio:node` | `AudioNodeEvent` |

## Error Handling

All protocol failures are surfaced as `DeskbridError`.

```python
from deskbrid import Deskbrid, DeskbridError

client = Deskbrid()
try:
    try:
        client.focus_window(title="Definitely Not Open", exact=True)
    except DeskbridError as exc:
        print(exc.code)
        print(exc.message)
finally:
    client.close()
```

Async code uses the same exception type.

```python
import asyncio

from deskbrid import AsyncDeskbrid, DeskbridError


async def main() -> None:
    client = AsyncDeskbrid()
    await client.connect()
    try:
        try:
            await client.send_keys(["ctrl", "made_up_key"])
        except DeskbridError as exc:
            print(f"{exc.code}: {exc.message}")
    finally:
        await client.close()


asyncio.run(main())
```

## Full API Reference

### Sync client: `Deskbrid`

| Method | Returns | Notes |
|---|---|---|
| `Deskbrid(socket_path=None, reconnect_delay=1.0)` | `Deskbrid` | Connects immediately on construction. |
| `close()` | `None` | Closes the background event loop thread. |
| `on(event)` | decorator | Registers an event callback and syncs subscriptions. |
| `listen()` | `None` | Blocks and keeps the connection alive for event handling. |
| `type_text(text)` | `None` | Sends `inject:type`. |
| `send_keys(keys)` | `None` | Sends `inject:key`. |
| `mouse_click(x, y, button="left")` | `None` | Sends a click action. |
| `clipboard_read()` | `ClipboardContent` | Reads current clipboard text and mime types. |
| `clipboard_write(text)` | `None` | Writes clipboard text. |
| `screenshot(monitor=None)` | `str` | Returns the saved screenshot path. |
| `notify(title, body="", urgency="normal")` | `int` | Returns notification ID when available. |
| `list_windows()` | `list[WindowInfo]` | Lists open windows. |
| `focus_window(app_id=None, title=None, exact=False)` | `None` | Focuses by app ID or title. |
| `list_displays()` | `list[MonitorInfo]` | Lists active displays. |
| `info()` | `DaemonInfo` | Returns daemon metadata and capabilities. |

### Async client: `AsyncDeskbrid`

| Method | Returns | Notes |
|---|---|---|
| `AsyncDeskbrid(socket_path=None, reconnect_delay=1.0)` | `AsyncDeskbrid` | Creates the client without connecting. |
| `connect()` | `None` | Opens the Unix socket and consumes the daemon hello message. |
| `close()` | `None` | Closes the socket and background tasks. |
| `on(event)` | decorator | Registers sync or async callbacks. |
| `subscribe(*events)` | `None` | Adds placeholder listeners and sends a subscribe message. |
| `listen()` | `None` | Connects, then waits indefinitely while events arrive. |
| `type_text(text)` | `None` | Sends `inject:type`. |
| `send_keys(keys)` | `None` | Sends `inject:key`. |
| `mouse_click(x, y, button="left")` | `None` | Sends click input. |
| `mouse_move(x, y)` | `None` | Sends pointer move input. |
| `mouse_scroll(dx=0.0, dy=0.0)` | `None` | Sends scroll input. |
| `clipboard_read()` | `ClipboardContent` | Reads current clipboard text and mime types. |
| `clipboard_write(text)` | `None` | Writes clipboard text. |
| `screenshot(monitor=None)` | `ScreenshotResult` | Returns structured screenshot metadata. |
| `notify(title, body="", urgency="normal")` | `int` | Returns notification ID when available. |
| `list_windows()` | `list[WindowInfo]` | Lists open windows. |
| `focus_window(app_id=None, title=None, exact=False)` | `None` | Focuses by app ID or title. |
| `list_displays()` | `list[MonitorInfo]` | Lists active displays. |
| `info()` | `DaemonInfo` | Returns daemon metadata and capabilities. |

### Data models

| Type | Fields |
|---|---|
| `WindowInfo` | `title`, `app_id`, `pid`, `workspace`, `focused`, `geometry`, `wm_class` |
| `WindowClosedEvent` | `app_id`, `pid` |
| `ClipboardContent` | `text`, `mime_types`, `timestamp` |
| `NotificationEvent` | `app`, `app_icon`, `summary`, `body`, `urgency`, `id` |
| `IdleEvent` | `idle`, `idle_since`, `idle_seconds` |
| `AudioNodeEvent` | `id`, `name`, `state`, `volume`, `muted` |
| `MonitorInfo` | `id`, `width`, `height`, `scale`, `refresh` |
| `ScreenshotResult` | `path`, `width`, `height` |
| `DaemonInfo` | `deskbrid_version`, `desktop`, `session_type`, `capabilities` |

## Notes

- A running Deskbrid daemon is required before the client can connect.
- The default socket path is `$XDG_RUNTIME_DIR/deskbrid/socket`.
- Input injection depends on GNOME Wayland remote desktop capabilities being available in the session.
