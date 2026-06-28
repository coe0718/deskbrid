# Python Client

Use Deskbrid from Python applications.

## Installation

```bash
pip install deskbrid
```

Or install from source:

```bash
git clone https://github.com/coe0718/deskbrid
cd deskbrid/clients/python
pip install -e .
```

## Quick Start

```python
from deskbrid import Deskbrid

# Connect to daemon
client = Deskbrid()

# List windows
windows = client.list_windows()
for w in windows:
    print(f"{w.app_id}: {w.title}")
```

## API

| Method | Action | Description |
|--------|--------|-------------|
| `list_windows()` | `windows.list` | List all open windows |
| `focus_window(app_id=...)` | `windows.focus` | Focus a window by app_id |
| `get_window(window_id=...)` | `windows.get` | Get window details |
| `type_text(text)` | `input.keyboard` | Type text |
| `press_key(combo)` | `input.keyboard` | Press key combination |
| `read_clipboard()` | `clipboard.read` | Read clipboard contents |
| `write_clipboard(text)` | `clipboard.write` | Write to clipboard |
| `screenshot()` | `screenshot` | Take a screenshot |
| `get_system_info()` | `system.info` | Get system information |
| `send_notification(title, body)` | `notification.send` | Send a notification |
| `list_monitors()` | `monitors.list` | List connected monitors |
| `list_services()` | `service.list` | List systemd services |
| `start_service(name)` | `service.start` | Start a systemd service |
| `stop_service(name)` | `service.stop` | Stop a systemd service |

## Full Example

```python
import asyncio
from deskbrid import Deskbrid

async def main():
    client = Deskbrid()

    # Get system info
    info = await client.get_system_info()
    print(f"Host: {info.hostname}")

    # Take a screenshot
    await client.screenshot(path="/tmp/desktop.png")

    # List windows
    windows = await client.list_windows()
    for w in windows:
        print(f"  {w.app_id}: {w.title}")

    # Focus VS Code
    await client.focus_window(app_id="code")

    # Type and press enter
    await client.type_text("Hello from Deskbrid!\\n")

    # Read clipboard
    clip = await client.read_clipboard()
    print(f"Clipboard: {clip}")

    # Subscribe to events
    async for event, data in client.events(["window.*"]):
        print(f"{event}: {data}")

asyncio.run(main())
```

## Async API

All methods have both sync and async versions:

```python
# Sync
windows = client.list_windows()

# Async
windows = await client.list_windows_async()
```
```

## Async Client

```python
from deskbrid import AsyncDeskbrid
import asyncio

async def main():
    async with AsyncDeskbrid() as client:
        windows = await client.list_windows()
        print(f"Found {len(windows)} windows")

asyncio.run(main())
```

## Client Types

### SyncDeskbrid

Blocking synchronous client:

```python
from deskbrid import SyncDeskbrid

client = SyncDeskbrid()
windows = client.list_windows()  # Blocks until response
```

### AsyncDeskbrid

Async context manager:

```python
from deskbrid import AsyncDeskbrid

async with AsyncDeskbrid() as client:
    windows = await client.list_windows()
```

### Shared Async/Sync Client

The main `Deskbrid` class provides both:

```python
client = Deskbrid()

# Sync methods (blocking)
windows = client.list_windows()
client.type_text("sync text")

# Internal async client for mixing modes
async def some_async_func():
    async with client._client() as async_client:
        await async_client.type_text("async text")
```

## Available Methods

### Windows
```python
client.list_windows()           # List all windows
client.focus_window(app_id="code")  # Focus by app ID
client.focus_window(title="Terminal")  # Focus by title
client.activate_or_launch("firefox")  # Launch or focus
client.tile_window(window_id, preset="left")
```

### Input
```python
client.type_text("Hello!")
client.send_keys(["Ctrl_L", "c"])
client.mouse_click(x=100, y=200)
client.mouse_move(x=500, y=300)
client.mouse_scroll(dy=3)
```

### Clipboard
```python
content = client.clipboard_read()  # Returns ClipboardContent
client.clipboard_write("New text")
history = client.clipboard_history(limit=10)
```

### Screenshots
```python
path = client.screenshot()  # Returns screenshot path
result = client.screenshot_ocr()  # Returns dict with text
diff = client.screenshot_diff("/before.png", "/after.png")
```

### System
```python
info = client.info()  # Returns DaemonInfo object
battery = client.battery()  # Returns dict with battery info
client.system_power("suspend")
```

### Media
```python
players = client.mpris_list()
client.mpris_control("play")
client.mpris_control("next")
```

### Terminals
```python
result = client.terminal_create()
term_id = result["terminal_id"]
client.terminal_write(term_id, "ls -la\n")
output = client.terminal_read(term_id)
client.terminal_kill(term_id)
```

## Error Handling

```python
from deskbrid.errors import DeskbridError

