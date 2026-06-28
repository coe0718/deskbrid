# D-Bus

Make arbitrary D-Bus method calls on the system or session bus. Use this
for deep integration with desktop services that aren't exposed through
Deskbrid's higher-level actions.

## Actions

### d_bus.call

Call a D-Bus method on a specific service, path, and interface.

| Parameter   | Type      | Description                                         |
|-------------|-----------|-----------------------------------------------------|
| `bus`       | string?   | Bus type: `system` or `session` (default: `session`)|
| `service`   | string    | D-Bus service name (e.g., `org.freedesktop.NetworkManager`) |
| `path`      | string    | Object path (e.g., `/org/freedesktop/NetworkManager`) |
| `interface` | string    | Interface name (e.g., `org.freedesktop.DBus.Properties`) |
| `method`    | string    | Method name to call                                 |
| `args`      | JSON?     | Arguments to pass to the method                     |

```bash
deskbrid d_bus.call '{
  "bus": "session",
  "service": "org.freedesktop.Notifications",
  "path": "/org/freedesktop/Notifications",
  "interface": "org.freedesktop.Notifications",
  "method": "GetCapabilities"
}'
```

```json
{
  "type": "d_bus.call",
  "bus": "session",
  "service": "org.freedesktop.Notifications",
  "path": "/org/freedesktop/Notifications",
  "interface": "org.freedesktop.Notifications",
  "method": "GetCapabilities"
}
```

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "result": ["action-icons", "actions", "body", "body-hyperlinks", "body-images"]
  }
}
```

## Python Example

```python
from deskbrid import Deskbrid
client = Deskbrid()

result = client.dbus_call(
    bus="session",
    service="org.freedesktop.Notifications",
    path="/org/freedesktop/Notifications",
    interface="org.freedesktop.Notifications",
    method="GetCapabilities",
)
print(result["result"])
```

## Safety

D-Bus calls bypass most of Deskbrid's safety layer — use with caution.
Some methods can perform destructive system operations. Consider adding
rate limits and confirmation requirements for `d_bus.*` actions.

## Requirements

- Requires `dbus` or `zbus` on the system
- Works with both system and session buses

## Current Status

**Stable** — arbitrary D-Bus method calls supported.
