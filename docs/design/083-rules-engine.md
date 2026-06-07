# Rules Engine Design

**Deskbrid Issue #83**

## Overview
Provide event-driven triggers that allow agents to automatically execute actions when specific desktop events occur. Agents should be able to define rules like "when clipboard changes, run action X" or "when window Y closes, do Z" without polling.

This spec integrates the rules engine with Deskbrid's confirmation system to create a coherent safety model where rules can only execute actions that would normally require user confirmation, making the system safe enough to ship.

## Background

### Current Limitations
Deskbrid currently offers:
- **Cron/scheduled actions** (Issue #27): Time-based scheduling only
- **File watching** (via `files.watch`): Limited to specific paths, no system-wide awareness
- **Subscription system**: Clients can subscribe to event streams but must actively poll/process

There's no built-in mechanism for the daemon to automatically trigger actions based on events without agent-side polling logic.

### What's Needed
A lightweight rules engine that:
1. Listens to Deskbrid's internal event bus (same as subscription system)
2. Evaluates user-defined rules when events occur
3. Executes associated actions when trigger conditions match, **but only after confirmation**
4. Supports optional filtering conditions
5. Includes safety features like cooldowns and max firings
6. Persists rules to survive daemon restarts
7. Integrates with the existing confirmation system for safety

## Design Principles

### Safety First (Integrated Model)
- Rules execute with the same permissions as the agent that created them
- **All actions triggered by rules go through the confirmation system** - this is the key safety integration
- Rules cannot create other rules (prevents rule-based rule creation loops)
- No privilege escalation through rules
- The confirmation system acts as the gatekeeper for all rule-triggered actions

### Simplicity
- Declarative rule definition (JSON over protocol)
- Minimal rule syntax focused on common desktop automation use cases
- Clear separation of trigger, condition, and action
- Condition language is intentionally limited to prevent complexity while remaining useful

### Integration
- Uses existing Deskbrid event subscription infrastructure
- Leverages existing action execution pipeline **including confirmation system**
- Persists to SQLite (tying into Issue #84 persistence layer)
- Exposes via standard protocol actions and MCP tools

## Core Components

### Rule Structure
Each rule consists of:
- **id**: Unique identifier (UUID)
- **name**: Human-readable label
- **trigger**: What event to watch for (see EventTrigger enum)
- **condition**: Optional filter expression (e.g., "clipboard contains 'password'", "window title matches regex")
- **action**: What to execute when trigger fires (any valid Deskbrid action)
- **enabled**: Boolean toggle
- **max_fires**: Optional limit after which rule auto-disables
- **cooldown_ms**: Minimum time between executions
- **confirmation_required**: Boolean (defaults to true) - whether this rule's actions require confirmation

**Key Safety Feature**: By default, all rules require confirmation for their actions. Agents can set `confirmation_required: false` only for actions that are explicitly marked as safe in the permissions system (rarely used).

### Event Triggers
The daemon listens to these events from its internal bus (the same DeskbridEvent system used for subscriptions):
- `ClipboardChanged`: Clipboard content changed
- `WindowOpened { app_id }`: New window appeared (optional app filter)
- `WindowClosed { app_id }`: Window closed
- `WindowFocused { app_id }`: Window gained focus
- `SessionLocked`: User session locked
- `SessionUnlocked`: User session unlocked
- `IdleStarted`: System became idle
- `IdleEnded`: System returned from idle
- `FileChanged { path }`: Specific file modified
- `TimeRange { start_hour, end_hour, days }`: Time-based trigger (like cron but integrated)
- `PresenceChanged { to }`: User presence changed (active/idle/away)

**Event Bus Integration**: The rules engine subscribes to the Deskbrid event bus using the same mechanism as client subscriptions. When an event is emitted, it's broadcast to all subscribers including the rules engine, which then evaluates matching rules.

Note: TimeRange triggers are evaluated every minute to avoid excessive processing.

### Condition Language
Conditions are simple expressions evaluated against event data, designed to be **simple enough for agents to write but powerful enough to be useful**:
- **Comparison**: `==`, `!=`, `<`, `>`, `<=`, `>=`
- **String operations**: `contains`, `starts_with`, `ends_with`, `matches` (regex)
- **Logical**: `AND`, `OR`, `NOT`
- **Parentheses** for grouping
- **Literals**: strings, numbers, booleans

Examples:
- `"clipboard_text contains 'https://' AND clipboard_text.length > 10"`
- `"window_app_id == 'code' AND window_title matches '.* - GitHub'"`  
- `"file_path.ends_with('.log') AND file_size > 1024"`
- `"presence == 'idle' AND time_of_day.hour >= 22"`

**Design Rationale**: This language avoids complex programming constructs that could lead to security issues or difficult-to-debug rules, while providing enough power for common automation scenarios. Event data available depends on trigger type but includes standard fields like timestamps, IDs, etc.

### Action Execution with Confirmation Integration
When a rule fires:
1. Validate rule is enabled and not past max_fires
2. Check cooldown period
3. Evaluate condition (if present) against event data
4. If all pass, **prepare the associated action for confirmation**
5. The action is submitted to the confirmation system (not executed directly)
6. User must approve via the confirmation UI before execution proceeds
7. If confirmed, the action executes through the normal dispatch pipeline
8. Update rule statistics (last_fired, fire_count)
9. Handle auto-disable if max_fires reached

**Critical Safety Integration**: Actions are never executed directly from rules. They always go through `Action::ConfirmAction` in the confirmation system, which presents the standard confirmation UI and waits for user approval.

Actions are any valid Deskbrid protocol action (same as what agents can send directly).

## Confirmation System Integration Details

The rules engine and confirmation system work together as a unified safety model:

### How It Works
1. When a rule condition matches, the rules engine creates a `ConfirmAction` protocol action instead of executing the rule's action directly
2. This `ConfirmAction` includes:
   - The original action to be performed
   - Metadata about the rule that triggered it (rule ID, name, trigger, condition)
   - The event data that caused the trigger
   - Agent session information
3. The confirmation system displays a prompt to the user:
   ```
   Rule "[rule-name]" triggered by [trigger] wants to:
   [action description]
   
   Event: [description of what triggered the rule]
   Conditions: [condition expression if any]
   
   Allow this action?
   [Allow] [Deny] [Always allow for this rule] [Never allow for this rule]
   ```
4. User response determines whether the action proceeds
5. All rule-triggered actions appear in the confirmation queue alongside manually-triggered confirmations

### Benefits of This Approach
- **Consistent UI**: Users see the same confirmation dialog whether an action comes from direct agent command or rule trigger
- **Centralized Policy**: Confirmation rules (temporary/permanent allow/deny) apply equally to rule-triggered and direct actions
- **Audit Trail**: All actions, whether rule-triggered or direct, go through the same confirmation audit log
- **Graceful Degradation**: If confirmation system is disabled, rules requiring confirmation simply won't fire (safe failure)

## Conflict Resolution

When multiple rules match the same event, the system needs a deterministic way to decide which actions to pursue:

### Conflict Resolution Algorithm
1. **Collect all matching rules** for the event (same trigger type, conditions pass)
2. **Sort by priority** (defined below)
3. **Process in order**, but with important constraints:
   - If a rule's action requires confirmation, it goes to the confirmation queue
   - Multiple pending confirmations are allowed (they don't block each other)
   - However, if two rules would trigger the **exact same action** (same action type and parameters), only one confirmation is generated to avoid spam
4. **Rule Priority** (highest to lowest):
   - Rules with `max_fires` remaining (higher priority than exhausted rules)
   - Recently created rules (newer rules get higher priority - prevents old rules from hogging)
   - Lexicographic order by rule ID (deterministic tiebreaker)

### Special Cases
- **Same action from different rules**: Deduplicated in confirmation system to prevent spamming the user with identical requests
- **Conflicting actions** (e.g., one rule wants to volume up, another volume down): Both generate separate confirmation requests - user decides
- **Rule disables itself**: If a rule has `max_fires: 1`, it disables after first successful confirmation

This approach ensures that users are informed about all significant actions while avoiding unnecessary duplication.

## Persistence Layer Integration
- Rules stored in SQLite database (tying into Issue #84)
- Table: `rules` with columns for all Rule fields plus agent_id for ownership
- Index on `agent_id` and `enabled` for efficient lookup
- Automatic cleanup when agent sessions expire
- Background task evaluates rules against incoming events
- Persistence ensures rules survive daemon restarts, maintaining agent automation configurations

## Protocol Actions

All actions require agent identification (via existing auth) and return standard response format.

### Rule Management
```json
// Create a new rule
{
  "type": "rule.create",
  "id": "<uuid>",
  "rule": {
    "name": "Clipboard URL Logger",
    "trigger": "ClipboardChanged",
    "condition": "clipboard_text.starts_with('http')",
    "action": { "type": "files.append", "path": "/tmp/clipboard-log.txt", "content": "${clipboard_text}\\n" },
    "enabled": true,
    "max_fires": null,
    "cooldown_ms": 5000,
    "confirmation_required": true  // explicit default
  }
}

// List rules (optionally filter by enabled state)
{
  "type": "rule.list",
  "id": "<uuid>",
  "enabled": true  // optional
}

// Get specific rule
{
  "type": "rule.get",
  "id": "<uuid>",
  "rule_id": "<rule-uuid>"
}

// Update rule (partial updates allowed)
{
  "type": "rule.update",
  "id": "<uuid>",
  "rule_id": "<rule-uuid>",
  "changes": { "enabled": false, "cooldown_ms": 10000, "confirmation_required": false }
}

// Delete rule
{
  "type": "rule.delete",
  "id": "<uuid>",
  "rule_id": "<rule-uuid>"
}

// Pause/resume rule
{
  "type": "rule.pause",
  "id": "<uuid>",
  "rule_id": "<rule-uuid>"
}
{
  "type": "rule.resume",
  "id": "<uuid>",
  "rule_id": "<rule-uuid>"
}
```

### Response Format
All responses follow:
```json
{
  "type": "<action.type>",
  "id": "<request-id>",
  // ... action-specific fields or error
}
```

Example rule.create success:
```json
{
  "type": "rule.create",
  "id": "req-123",
  "rule_id": "rule-abc-456",
  "success": true
}
```

Example rule.get:
```json
{
  "type": "rule.get",
  "id": "req-123",
  "rule": {
    "id": "rule-abc-456",
    "name": "Clipboard URL Logger",
    "trigger": "ClipboardChanged",
    "condition": "clipboard_text.starts_with('http')",
    "action": { "type": "files.append", ... },
    "enabled": true,
    "created_at": "2026-06-06T10:30:00Z",
    "last_fired": null,
    "fire_count": 0,
    "max_fires": null,
    "cooldown_ms": 5000,
    "confirmation_required": true
  }
}
```

## MCP Tool Signatures

```typescript
interface Rule {
  id: string;
  name: string;
  trigger: EventTrigger;
  condition?: string;  // expression language
  action: DeskbridAction;  // any valid action object
  enabled: boolean;
  max_fires?: number;
  cooldown_ms?: number;
  confirmation_required?: boolean;  // defaults to true
  created_at: string;  // ISO timestamp
  last_fired?: string;
  fire_count: number;
}

// Event trigger types
type EventTrigger = 
  | { type: 'ClipboardChanged' }
  | { type: 'WindowOpened'; app_id?: string }
  | { type: 'WindowClosed'; app_id?: string }
  | { type: 'WindowFocused'; app_id?: string }
  | { type: 'SessionLocked' }
  | { type: 'SessionUnlocked' }
  | { type: 'IdleStarted' }
  | { type: 'IdleEnded' }
  | { type: 'FileChanged'; path: string }
  | { type: 'TimeRange'; start_hour: number; end_hour: number; days: number[] }  // 0-6, Sun-Sat
  | { type: 'PresenceChanged'; to: 'active' | 'idle' | 'away' };

// MCP Tools
async function listRules(enabled?: boolean): Promise<Rule[]>
async function getRule(ruleId: string): Promise<Rule | null>
async function createRule(rule: Omit<Rule, 'id' | 'created_at' | 'last_fired' | 'fire_count'>): Promise<string>  // returns ruleId
async function updateRule(ruleId: string, changes: Partial<Rule>): Promise<void>
async function deleteRule(ruleId: string): Promise<void>
async function pauseRule(ruleId: string): Promise<void>
async function resumeRule(ruleId: string): Promise<void>
```

## Security Considerations

### Agent Boundaries
- Rules can only trigger actions the creating agent is authorized to perform
- Rule definitions themselves cannot contain executable code (only data)
- No rule-to-rule chaining that could create infinite loops
- **All actions funnel through confirmation system** - the ultimate gatekeeper

### Data Exposure
- Rule names/triggers/actions may be visible in audit logs and confirmation prompts
- Condition expressions may contain sensitive patterns (e.g., regex for passwords)
- Mitigation: The confirmation UI shows condition expressions but users can see they're just conditions, not secrets

### Resource Limits
- Max rules per agent (configurable, default 100)
- Max condition complexity to prevent DoS (limits on expression length and operator count)
- Event evaluation timeout (e.g., 100ms) to prevent blocking event loop
- Confirmation deduplication prevents UI spam from similar rules

## Implementation Notes

### Event Processing Flow
1. Deskbrid daemon emits events to internal bus (existing mechanism)
2. Rules engine subscribes to all relevant event types via the subscription system
3. When event arrives:
   - Load enabled rules for the event's agent scope from SQLite
   - For each rule, check if trigger matches event type
   - If trigger matches, evaluate condition against event data
   - If condition passes (or no condition), create a ConfirmAction for the rule's action
   - Send ConfirmAction to confirmation system (not direct execution)
   - Update rule statistics (last checked, etc.) regardless of confirmation outcome
4. All actions go through standard dispatch pipeline (confirmations, rate limits, etc.)

### Condition Evaluation
- Safe expression evaluator (no arbitrary code execution)
- Whitelisted operators and functions only
- Timeout protection for complex evaluations
- Type checking to prevent runtime errors

### Performance
- Rule matching optimized by indexing rules by trigger type in memory (loaded from SQLite on change)
- Condition evaluation only runs when trigger matches
- Background evaluation avoids blocking event loop
- SQLite queries optimized with proper indexing
- Rules loaded into memory cache with invalidation on database changes

## Open Questions

### Condition Language Complexity
- How powerful should the condition language be?
- Start simple (comparisons, basic string ops) and extend if needed
- Consider security implications of complex expressions

### Wildcard Triggers
- Should we support triggers like "WindowOpened for any editor"?
- Could use condition: `window_app_id.matches('.*code.*|.*vim.*|.*nvim.*')`

### Nested Conditions
- Support for AND/OR/NOT with parentheses sufficient for most use cases?
- Or should we allow more complex decision trees?

### Rule Inheritance/Templates
- Allow agents to define rule templates for reuse?
- Or keep it flat and simple?

### Cross-Agent Rules
- Should one agent be able to trigger rules for another?
- Probably not - maintains clear boundaries

### Persistence Details
- Should rules persist indefinitely or have TTL?
- Tie to agent session persistence (Issue #79)?
- Allow export/import of rule sets?

### Confirmation Defaults
- Should all rules require confirmation by default, or only "dangerous" actions?
- Current spec: all rules require confirmation by default for maximum safety
- Agents can override for specific safe actions if needed

---
*This spec assumes integration with Deskbrid's existing event subscription system, action pipeline, confirmation system, and planned SQLite persistence layer (Issue #84). The rules engine adds approximately 300-400 lines of Rust code plus protocol definitions. The confirmation system integration is the key safety feature that makes automated rules acceptable for production use.*