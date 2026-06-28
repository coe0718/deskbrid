# Notifications

Send, close, and manage desktop notifications. Supports urgency levels,
notification actions, history, and watching for incoming notifications.

## Actions

### notification.send

Send a desktop notification.

| Parameter       | Type   | Description                                      |
|-----------------|--------|--------------------------------------------------|
| `app_name`      | string | Source application name                          |
| `title`         | string | Notification title                               |
| `body`          | string | Notification body text                           |
| `urgency`       | string | One of: `low`, `normal`, `critical`              |

```bash
deskbrid notify "Build Complete" "All tests passed!"
deskbrid notify "Error" "Build failed" --urgency critical
```

```json
{
  "type": "notification.send",
  "app_name": "deskbrid",
  "title": "Build Complete",
  "body": "All tests passed!",
  "urgency": "normal"
}
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

### notification.close

Close a previously sent notification by ID.

| Parameter         | Type   | Description          |
|-------------------|--------|----------------------|
| `notification_id` | uint   | Notification ID      |

```bash
deskbrid notify close 42
```

```json
{
  "type": "notification.close",
  "notification_id": 42
}
```

### notification.history

Retrieve notification history.

| Parameter  | Type    | Description                    |
|------------|---------|--------------------------------|
| `limit`    | uint?   | Max entries (default: 20)      |
| `app_name` | string? | Filter by source app name      |
| `since`    | uint?   | Unix timestamp, only show notifications after |

```bash
deskbrid notification.history '{"limit": 10, "app_name": "deskbrid"}'
```

```json
{
  "type": "notification.history",
  "limit": 10,
  "app_name": "deskbrid"
}
```

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {"id": 42, "app_name": "deskbrid", "title": "Build Complete", "body": "All tests passed!", "urgency": "normal", "timestamp": 1705312800},
    {"id": 41, "app_name": "deskbrid", "title": "Task Started", "body": "Processing...", "urgency": "low", "timestamp": 1705312700}
  ]
}
```

### notification.action

Trigger an action button on a notification. Notifications can include action
buttons (e.g., "Open", "Dismiss"). This action activates one of those buttons.

| Parameter         | Type   | Description                    |
|-------------------|--------|--------------------------------|
| `notification_id` | uint   | Notification ID                |
| `action_key`      | string | Action key to trigger          |

```bash
deskbrid notification.action '{"notification_id": 42, "action_key": "open"}'
```

```json
{
  "type": "notification.action",
  "notification_id": 42,
  "action_key": "open"
}
```

### notification.clear_history

Clear all notification history.

```bash
deskbrid notification.clear_history
```

No parameters.

### notification.watch

Subscribe to incoming notifications in real-time. Returns a stream of
notification events as they occur.

```bash
deskbrid notification.watch
```

No parameters (streaming action — runs until cancelled).

## Rate Limits

Notification sends are rate-limited per `permissions.toml`:

```toml
[rate_limits.notifications]
rpm = 60
burst = 10
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# Send info notification
client.notification_send(
    app_name="deskbrid",
    title="Task Started",
    body="Processing files...",
    urgency="normal"
)

# Send error notification
client.notification_send(
    app_name="deskbrid",
    title="Error",
    body="Failed to process file",
    urgency="critical"
)

# View history
history = client.notification_history(limit=5)
for n in history:
    print(f"[{n['id']}] {n['title']}: {n['body']}")
```

## Requirements

- **GNOME**: Notifications go through `org.freedesktop.Notifications` D-Bus
  service (standard across most Linux desktops).
- **KDE**: KDE Plasma's notification daemon supports the same D-Bus interface.
- Notification history and watching require the Deskbrid daemon to log
  notifications to its state database.

## Current Status

**Stable** — send and close notifications.
**Experimental** — notification history, actions, clear history, watching.
