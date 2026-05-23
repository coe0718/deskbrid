# System Information

Query system status, power information, and control system functions.

## System Info

```bash
deskbrid system info
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "hostname": "mycomputer",
    "os": "linux",
    "desktop": "gnome",
    "shell": "/bin/bash",
    "uptime": 86400,
    "load_avg": [0.5, 0.3, 0.2]
  }
}
```

Protocol:
```json
{"action": "system.info"}
```

## System Capabilities

```bash
deskbrid system capabilities
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

Protocol:
```json
{"action": "system.capabilities"}
```

## System Health

Check system health and diagnose issues:

```bash
deskbrid system health
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "healthy": true,
    "checks": [
      {"name": "socket", "status": "ok"},
      {"name": "permissions", "status": "ok"},
      {"name": "backends", "status": "ok"}
    ]
  }
}
```

Protocol:
```json
{"action": "system.health"}
```

## Power Management

### Power Actions

```bash
deskbrid system power suspend
deskbrid system power reboot
deskbrid system power shutdown
```

Protocol:
```json
{"action": "system.power", "action": "suspend"}
```

Actions:
- `suspend` - Suspend to RAM
- `hibernate` - Suspend to disk
- `reboot` - Reboot system
- `shutdown` - Power off
- `lock` - Lock screen

## Battery Status

```bash
deskbrid system battery
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "percentage": 85,
    "charging": true,
    "time_remaining": 3600
  }
}
```

Protocol:
```json
{"action": "system.battery"}
```

## Idle Detection

Check how long the system has been idle:

```bash
deskbrid system idle
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "idle_ms": 300000,
    "idle": true
  }
}
```

Protocol:
```json
{"action": "system.idle"}
```

## Inhibit System

Prevent system sleep, screensaver, or session lock:

```bash
deskbrid system inhibit suspend --who "backup-script" --why "long-running backup"
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "inhibitor_id": 42
  }
}
```

Protocol:
```json
{
  "action": "system.inhibit",
  "what": "suspend",
  "who": "backup-script",
  "why": "long-running backup"
}
```

What to inhibit:
- `suspend` - Prevent system suspend
- `sleep` - Prevent system sleep
- `idle` - Prevent idle activation
- `logout` - Prevent automatic logout

Modes:
- `block` - Hard block (default)
- `delay` - Delay for a time
- `transient` - Until next session

### Release Inhibit

```bash
deskbrid system release-inhibit 42
```

Protocol:
```json
{"action": "system.release_inhibit", "inhibitor_id": 42}
```

## Session Management

### List Sessions

```bash
deskbrid system sessions
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {
      "session_id": "1",
      "username": "user",
      "seat": "seat0",
      "active": true
    }
  ]
}
```

### Lock Session

```bash
deskbrid system lock-session
deskbrid system lock-session --session 2
```

Protocol:
```json
{"action": "system.lock_session"}
```

### Switch User

```bash
deskbrid system switch-user alice
```

Protocol:
```json
{"action": "system.switch_user", "username": "alice"}
```

## Privilege Escalation

### Check Auth

Check if an action requires authorization:

```bash
deskbrid system check-auth system.power
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "authorized": false,
    "action_id": "system.power"
  }
}
```

### Elevate Privileges

Request privilege elevation:

```bash
deskbrid system elevate system.power --reason "User requested shutdown"
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "authorized": true
  }
}
```

## Confinement Status

Check if running in a sandbox (Flatpak, Snap):

```bash
deskbrid system confinement
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "confined": true,
    "type": "flatpak",
    "sandbox": true
  }
}
```

## Remediate Issues

Fix common problems:

```bash
deskbrid system remediate --check socket
deskbrid system remediate --check permissions --apply
```

Protocol:
```json
{"action": "system.remediate", "check": "socket", "apply": true}
```

## Normalize Coordinates

Convert pixel coordinates between monitor layouts:

```bash
deskbrid system normalize-coords --x 1000 --y 500
```

Protocol:
```json
{"action": "system.normalize_coords", "x": 1000, "y": 500}
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# Get system status
info = client.info()
print(f"Desktop: {info.desktop}, Uptime: {info.uptime}s")

# Check battery
battery = client.system_battery()
print(f"Battery: {battery['percentage']}%")

# Prevent sleep during long operation
inhibit = client.inhibit_system("suspend", who="backup", why="backup running")
try:
    # ... long running task ...
    pass
finally:
    client.release_inhibit(inhibit["inhibitor_id"])
```