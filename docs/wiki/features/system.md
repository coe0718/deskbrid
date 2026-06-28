# System

Query and control system-level state: platform info, power management, battery,
backlight, hardware pressure, CPU frequency/governor, print services, idle
detection, session management, inhibit locks, authentication elevation, and
self-update.

## Actions

### system.info

Return platform identity, compositor, session type, monitors, workspaces, and
idle time in a single response.

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

### system.health

Run subsystem health checks and return a status map. Useful before dispatching
dependent actions.

```bash
deskbrid system.health
```

No parameters.

### system.capabilities

List which subsystems are available on the current desktop environment.

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

### system.confinement

Return the confinement profile Kerning the agent — whether it's sandboxed
(Flatpak, Snap, container) and which desktop portal capabilities are restricted.

```bash
deskbrid system.confinement
```

No parameters.

### system.remediate

Run a pre-flight health check and optionally apply fixes.

| Parameter | Type   | Description                                           |
|-----------|--------|-------------------------------------------------------|
| `check`   | string | The health check to run (e.g. `"clipboard"`, `"uinput"`) |
| `apply`   | bool   | If true, attempt automatic remediation                 |

```bash
deskbrid system.remediate { check: "uinput", apply: true }
```

### system.normalize_coords

Convert coordinates that may use fractional scaling or mixed-monitor layouts
into absolute pixel coordinates suitable for mouse and screenshot operations.

| Parameter | Type     | Description                            |
|-----------|----------|----------------------------------------|
| `x`       | float    | X coordinate (logical)                 |
| `y`       | float    | Y coordinate (logical)                 |
| `monitor` | uint?    | Target monitor ID (optional)            |

```bash
deskbrid system.normalize_coords { x: 1920.5, y: 1080.0, monitor: 0 }
```

### system.idle

Return the current idle time in milliseconds.

```bash
deskbrid system.idle
```

Response:

```json
{"type": "response", "status": "ok", "data": {"idle_ms": 300000}}
```

No parameters.

### system.power

Trigger a power-state transition.

| Parameter | Type   | Description                                |
|-----------|--------|--------------------------------------------|
| `action`  | string | One of: `suspend`, `hibernate`, `reboot`, `shutdown`, `lock` |

```bash
deskbrid system.power { action: "suspend" }
deskbrid system.power { action: "reboot" }
deskbrid system.power { action: "shutdown" }
deskbrid system.power { action: "lock" }
```

### system.battery

Return battery status for all connected batteries (or an empty array on a
desktop without a battery).

```bash
deskbrid system.battery
```

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {"name": "BAT0", "percentage": 85.0, "status": "Charging", "time_remaining": 1800}
  ]
}
```

No parameters.

### system.backlight_list

List backlight devices available through sysfs (usually in `/sys/class/backlight/`).

```bash
deskbrid system.backlight_list
```

No parameters.

### system.backlight_get

Get the current brightness for a backlight device. If `device` is omitted,
returns the first (usually primary) backlight.

| Parameter | Type   | Description                |
|-----------|--------|----------------------------|
| `device`  | string?| Backlight device name (optional) |

```bash
deskbrid system.backlight_get { device: "intel_backlight" }
```

### system.backlight_set

Set the brightness for a backlight device.

| Parameter | Type   | Description                          |
|-----------|--------|--------------------------------------|
| `device`  | string?| Backlight device name (optional)      |
| `value`   | string | Brightness value; absolute number or percentage (e.g. `"50%"`) |

```bash
deskbrid system.backlight_set { device: "intel_backlight", value: "75%" }
```

### system.print_list

List installed printers on the system (via CUPS).

```bash
deskbrid system.print_list
```

No parameters.

### system.print_default

Set or query the default printer.

| Parameter | Type   | Description                          |
|-----------|--------|--------------------------------------|
| `printer` | string?| Printer name to set as default (omit to query) |

```bash
deskbrid system.print_default { printer: "Brother-HL-L2370DW" }
```

### system.print_file

Send a file to a printer.

| Parameter | Type   | Description       |
|-----------|--------|-------------------|
| `printer` | string | Target printer name |
| `path`    | string | Absolute path to the file |

```bash
deskbrid system.print_file { printer: "Brother-HL-L2370DW", path: "/tmp/report.pdf" }
```

### system.print_job_list

List active and recent print jobs.

```bash
deskbrid system.print_job_list
```

No parameters.

### system.print_job_cancel

Cancel a print job by ID.

| Parameter | Type   | Description   |
|-----------|--------|---------------|
| `job_id`  | string | Job identifier |

```bash
deskbrid system.print_job_cancel { job_id: "Brother-42" }
```

### system.print_job_pause

Pause a print job.

| Parameter | Type   | Description   |
|-----------|--------|---------------|
| `job_id`  | string | Job identifier |

```bash
deskbrid system.print_job_pause { job_id: "Brother-42" }
```

### system.print_job_resume

Resume a paused print job.

| Parameter | Type   | Description   |
|-----------|--------|---------------|
| `job_id`  | string | Job identifier |

```bash
deskbrid system.print_job_resume { job_id: "Brother-42" }
```

### system.pressure

Return Pressure Stall Information (PSI) — CPU, memory, and I/O pressure
metrics. Useful for detecting system overload.

```bash
deskbrid system.pressure
```

No parameters.

### system.thermal_get

Return thermal zone temperatures from the kernel.

```bash
deskbrid system.thermal_get
```

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {"zone": 0, "type": "x86_pkg_temp", "temp_c": 52.0}
  ]
}
```

