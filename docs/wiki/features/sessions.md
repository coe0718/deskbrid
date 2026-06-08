# Sessions

Deskbrid v1.0.0 provides named sessions with isolated variable namespaces for
multi-agent coordination, persisted across restarts in SQLite.

## Concepts

- Each connection/session gets a unique name or auto-generated ID.
- Variables are per-session and don’t leak.
- Sessions survive daemon restarts.
- You can clone variables from one session into another.

## Examples

```bash
deskbrid session.create { name: "agent-1" }
deskbrid system.sessions
deskbrid session.switch { name: "agent-2" }
deskbrid session.destroy { name: "agent-1" }
```

## Session variables

```bash
deskbrid session.var.set { name: "counter", value: "42" }
deskbrid session.var.get { name: "counter" }
deskbrid session.var.list
```

## Multiple agents

```bash
deskbrid session.create { name: "coder" }
deskbrid session.var.set { name: "current_file", value: "main.rs" }

deskbrid session.create { name: "tester" }
deskbrid session.var.list
```

## Related

See [Persistence](persistence.md) for SQLite-backed state and [Blackboard](blackboard.md) for shared namespaced key/value storage.
