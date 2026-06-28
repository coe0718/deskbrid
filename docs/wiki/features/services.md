# Services

Systemd service, timer, and journal control.

> **Note:** The canonical documentation for service management is in
> [systemd_units_journal_and_timers.md](systemd_units_journal_and_timers.md).
> This page is a convenience alias.

## Service management

```bash
deskbrid service.list {}
deskbrid service.status { name: "nginx.service" }
deskbrid service.start { name: "nginx.service" }
deskbrid service.stop { name: "nginx.service" }
deskbrid service.restart { name: "nginx.service" }
deskbrid service.enable { name: "nginx.service", runtime: false }
deskbrid service.disable { name: "nginx.service" }
```

## Timers

```bash
deskbrid timer.list {}
deskbrid timer.start { name: "daily-apt.timer" }
deskbrid timer.stop { name: "daily-apt.timer" }
```

## Journal

```bash
deskbrid journal.query {
  since: 3600,
  unit: "nginx.service",
  tail: 100
}
```

## Python example

```python
from deskbrid import Deskbrid
client = Deskbrid()
status = client.service_status("nginx")
print(status["status"])
```
