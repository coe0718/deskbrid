# Cron

Deskbrid v1.0.0 includes a cron engine for scheduling actions. The schema uses
`rules.create` with `cron` triggers, not a separate `cron.*` namespace.

## Schedule events with cron

Use cron expressions as the rule trigger:

```bash
deskbrid rules.create {
  name: "periodic-system-info",
  trigger: "0 * * * *",
  action_type: "system.info",
  action_params: {},
  enabled: true
}
```

You can also define schedules via `schedule.json` when running the daemon with
the cron scheduler enabled.

## List schedules (runtime queue)

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
        "name": "hourly-screenshot",
        "interval_secs": 3600,
        "action_type": "screenshot",
        "action_params": { "output_path": "/tmp/screen.png" },
        "enabled": true,
        "last_run": "2026-05-30T10:00:00Z",
        "next_run": "2026-05-30T11:00:00Z"
      }
    ]
  }
}
```

## Add a scheduled job

```bash
deskbrid schedule.add {
  name: "daily-backup",
  interval_secs: 86400,
  action_type: "windows.list",
  action_params: {},
  enabled: true
}
```

## Remove a scheduled job

```bash
deskbrid schedule.remove { name: "daily-backup" }
```

## Python example

```python
from deskbrid import Deskbrid
client = Deskbrid()

client.schedule_add(
    name="hourly-check",
    interval_secs=3600,
    action_type="system.info",
    action_params={},
    enabled=True,
)
```

## Notes

- Mutating scheduler state is independent of `rules.*` management.
- Some desktop environments may throttle timers when the session is idle.
- The scheduler runs inside the daemon; if the daemon stops, scheduled actions
  do not fire.
