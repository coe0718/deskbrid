# Rules Engine

Define event-driven automation rules that trigger actions based on system
events, conditions, and schedules.

## Actions

### rule.create

Create a new automation rule.

| Parameter        | Type          | Description                                    |
|-----------------|---------------|------------------------------------------------|
| `name`          | string        | Human-readable rule name                       |
| `trigger`       | EventTrigger  | Trigger definition (event type + params)       |
| `condition`     | RuleCondition?| Optional condition for the rule to fire        |
| `action_type`   | string        | Action to execute (e.g. `notification.send`)   |
| `action_params` | JSON value    | Parameters to pass to the action               |
| `enabled`       | bool          | Whether the rule starts enabled                |
| `max_fires`     | uint?         | Max times the rule can fire (optional)         |
| `cooldown_ms`   | uint?         | Min time between firings in ms                 |

```bash
deskbrid rule.create '{
  "name": "notify-large-download",
  "trigger": {"event": "file.created", "pattern": "*.iso"},
  "action_type": "notification.send",
  "action_params": {"summary": "Large download complete", "body": "{path}"},
  "enabled": true
}'
```

```json
{
  "type": "rule.create",
  "name": "notify-large-download",
  "trigger": {"event": "file.created"},
  "condition": null,
  "action_type": "notification.send",
  "action_params": {"summary": "File created", "body": "A new file appeared"},
  "enabled": true,
  "max_fires": null,
  "cooldown_ms": 5000
}
```

### rule.list

List all configured automation rules.

```bash
deskbrid rule.list
```

No parameters.

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {"id": "rule-001", "name": "notify-large-download", "enabled": true, "trigger": "file.created", "action_type": "notification.send"},
    {"id": "rule-002", "name": "auto-suspend", "enabled": false, "trigger": "idle.timeout", "action_type": "system.power"}
  ]
}
```

### rule.get

Get the full configuration of a specific rule.

| Parameter | Type   | Description |
|-----------|--------|-------------|
| `rule_id` | string | Rule ID     |

```bash
deskbrid rule.get '{"rule_id": "rule-001"}'
```

### rule.delete

Delete a rule permanently.

| Parameter | Type   | Description |
|-----------|--------|-------------|
| `rule_id` | string | Rule ID     |

```bash
deskbrid rule.delete '{"rule_id": "rule-001"}'
```

### rule.pause

Temporarily disable a rule without deleting it.

| Parameter | Type   | Description |
|-----------|--------|-------------|
| `rule_id` | string | Rule ID     |

```bash
deskbrid rule.pause '{"rule_id": "rule-001"}'
```

### rule.resume

Re-enable a paused rule.

| Parameter | Type   | Description |
|-----------|--------|-------------|
| `rule_id` | string | Rule ID     |

```bash
deskbrid rule.resume '{"rule_id": "rule-001"}'
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# Create a rule
rule = client.rule_create(
    name="low-battery-alert",
    trigger={"event": "system.battery_low"},
    action_type="notification.send",
    action_params={
        "summary": "Battery low",
        "body": "Connect charger soon",
        "urgency": "critical"
    },
    enabled=True
)

print(f"Created rule: {rule['id']}")

# List rules
for r in client.rule_list():
    print(f"  {r['name']} - {'enabled' if r['enabled'] else 'disabled'}")
```

## Requirements

- Rules are evaluated by the Deskbrid daemon at runtime.
- Trigger events depend on the event monitoring subsystem (inotify, D-Bus
  signals, timer events).
- Rules persist across daemon restarts (stored in the state directory).

## Safety

- Rules with destructive actions (power, file delete, etc.) may require
  confirmation mode.
- `max_fires` and `cooldown_ms` prevent accidental infinite loops.
- Rules cannot create other rules (no recursion).

## Current Status

**Experimental** — v1.0.0 feature.
