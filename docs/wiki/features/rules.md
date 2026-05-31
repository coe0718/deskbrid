# Rules Engine

Deskbrid's rules engine allows you to define event-driven automation that triggers on desktop events like window focus, clipboard changes, or workspace switches.

## Overview

Rules are persistent and stored in the SQLite database. Each rule has:
- A unique ID (auto-generated)
- A name for identification
- A trigger event pattern
- An action type and parameters
- Optional cooldown and max_fires to prevent runaway loops
- An enabled/disabled state

## Creating a Rule

```bash
deskbrid rule.create { 
  name: "focus-terminal", 
  trigger: "window.focused", 
  action_type: "input.keyboard", 
  action_params: { text: "alacritty\\n" }, 
  enabled: true, 
  cooldown_ms: 1000, 
  max_fires: 5 
}
```

### Parameters

- `name`: Human-readable identifier for the rule
- `trigger`: Event pattern to listen for (see [Event Patterns](#event-patterns))
- `action_type`: The action to execute when triggered (see [Protocol Overview](Protocol-Overview))
- `action_params`: Parameters for the action (JSON object)
- `enabled`: Boolean to enable/disable the rule (default: true)
- `cooldown_ms`: Minimum time between executions in milliseconds (default: 0)
- `max_fires`: Maximum number of times the rule can fire (default: unlimited)

## Listing Rules

```bash
deskbrid rule.list
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "rules": [
      {
        "id": "rule_123",
        "name": "focus-terminal",
        "trigger": "window.focused",
        "action_type": "input.keyboard",
        "action_params": { "text": "alacritty\n" },
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

## Getting a Specific Rule

```bash
deskbrid rule.get { rule_id: "rule_123" }
```

## Deleting a Rule

```bash
deskbrid rule.delete { rule_id: "rule_123" }
```

## Pausing and Resuming Rules

```bash
# Pause a rule
deskbrid rule.pause { rule_id: "rule_123" }

# Resume a rule
deskbrid rule.resume { rule_id: "rule_123" }
```

## Event Patterns

Rules can listen for various desktop events:

### Window Events
- `window.focused` - When a window gains focus
- `window.created` - When a new window appears
- `window.closed` - When a window is closed
- `window.*` - All window events

### Input Events
- `input.keyboard` - Keyboard input
- `input.mouse` - Mouse movement/clicks
- `input.*` - All input events

### Clipboard Events
- `clipboard.changed` - When clipboard content changes
- `clipboard.*` - All clipboard events

### Monitor Events
- `monitor.added` - When a display is connected
- `monitor.removed` - When a display is disconnected
- `monitor.changed` - When display properties change
- `monitor.*` - All monitor events

### Workspace Events (where supported)
- `workspace.changed` - When active workspace changes
- `workspace.*` - All workspace events

### System Events
- `system.info` - System information queries
- `update.available` - When a deskbrid update is available
- `system.*` - All system events

## Examples

### Launch Terminal on Window Focus

Launch a terminal whenever a web browser window gains focus:

```bash
deskbrid rule.create {
  name: "browser-terminal",
  trigger: "window.focused",
  action_type: "input.keyboard",
  action_params: { text: "alacritty\n" },
  enabled: true,
  cooldown_ms: 5000
}
```

### Clipboard Monitoring with Cooldown

Send a notification when clipboard contains a URL, but limit to once per minute:

```bash
deskbrid rule.create {
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

### Workspace Switching Automation

Automatically launch applications when switching to a specific workspace:

```bash
deskbrid rule.create {
  name: "work-launch",
  trigger: "workspace.changed",
  action_type: "windows.activate_or_launch",
  action_params: { app_id: "code", name: "VS Code" },
  enabled: true
}
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# Create a rule
rule_id = client.rule_create(
    name="focus-notify",
    trigger="window.focused",
    action_type="notification.send",
    action_params={"title": "Window Focused", "body": "A window gained focus"},
    enabled=True,
    cooldown_ms=1000
)

# List rules
rules = client.rule_list()
for rule in rules['rules']:
    print(f"{rule['name']}: {rule['trigger']} -> {rule['action_type']}")

# Get a specific rule
rule = client.rule_get(rule_id=rule_id)
print(rule)

# Pause the rule
client.rule_pause(rule_id=rule_id)

# Resume the rule
client.rule_resume(rule_id=rule_id)

# Delete the rule
client.rule_delete(rule_id=rule_id)
```