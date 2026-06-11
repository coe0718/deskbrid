# Contributing to Deskbrid

## Adding a new action

Deskbrid has 90+ action types that flow through a dispatch pipeline. Adding a new one requires touching these files — use this checklist:

### Required files (8)

1. **`src/protocol/parse/<domain>.rs`** — add CLI→Action parsing for the new variant
2. **`src/protocol/mod.rs`** — add the `Action::YourAction { .. }` variant to the enum
3. **`src/protocol/serialize/<domain>.rs`** — add `Action::YourAction => json!(...)` serialization
4. **`src/daemon/execute_<domain>.rs`** — add the execution handler, returning `anyhow::Result<Value>`
5. **`src/daemon/dispatch.rs`** — add the `is_<domain>_action()` dispatch gate function, wire it into `dispatch_action_with_options()`
6. **`src/mcp/tools.rs`** — register the MCP tool definition (or add to an existing category)
7. **`src/permissions.toml`** or **`~/.config/deskbrid/permissions.toml`** — add the action to default policy (or `allow_all()` if safe)
8. **`src/daemon/execute_stubs.rs`** — add a placeholder or remove if fully implemented

### Optional files

- **`src/mcp/helpers.rs`** — if the action needs a custom MCP serialization path
- **`src/cli/into_action.rs`** — add CLI subcommand parsing for the new action
- **`src/daemon/execute.rs`** — only if the action needs special routing (most don't; the domain gate in dispatch.rs handles it)

### Domain pattern

Each domain follows this convention:
- **parse/** — turns CLI args into `Action` variants
- **serialize/** — turns `Action` variants into socket JSON
- **execute_<domain>.rs** — runs the action, returns `anyhow::Result<serde_json::Value>`
- **is_<domain>_action()** — match gate function in dispatch.rs, returns `bool`

### Quick reference

```
# Example: adding "system.suspend"
touch:   src/protocol/parse/system.rs (add parse arm)
edit:    src/protocol/mod.rs (add Action::SystemSuspend)
edit:    src/protocol/serialize/system.rs (add serialize arm)
edit:    src/daemon/execute_system.rs (add execution arm)
edit:    src/daemon/dispatch.rs (add to gate)
edit:    src/mcp/tools.rs (add tool definition)
edit:    ~/.config/deskbrid/permissions.toml (allow or gate)
```

## Code standards

- **Edition 2024** — Rust edition across the entire project
- **No `unreachable!()`** — dispatch fallthrough must return `anyhow::bail!("internal dispatch error: ...")`
- **Zero clippy warnings** — run `cargo clippy --all-targets` before committing
- **`cargo fmt`** — all code formatted with default rustfmt rules
- **No feature flags** for zero-dependency code — compile always-on
- **High-risk actions** (`terminal.create`, `browser.evaluate`, `process.start`) go in user-level `permissions.toml`, NOT in the `allow_all()` default

## Lock ordering

When adding code that acquires multiple `DaemonState` locks, follow this order:
1. `backend` (RwLock)
2. `database` (tokio Mutex)
3. `rules` (Mutex)
4. `screencast_process` (Mutex)
5. `recording` (Mutex)