No parameters.

### system.cpu_frequency

Return current CPU frequencies for all cores.

```bash
deskbrid system.cpu_frequency
```

No parameters.

### system.cpu_governor

Return the current CPU frequency governor (e.g. `powersave`, `performance`).

```bash
deskbrid system.cpu_governor
```

No parameters.

### system.cpu_set_governor

Set the CPU frequency governor.

| Parameter | Type   | Description                                      |
|-----------|--------|--------------------------------------------------|
| `governor`| string | Governor name: `performance`, `powersave`, `ondemand`, `conservative`, `schedutil` |

```bash
deskbrid system.cpu_set_governor { governor: "performance" }
```

### system.inhibit

Acquire a system inhibit lock to prevent idle suspension, screen blanking, or
power transitions while a background task runs.

| Parameter | Type   | Description                                      |
|-----------|--------|--------------------------------------------------|
| `what`    | string | Inhibit category: `suspend`, `idle`, `screen`     |
| `who`     | string | Application/inhibitor name (shown in inhibitors UI) |
| `why`     | string?| Human-readable reason (optional)                  |
| `mode`    | string?| `"block"` (default) or `"delay"` (optional)       |

```bash
deskbrid system.inhibit {
  what: "suspend",
  who: "backup-script",
  why: "long-running backup"
}
```

### system.release_inhibit

Release an inhibit lock acquired earlier.

| Parameter      | Type | Description     |
|----------------|------|-----------------|
| `inhibitor_id` | uint | Cookie returned by `system.inhibit` |

```bash
deskbrid system.release_inhibit { inhibitor_id: 42 }
```

### system.sessions

List active login sessions.

```bash
deskbrid system.sessions
```

```json
{"type": "system.sessions"}
```

No parameters.

### system.lock_session

Lock the current session, or a specific session by ID.

| Parameter    | Type    | Description                          |
|--------------|---------|--------------------------------------|
| `session_id` | string? | Logind session ID (omit for current) |

```bash
deskbrid system.lock_session { session_id: "c2" }
```

### system.switch_user

Switch to another user's session via fast-user-switching.

| Parameter  | Type   | Description           |
|------------|--------|-----------------------|
| `username` | string | Target user to switch to |

```bash
deskbrid system.switch_user { username: "alice" }
```

### system.check_auth

Check whether a specific action type would require authentication under the
current confirmation mode configuration.

| Parameter   | Type   | Description                       |
|-------------|--------|-----------------------------------|
| `action_id` | string | Dot‑notation action name, e.g. `"system.power"` |

```bash
deskbrid system.check_auth { action_id: "system.power" }
```

### system.elevate

Request elevation / confirmation for a privileged action. Provides input to the
confirmation mode workflow.

| Parameter   | Type    | Description                           |
|-------------|---------|---------------------------------------|
| `action_id` | string  | Dot‑notation action name               |
| `reason`    | string? | Optional human-readable reason         |

```bash
deskbrid system.elevate {
  action_id: "system.power",
  reason: "User shutdown via agent"
}
```

### system.update

Check for and apply Deskbrid updates.

| Parameter | Type | Description                             |
|-----------|------|-----------------------------------------|
| `check`   | bool | If true, only check — do not apply      |
| `force`   | bool | If true, force re-download even if up-to-date |

```bash
deskbrid system.update { check: true, force: false }
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# System info
info = client.system_info()
print(info["desktop"], info["session_type"])

# Battery status
batteries = client.system_battery()
for b in batteries:
    print(b["name"], b["percentage"], b["status"])

# Backlight
client.system_backlight_set(device="intel_backlight", value="50%")

# Inhibit while doing work
inhibitor = client.system_inhibit(what="suspend", who="my-script", why="busy")
# ... do work ...
client.system_release_inhibit(inhibitor_id=inhibitor["id"])

# Power management
client.system_power(action="suspend")
```

## Requirements

- Power management requires logind (`systemd-logind`).
- Backlight control requires read/write access to `/sys/class/backlight/` (usually
  in the `video` group).
- Print actions require a running CUPS daemon and the `cups` CLI installed.
- PSI pressure metrics read from `/proc/pressure/` (kernel ≥ 4.20).
- CPU frequency/governor control requires `cpufreq` kernel support and
  appropriate permissions.
- `system.update` downloads from GitHub releases — requires network access.

## Rate Limits

System health and info queries are typically unbounded. Mutating actions (power,
inhibit, update) may be rate-limited per the agent's `permissions.toml`.

## Current Status

**Stable** — system.info, capabilities, power, idle, battery, backlight.
**Experimental** — print, pressure, thermal, CPU frequency/governor, session
management, update.
