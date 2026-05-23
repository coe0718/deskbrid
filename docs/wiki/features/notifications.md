# Notifications

Send desktop notifications.

## Send Notification

```bash
deskbrid notify "Build Complete" "All tests passed!"
deskbrid notify "Error" "Build failed" --urgency critical
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "notification_id": 42
  }
}
```

Protocol:
```json
{
  "action": "notification.send",
  "app_name": "deskbrid",
  "title": "Build Complete",
  "body": "All tests passed!",
  "urgency": "normal"
}
```

Urgency levels:
- `low` - Passive notification
- `normal` - Standard notification (default)
- `critical` - Urgent, bypasses Do Not Disturb

## Close Notification

```bash
deskbrid notify close 42
```

Protocol:
```json
{"action": "notification.close", "notification_id": 42}
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# Send info notification
client.notify("Task Started", "Processing files...")

# Send error notification
client.notify("Error", "Failed to process file", urgency="critical")
```

## AI Agent Example

```json
→ {"action": "notification.send", "app_name": "agent", "title": "Task Complete", "body": "PR created successfully"}
← {"type": "response", "status": "ok", "data": {"notification_id": 5}}
```