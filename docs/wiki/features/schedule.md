# Schedule Feature

The Schedule feature allows agents to create, list, and delete deferred actions — commands that fire at a specific future time. The daemon runs a built-in scheduler loop that matches pending events against the current system clock. When a scheduled time arrives, the daemon internally emits a `schedule.event` message, which the agent can listen for. This is useful for reminders, periodic maintenance tasks, delayed automation, and cron-like workflows from an agent session.

## Actions

### schedule.list

Returns all currently scheduled events known to the daemon. Each event includes its id, target action, parameters, and scheduled time.

No parameters required.

```bash
deskbrid schedule.list {}
```

```json
{"type": "schedule.list"}
```

**Response format:**

```json
{
  "events": [
    {
      "id": "evt_01j3...",
      "at": "2026-06-27T14:30:00Z",
      "action": "notifications.send",
      "params": {"summary": "Meeting in 5 minutes"},
      "one_shot": true
    }
  ]
}
```

### schedule.create

Create a new scheduled event. The daemon will execute the specified `action` with the provided `params` at the given `at` timestamp. When the event fires, the daemon emits `schedule.event` with the id, action, params, and the original `at` time.

| Parameter | Type | Description |
|-----------|------|-------------|
| `at` | string (ISO 8601) | The ISO timestamp at which the event should fire (e.g. `"2026-06-27T14:30:00Z"`). |
| `action` | string | The protocol action to execute when the event fires (e.g. `"notifications.send"`). |
| `params` | object | A JSON object of parameters to pass to the target action at execution time. |
| `one_shot` | boolean | Optional. If `true`, the event is removed after firing. Defaults to `true` if omitted. Set to `false` for recurring events. |

```bash
deskbrid schedule.create {
  at: "2026-06-27T14:30:00Z",
  action: "notifications.send",
  params: { summary: "Time to stand up!" },
  one_shot: true
}
```

```json
{"type": "schedule.create", "at": "2026-06-27T14:30:00Z", "action": "notifications.send", "params": {"summary": "Time to stand up!"}, "one_shot": true}
```

**Response format:**

```json
{
  "id": "evt_01j3..."
}
```

### schedule.delete

Delete a previously created scheduled event by its unique id. Once deleted, the event will never fire.

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | string | The unique identifier of the scheduled event to delete (returned from `schedule.create`). |

```bash
deskbrid schedule.delete { id: "evt_01j3..." }
```

```json
{"type": "schedule.delete", "id": "evt_01j3..."}
```

### schedule.event

Internal event message emitted by the daemon when a scheduled event's time arrives. This is not an action an agent sends — it is received on the socket when a timer fires. Agents listen for incoming messages of this type.

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | The id of the event that fired. |
| `at` | string (ISO 8601) | The original scheduled timestamp. |
| `action` | string | The action that should be executed. |
| `params` | object | The payload to pass to the target action. |
| `one_shot` | boolean | Whether this event was one-shot (will not fire again). |

```json
{"type": "schedule.event", "id": "evt_01j3...", "at": "2026-06-27T14:30:00Z", "action": "notifications.send", "params": {"summary": "Time to stand up!"}, "one_shot": true}
```

## Safety Boundary

- Timing precision depends on the daemon's event loop — events are not real-time and may fire slightly late under load.
- Deleting an event that has already fired is a no-op; no error is returned.
- Recurring events (non-one-shot) continue to fire until explicitly deleted.
- There is no default rate limit on scheduled events, but the daemon applies its global rate-limiting policies per namespace.

## Local Development

Start the daemon in verbose mode and schedule a test event a few seconds in the future:

```bash
deskbrid daemon --verbose
```

In another terminal:

```bash
# Schedule an event 5 seconds from now
echo '{"type":"schedule.create","at":"'"$(date -u -d '+5 seconds' +%Y-%m-%dT%H:%M:%SZ)"'","action":"notifications.send","params":{"summary":"test"},"one_shot":true}' | nc -U $XDG_RUNTIME_DIR/deskbrid.sock -w 2

# List all events
echo '{"type":"schedule.list"}' | nc -U $XDG_RUNTIME_DIR/deskbrid.sock -w 2
```

## Configuration

The scheduler uses the daemon's internal timer loop and requires no additional configuration. Event persistence (surviving daemon restart) depends on whether the daemon is configured to persist its scheduler state to disk — see the daemon configuration guide for `persist_schedule` settings.
