# Persistence

Deskbrid v0.11.0 introduces a SQLite persistence layer that survives daemon restarts. The database is located at `~/.local/share/deskbrid/deskbrid.db` and uses WAL mode for better concurrency.

## Database Schema

The persistence layer consists of 6 active tables:

| Table | Status | Description |
|-------|--------|-------------|
| `clipboard_history` | ✅ | Stores clipboard entries with timestamp and source |
| `audit_log` | ✅ | Records every action (success or failure) for debugging and security |
| `blackboard` | ✅ | Namespace-scoped KV store for multi-agent coordination |
| `notifications` | ✅ | Intercepted desktop notifications via D-Bus |
| `rules` | ✅ | Persistent storage for event-driven automation rules |
| `sessions` | ✅ | Named sessions with isolated variable namespaces |
| `macros` | — | Table exists but engine uses file-based storage by design |
| `cron_jobs` | ☠️ | Removed — scheduler uses `~/.config/deskbrid/schedule.json` |

## Wiring

Persistence is wired via public methods in `src/daemon/persistence.rs`:

- `record_clipboard_text()` — fire-and-forget on every clipboard read/write
- `record_audit_entry()` — every action, success or failure
- `blackboard.set/get/delete/list` — 62-line executor
- `rule.create/delete` persistence
- `session.create/destroy` persistence

## Usage Examples

### Clipboard History

Clipboard history is automatically persisted and can be queried via the protocol:

```bash
deskbrid clipboard history --limit 50
deskbrid clipboard history --query "error"
```

### Audit Log

Every deskbrid action is logged to the audit table. This can be useful for debugging or security auditing:

```bash
# Not directly exposed via protocol yet, but accessible via:
sqlite3 ~/.local/share/deskbrid/deskbrid.db "SELECT * FROM audit_log ORDER BY id DESC LIMIT 10;"
```

### Blackboard

The blackboard provides a shared namespace-scoped KV store for multi-agent coordination:

```bash
# Set a value
deskbrid blackboard set --key "counter" --value "42" --namespace "agent1"

# Get a value
deskbrid blackboard get --key "counter" --namespace "agent1"

# List all keys in a namespace
deskbrid blackboard list --namespace "agent1"

# Delete a key
deskbrid blackboard delete --key "counter" --namespace "agent1"
```

Protocol equivalents:
```json
{"type": "blackboard.set", "key": "counter", "value": "42", "namespace": "agent1"}
{"type": "blackboard.get", "key": "counter", "namespace": "agent1"}
{"type": "blackboard.list", "namespace": "agent1"}
{"type": "blackboard.delete", "key": "counter", "namespace": "agent1"}
```

## Backup and Restore

The SQLite database can be backed up and restored like any other file:

```bash
# Backup
cp ~/.local/share/deskbrid/deskbrid.db ~/backups/deskbrid.db.$(date +%Y%m%d_%H%M%S)

# Restore
cp ~/backups/deskbrid.db.20260530_120000 ~/.local/share/deskbrid/deskbrid.db
# Restart daemon for changes to take effect
pkill deskbrid && deskbrid daemon &
```

## Vacuum and Integrity Check

Periodic maintenance can be performed with standard SQLite commands:

```bash
# Check integrity
sqlite3 ~/.local/share/deskbrid/deskbrid.db "PRAGMA integrity_check;"

# Vacuum to reclaim space
sqlite3 ~/.local/share/deskbrid/deskbrid.db "VACUUM;"
```