# System Info

Query system status, power state, security/inhibition helpers, and session
information.

## System info

```bash
deskbrid system.info
```

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "desktop": "gnome",
    "desktop_version": "45.0",
    "compositor": "gnome-shell",
    "session_type": "wayland",
    "monitors": [
      {"id": 0, "name": "DP-1", "width": 1920, "height": 1080, "scale": 1.0, "primary": true}
    ],
    "workspace_count": 4,
    "current_workspace": 0,
    "idle_seconds": 300
  }
}
```

## Capabilities

```bash
deskbrid system.capabilities
```

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "screenshot": true,
    "ocr": true,
    "clipboard": true,
    "input": true,
    "notifications": true,
    "mpris": true
  }
}
```

## Power management

```bash
deskbrid system.power { action: "suspend" }
deskbrid system.power { action: "reboot" }
deskbrid system.power { action: "shutdown" }
deskbrid system.power { action: "lock" }
```

Valid `action` values: `suspend`, `hibernate`, `reboot`, `shutdown`, `lock`.

## Idle detection

```bash
deskbrid system.idle
```

Response:

```json
{"type":"response","status":"ok","data":{"idle_ms":300000}}
```

## Inhibit / release_inhibit

```bash
deskbrid system.inhibit {
  what: "suspend",
  who: "backup-script",
  why: "long-running backup"
}
deskbrid system.release_inhibit { inhibitor_id: 42 }
```

## Session helpers

```bash
deskbrid system.sessions
deskbrid system.lock_session {}
deskbrid system.switch_user { username: "alice" }
```

## Auth probes

```bash
deskbrid auth.check { action_type: "system.power" }
deskbrid auth.prompt {
  action_type: "system.power",
  action_params: { action: "shutdown" },
  reason: "User shutdown via agent"
}
```

`auth.check` returns `authorized`, `required`, and `method`. `auth.prompt`
dispatches the configured confirmation flow (dashboard challenge, policy
allowlist, timeout).

## Confinement

```bash
deskbrid system.confinement {}
```
