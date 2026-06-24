# Deskbrid Quickstart

## Daemon

```bash
deskbrid daemon --dashboard-bind 127.0.0.1
nohup deskbrid daemon --dashboard-bind 0.0.0.0 > /tmp/deskbrid.log 2>&1 &
```

## CLI Examples

```bash
deskbrid windows list
deskbrid windows focus --app-id firefox
deskbrid input type "Hello from Deskbrid!"
deskbrid system info
deskbrid screenshot
deskbrid clipboard read
```

## Unix Socket

```bash
echo '{"type":"system.info","id":"1"}' | nc -U $XDG_RUNTIME_DIR/deskbrid.sock
echo '{"type":"windows.list","id":"2"}' | nc -U $XDG_RUNTIME_DIR/deskbrid.sock
```

## Python Client

```python
from deskbrid import Deskbrid
client = Deskbrid()
client.focus_window(app_id='firefox')
client.type_text("Hello from Python!\n")
```

## MCP Integration

```bash
deskbrid mcp
hermes mcp add deskbrid --command "deskbrid mcp"
hermes mcp test deskbrid
```