try:
    client.focus_window(app_id="nonexistent")
except DeskbridError as e:
    print(f"Error: {e.code} - {e.message}")
```

## Connection Options

```python
from deskbrid import Deskbrid

# Custom socket path
client = Deskbrid(socket_path="/tmp/custom.sock")

# With reconnect delay
client = Deskbrid(reconnect_delay=2.0)
```

## Running Multiple Clients

```python
# Each client has its own connection
client1 = Deskbrid()
client2 = Deskbrid()

# Use different clients for different tasks concurrently
```

## Integration with AI Frameworks

### LangChain

```python
from langchain.tools import tool
from deskbrid import Deskbrid

client = Deskbrid()

@tool
def type_text_tool(text: str) -> str:
    """Type text into the focused window."""
    client.type_text(text)
    return f"Typed: {text}"
```

### LlamaIndex

```python
from llama_index.core.tools import FunctionTool
from deskbrid import Deskbrid

client = Deskbrid()

def take_screenshot() -> str:
    """Take a screenshot and return the path."""
    result = client.screenshot()
    return result.path

screenshot_tool = FunctionTool.from_defaults(
    fn=take_screenshot,
    name="take_screenshot",
    description="Take a screenshot of the desktop"
)
```

## Method Reference

All methods return objects (not raw dicts), with synchronous wrappers provided via `SyncDeskbrid`:

### Windows
```python
client.list_windows()           # Returns list[WindowInfo]
client.focus_window(app_id="code")  # None
client.activate_or_launch(app_id, command=None, workdir=None, env=None)  # Returns dict
client.tile_window(window_id, preset, monitor=None, padding=None)  # Returns dict
```

### Input
```python
client.type_text("Hello!")      # None (fire-and-forget)
client.send_keys(["Ctrl_L", "c"])  # None
client.mouse_click(x=100, y=200, button="left")  # None
client.mouse_move(x=500, y=300)  # None
client.mouse_scroll(dy=3)        # None
client.mouse_drag(from_x, from_y, to_x, to_y, button="left", duration_ms=None)  # Returns dict
```

### Clipboard
```python
content = client.clipboard_read()  # Returns ClipboardContent object
client.clipboard_write("New text")  # None
history = client.clipboard_history(limit=10)  # Returns list[dict]
```

### Screenshots
```python
result = client.screenshot(monitor=None)  # Returns ScreenshotResult with .path attribute
result = client.screenshot_ocr(path=None, language=None, monitor=None, region=None)  # Returns dict
result = client.screenshot_diff("/before.png", "/after.png")  # Returns dict
```

### System
```python
info = client.info()  # Returns DaemonInfo object
battery = client.battery()  # Returns dict
backlight = client.backlight_get()  # Returns dict
client.backlight_set(percent=0.5)  # Returns dict
thermal = client.thermal()  # Returns dict
freq = client.cpu_frequency()  # Returns dict
gov = client.cpu_governor()  # Returns dict
client.cpu_set_governor("performance")  # Returns dict

# Session/system control
inhibit = client.inhibit_system("suspend", who="backup", why="backup running")
client.release_inhibit(inhibit["inhibitor_id"])
sessions = client.list_sessions()
client.lock_session()
client.switch_user("alice")
auth = client.check_auth("system.power")
client.elevate("system.power", reason="rebooting")
conf = client.confinement()
```

### Services
```python
client.service_status(name)
client.service_start(name)
client.service_stop(name)
client.service_restart(name)
client.service_enable(name, runtime=False)
client.service_disable(name, runtime=False)
client.service_list(unit_type=None)
```

### Timers
```python
client.timer_list()
client.timer_start(name)
client.timer_stop(name)
```

### Audit
```python
client.audit_log(limit=None, action_type=None, status=None)  # Returns list[dict]
client.audit_clear()  # Returns dict
```

### Wait For
```python
client.wait_for(condition, params=None, timeout_ms=30000, interval_ms=None)  # Returns dict
```