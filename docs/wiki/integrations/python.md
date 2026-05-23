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

# Focus VS Code
client.focus_window(app_id="code")

# Type something
client.type_text("Hello from Python!")
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
info = client.info()  # Returns DaemonInfo
battery = client.system_battery()
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

# With timeout
client = Deskbrid(timeout=30.0)
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