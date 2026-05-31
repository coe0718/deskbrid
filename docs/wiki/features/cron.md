# Cron

Deskbrid's cron engine allows you to schedule actions to run at regular intervals. The cron engine reads from `~/.config/deskbrid/schedule.json` and polls every 60 seconds for scheduled actions.

## Overview

Scheduled actions are dispatched through the same pipeline as socket requests, meaning they have full access to all deskbrid capabilities. Each scheduled action has:
- A name for identification
- An interval in seconds
- An action type and parameters
- Optional enabled/disabled state

## Schedule File

The schedule is stored in `~/.config/deskbrid/schedule.json` as a JSON array of scheduled actions:

```json
[
  {
    "name": "daily-backup",
    "interval_secs": 86400,
    "action_type": "windows.list",
    "action_params": {},
    "enabled": true
  }
]
```

## Listing Scheduled Actions

```bash
deskbrid schedule.list
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "schedules": [
      {
        "name": "daily-backup",
        "interval_secs": 86400,
        "action_type": "windows.list",
        "action_params": {},
        "enabled": true,
        "last_run": "2026-05-29T02:00:00Z",
        "next_run": "2026-05-30T02:00:00Z"
      },
      {
        "name": "clipboard-history-cleanup",
        "interval_secs": 3600,
        "action_type": "clipboard.history.clear",
        "action_params": {},
        "enabled": true,
        "last_run": "2026-05-30T09:30:00Z",
        "next_run": "2026-05-30T10:30:00Z"
      }
    ]
  }
}
```

## Adding a Scheduled Action

```bash
deskbrid schedule.add { 
  name: "hourly-screenshot", 
  interval_secs: 3600, 
  action_type: "screenshot", 
  action_params: { output_path: "/tmp/screenshot_%Y%m%d_%H%M%S.png" }, 
  enabled: true 
}
```

### Parameters

- `name`: Identifier for the scheduled action (required)
- `interval_secs`: Interval in seconds between executions (required)
- `action_type`: The action to execute (see [Protocol Overview](Protocol-Overview))
- `action_params`: Parameters for the action (JSON object, optional)
- `enabled`: Boolean to enable/disable the scheduled action (default: true)

## Removing a Scheduled Action

```bash
deskbrid schedule.remove { name: "hourly-screenshot" }
```

## Examples

### Daily Window List Backup

```bash
deskbrid schedule.add {
  name: "daily-windows-backup",
  interval_secs: 86400,
  action_type: "windows.list",
  action_params: {},
  enabled: true
}
```

This will run `windows.list` once per day and store the result (though currently the result is not saved - you'd need to modify this to save to a file or send elsewhere).

### Hourly Clipboard History Cleanup

```bash
deskbrid schedule.add {
  name: "hourly-clipboard-cleanup",
  interval_secs: 3600,
  action_type: "clipboard.history.clear",
  action_params: {},
  enabled: true
}
```

This clears the clipboard history every hour.

### Periodic System Info Logging

```bash
deskbrid schedule.add {
  name: "periodic-system-info",
  interval_secs: 300,  # Every 5 minutes
  action_type: "system.info",
  action_params: {},
  enabled: true
}
```

### Regular Screenshot Capture

```bash
deskbrid schedule.add {
  name: "regular-screenshot",
  interval_secs: 1800,  # Every 30 minutes
  action_type: "screenshot",
  action_params: { 
    output_path: "/home/user/screenshots/deskbrid_%Y%m%d_%H%M%S.png" 
  },
  enabled: true
}
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# List schedules
schedules = client.schedule_list()
for schedule in schedules['schedules']:
    print(f"{schedule['name']}: every {schedule['interval_secs']}s ({'enabled' if schedule['enabled'] else 'disabled'})")

# Add a schedule
client.schedule_add(
    name="periodic-notify",
    interval_secs=3600,
    action_type="notification.send",
    action_params={"title": "Hourly", "body": "This is an hourly reminder"},
    enabled=True
)

# Remove a schedule
client.schedule_remove(name="periodic-notify")
```