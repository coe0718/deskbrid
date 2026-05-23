# Services

Manage systemd services.

## List Services

```bash
deskbrid services list
deskbrid services list --type running
deskbrid services list --type enabled
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {
      "name": "nginx.service",
      "status": "active",
      "enabled": true,
      "type": "service"
    }
  ]
}
```

Protocol:
```json
{"action": "service.list", "unit_type": "service"}
```

Unit types:
- `service` - Regular services
- `socket` - Socket-activated services
- `timer` - Scheduled timers
- `all` - All unit types

## Service Status

```bash
deskbrid services status nginx
```

Protocol:
```json
{"action": "service.status", "name": "nginx.service"}
```

## Start/Stop/Restart

```bash
deskbrid services start nginx
deskbrid services stop nginx
deskbrid services restart nginx
```

Protocol:
```json
{"action": "service.start", "name": "nginx.service"}
```

## Enable/Disable

```bash
deskbrid services enable nginx
deskbrid services disable nginx
deskbrid services enable --runtime nginx  # Only for this boot
```

Protocol:
```json
{"action": "service.enable", "name": "nginx.service", "runtime": false}
```

## Timers

```bash
deskbrid services timers                    # List all timers
deskbrid services timer start daily-apt     # Start a timer
deskbrid services timer stop daily-apt      # Stop a timer
```

Protocol:
```json
{"action": "timer.start", "name": "daily-apt.timer"}
```

## Journal Query

```bash
deskbrid journal query --since 1h --unit nginx --tail 100
deskbrid journal query --since 2024-01-01 --priority 3  # Errors only
```

Protocol:
```json
{
  "action": "journal.query",
  "since": 3600,
  "unit": "nginx.service",
  "tail": 100
}
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# Check nginx status
status = client.service_status("nginx")
print(f"nginx: {status['status']}")

# Get recent logs
logs = client.journal_query(unit="nginx", tail=50)
for line in logs:
    print(line)
```