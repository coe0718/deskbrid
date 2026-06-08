# Protocol Overview

## Transport

Deskbrid communicates over a Unix domain socket:

```
/run/user/<UID>/deskbrid.sock
```

Clients send JSON commands terminated by newline, then read newline-delimited JSON responses.

### Raw socket shape

```json
{"type":"<action>", "<extra fields vary by action>"}
```

All successful responses share the envelope:

```json
{"type":"response","request_id":"...","status":"ok","data":{...}}
```

Error responses use:

```json
{"type":"response","request_id":"...","status":"error","error":{"code":"<code>","message":"<human text>"}}
```

> The project uses **dot-notation action strings** for runtime subscription and reply `type` fields, but the daemon does **not** semantically dispatch `event.subscribe` UI actions through the same path.

## Dispatch rules

- **socket domain** — actions like `windows.list`, `clipboard.read`, `system.info`
- **UI/Auth domain** — `confirm.challenge`, `confirm.resolve`, `auth.elevate`
- **event subscription** — `event.subscribe`, `event.unsubscribe`

### UI action flow (v1.0.0)

1. Send `confirm.challenge` with a challenge ID / prompt.
2. Present the challenge in the Dashboard / desktop UI.
3. Send `confirm.resolve` with approval or rejection.
4. For elevated actions, send `auth.elevate` with the action ID, reason, and confirmation.

### Auth rule (v1.0.0)

System-level `system.*` actions that mutate system state require authorization by default. The safer route is via the UI/`confirm.challenge` flow rather than by passing secrets over the socket.

## Error codes

| code | when |
|---|---|
| `invalid_params` | missing or invalid parameters |
| `not_found` | window / session / record missing |
| `permission_denied` | policy or auth check failed |
| `not_supported` | backend/desktop doesn’t support it |
| `backend_error` | compositor/backend error |
| `internal_error` | daemon error |

## Event subscription

Subscribe with wildcard patterns:

```json
{"type":"event.subscribe","events":["window.*"]}
```

Unsubscribe:

```json
{"type":"event.unsubscribe","events":["window.*"]}
```

Events are newline-delimited JSON messages from the daemon:

```json
{"type":"event","event":"window.focused","data":{"window_id":"...","app_id":"org.gnome.Terminal"},"timestamp":"..."}
```

## Python example

```python
import socket, json

sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
sock.connect("/run/user/1000/deskbrid.sock")

sock.send(b'{"type":"windows.list"}\n')
sock.send(b'{"type":"event.subscribe","events":["window.*"]}\n')
```
