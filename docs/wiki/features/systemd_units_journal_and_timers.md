# Systemd Units, Journal, and Timers

Manage systemd services, query journald logs, and control systemd timers directly from the AI agent. Use this feature to inspect service health, restart failing units, retrieve boot or application logs for debugging, and schedule or manage timer-driven tasks.

## Actions

### service.status

Get the current status of a systemd unit including active state, enabled state, PID, memory usage, and recent log entries.

| Parameter | Type | Description |
|-----------|------|-------------|
| `name` | string | The systemd unit name (e.g. `nginx.service`, `sshd`) |

```bash
deskbrid service.status '{ "name": "nginx.service" }'
```

```json
{"type": "service.status", "name": "nginx.service"}
```

**Response:** Returns an object with `active` (bool), `enabled` (bool), `pid` (int|null), `load_state`, `active_state`, `sub_state`, `memory_current` (bytes), and `recent_logs` (array of recent journal entries).

### service.start

Start a systemd unit. Equivalent to `systemctl start <name>`.

| Parameter | Type | Description |
|-----------|------|-------------|
| `name` | string | The systemd unit name to start |

```bash
deskbrid service.start '{ "name": "postgresql.service" }'
```

```json
{"type": "service.start", "name": "postgresql.service"}
```

**Response:** Returns `{"success": true}` on success, or an error message if the unit could not be started (e.g. unit not found, insufficient permissions).

### service.stop

Stop a running systemd unit. Equivalent to `systemctl stop <name>`.

| Parameter | Type | Description |
|-----------|------|-------------|
| `name` | string | The systemd unit name to stop |

```bash
deskbrid service.stop '{ "name": "nginx.service" }'
```

```json
{"type": "service.stop", "name": "nginx.service"}
```

**Response:** Returns `{"success": true}` on success.

### service.restart

Restart a systemd unit. Equivalent to `systemctl restart <name>`.

| Parameter | Type | Description |
|-----------|------|-------------|
| `name` | string | The systemd unit name to restart |

```bash
deskbrid service.restart '{ "name": "nginx.service" }'
```

```json
{"type": "service.restart", "name": "nginx.service"}
```

**Response:** Returns `{"success": true}` on success.

### service.enable

Enable a systemd unit so it starts automatically at boot. Equivalent to `systemctl enable <name>`.

| Parameter | Type | Description |
|-----------|------|-------------|
| `name` | string | The systemd unit name to enable |

```bash
deskbrid service.enable '{ "name": "docker.service" }'
```

```json
{"type": "service.enable", "name": "docker.service"}
```

**Response:** Returns `{"success": true}` on success.

### service.disable

Disable a systemd unit so it no longer starts at boot. Equivalent to `systemctl disable <name>`.

| Parameter | Type | Description |
|-----------|------|-------------|
| `name` | string | The systemd unit name to disable |

```bash
deskbrid service.disable '{ "name": "docker.service" }'
```

```json
{"type": "service.disable", "name": "docker.service"}
```

**Response:** Returns `{"success": true}` on success.

### service.list

List systemd units filtered by type and/or state. Equivalent to `systemctl list-units --type=<type> --state=<state>`.

| Parameter | Type | Description |
|-----------|------|-------------|
| `type` | string | *Optional.* Unit type filter: `service`, `timer`, `socket`, `mount`, `path`, etc. |
| `state` | string | *Optional.* Unit state filter: `active`, `inactive`, `failed`, `running`, `exited`, etc. |

```bash
deskbrid service.list '{ "type": "service", "state": "running" }'
```

```json
{"type": "service.list", "type": "service", "state": "running"}
```

**Response:** Returns an array of unit objects, each with `name`, `load_state`, `active_state`, `sub_state`, `description`, and `following`.

### journal.query

Query the systemd journal for log entries with flexible filtering.

