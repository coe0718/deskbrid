# Rules Engine

Deskbrid v1.0.0 provides a persistent, event-driven rules engine. Rules live in
SQLite and survive daemon restarts. They react to desktop events using the same
action strings the daemon dispatches over the socket.

## Overview

Rules are managed through the `rules.list`, `rules.create`, `rules.get`,
`rules.update`, `rules.patch`, `rules.trigger`, `rules.pause`, `rules.resume`,
and `rules.delete` actions. Each rule covers:

- `id` — auto-generated stable ID (`rules_<...>`) used for follow-up actions
- `name` — human-readable label
- `trigger` — event pattern (`window.focused`, `clipboard.changed`, `monitor.added`, ...)
- `action_type` — dispatch target (`input.keyboard`, `notification.send`, ...)
- `action_params` — optional JSON parameters for the action
- `enabled` — boolean, `true` means the rule can fire
- `cooldown_ms` — minimum interval between executions
- `max_fires` — optional cap on total executions
- `fires` — read-only execution counter
- `last_fired` — read-only timestamp of most recent execution

## Listing rules

```bash
deskbrid rules.list
```

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "rules": [
      {
        "id": "rules_123",
        "name": "focus-terminal",
        "trigger": "window.focused",
        "action_type": "input.keyboard",
        "action_params": {"text": "alacritty\\n"},
        "enabled": true,
        "cooldown_ms": 1000,
        "max_fires": 5,
        "fires": 2,
        "last_fired": "2026-05-30T10:30:00Z"
      }
    ]
  }
}
```

## Creating a rule

```bash
deskbrid rules.create {
  name: "focus-terminal",
  trigger: "window.focused",
  action_type: "input.keyboard",
  action_params: { text: "alacritty\\n" },
  enabled: true,
  cooldown_ms: 1000,
  max_fires: 5
}
```

## Inspecting, pausing, resuming, and deleting

```bash
deskbrid rules.get { rule_id: "rules_123" }
deskbrid rules.patch { rule_id: "rules_123", enabled: true }
deskbrid rules.pause { rule_id: "rules_123" }
deskbrid rules.resume { rule_id: "rules_123" }
deskbrid rules.delete { rule_id: "rules_123" }
```

Use `patch` for partial updates (available in v1.0.0). Read-only fields (`fires`,
`last_fired`) are not accepted on create.

## Triggering manually

```bash
deskbrid rules.trigger { rule_id: "rules_123" }
```

Force a rule to execute once regardless of schedule/cooldown. Useful for
debugging or ad-hoc automation.

## Event patterns

| pattern | meaning |
|---|---|
| `window.focused` | window gained focus |
| `window.created` | new window appeared |
| `window.closed` | window closed |
| `window.moved` | position changed |
| `window.resized` | size changed |
| `input.keyboard` | key pressed |
| `input.mouse` | mouse movement/clicks |
| `clipboard.changed` | clipboard content changed |
| `monitor.added` | display connected |
| `monitor.removed` | display disconnected |
| `monitor.changed` | display settings changed |
| `workspace.changed` | active workspace changed |
| `*` | everything |

Wildcards are supported, e.g. `window.*`.

## Examples

### Launch terminal on browser focus

```bash
deskbrid rules.create {
  name: "browser-terminal",
  trigger: "window.focused",
  action_type: "input.keyboard",
  action_params: { text: "alacritty\\n" },
  enabled: true,
  cooldown_ms: 5000
}
```

### Clipboard URL detector with rate limit

```bash
deskbrid rules.create {
  name: "url-notify",
  trigger: "clipboard.changed",
  action_type: "notification.send",
  action_params: {
    title: "URL Detected",
    body: "Clipboard contains a URL"
  },
  enabled: true,
  cooldown_ms: 60000
}
```

### Workspace-based auto-launch

```bash
deskbrid rules.create {
  name: "work-launch",
  trigger: "workspace.changed",
  action_type: "windows.activate_or_launch",
  action_params: { app_id: "code", name: "VS Code" },
  enabled: true
}
```

## Python example

```python
from deskbrid import Deskbrid

client = Deskbrid()

rule_id = client.rules_create(
    name="focus-notify",
    trigger="window.focused",
    action_type="notification.send",
    action_params={"title": "Window Focused", "body": "A window gained focus"},
    enabled=True,
    cooldown_ms=1000,
)

rules = client.rules_list()
for rule in rules["rules"]:
    print(rule["name"], "->", rule["action_type"])

client.rules_pause(rule_id=rule_id)
client.rules_trigger(rule_id=rule_id)
client.rules_resume(rule_id=rule_id)
client.rules_delete(rule_id=rule_id)
```