| Parameter | Type | Description |
|-----------|------|-------------|
| `since` | string | *Optional.* Start timestamp (e.g. `"2025-01-01"`, `"1 hour ago"`, `"yesterday"`). Parsed by systemd's time spec. |
| `until` | string | *Optional.* End timestamp in the same format as `since`. |
| `unit` | string | *Optional.* Filter to a specific systemd unit (e.g. `"sshd"`, `"nginx.service"`). |
| `priority` | uint | *Optional.* Log priority filter (0=emerg, 1=alert, 2=crit, 3=err, 4=warning, 5=notice, 6=info, 7=debug). Only entries at this level or higher priority are returned. |
| `limit` | uint | *Optional.* Maximum number of entries to return (default: 50). |
| `follow` | bool | *Optional.* If true, stream new journal entries as they arrive. Requires a persistent connection (WebSocket/long-poll). Default: false. |

```bash
deskbrid journal.query '{ "unit": "sshd", "priority": 3, "limit": 10, "since": "2 hours ago" }'
```

```json
{"type": "journal.query", "unit": "sshd", "priority": 3, "limit": 10, "since": "2 hours ago"}
```

**Response:** Returns an array of journal entry objects, each with `timestamp` (ISO 8601), `message`, `priority` (int), `unit`, `pid`, and optional fields like `comm`, `uid`, `gid`, `boot_id`.

### timer.list

List all active systemd timers with their next and last trigger times. Equivalent to `systemctl list-timers`.

| Parameter | Type | Description |
|-----------|------|-------------|
| *None* | | This action takes no parameters. |

```bash
deskbrid timer.list '{}'
```

```json
{"type": "timer.list"}
```

**Response:** Returns an array of timer objects, each with `name`, `next` (ISO timestamp), `left` (human-readable duration), `last` (ISO timestamp), `passed` (human-readable duration), `unit` (the service unit the timer activates), and `activates`.

### timer.start

Start a systemd timer immediately. Equivalent to `systemctl start <name>`.

| Parameter | Type | Description |
|-----------|------|-------------|
| `name` | string | The timer unit name (e.g. `fstrim.timer`) |

```bash
deskbrid timer.start '{ "name": "fstrim.timer" }'
```

```json
{"type": "timer.start", "name": "fstrim.timer"}
```

**Response:** Returns `{"success": true}` on success.

### timer.stop

Stop a systemd timer. Equivalent to `systemctl stop <name>`.

| Parameter | Type | Description |
|-----------|------|-------------|
| `name` | string | The timer unit name (e.g. `fstrim.timer`) |

```bash
deskbrid timer.stop '{ "name": "fstrim.timer" }'
```

```json
{"type": "timer.stop", "name": "fstrim.timer"}
```

**Response:** Returns `{"success": true}` on success.

## Safety Boundary

- Starting, stopping, enabling, or disabling services and timers requires **elevated permissions** (user's systemd bus access). The daemon must either run as root or the user must have sudo/PolKit privileges for the target unit.
- Querying the journal requires read access to `/var/log/journal/` or the user's own journal logs.
- Destructive actions (`stop`, `disable`) are subject to Deskbrid's **confirmation mode** — the agent must receive explicit user approval before execution when confirmation mode is enabled.

## Local Development

1. Ensure Deskbrid daemon is running: `deskbrid daemon`
2. Test queries with a safe non-critical unit:
   ```bash
   deskbrid service.status '{ "name": "systemd-journald.service" }'
   ```
3. Test journal queries:
   ```bash
   deskbrid journal.query '{ "limit": 5 }'
   ```
4. Test timer listing:
   ```bash
   deskbrid timer.list '{}'
   ```
5. Test destructive actions against a test unit you created, or with confirmation mode disabled for development.

## Configuration

No additional configuration is needed. All actions communicate over the systemd D-Bus API (`org.freedesktop.systemd1`) and the journal using `sd-journal` APIs. Permission scoping is handled by the user's PolKit and systemd user bus configuration.
