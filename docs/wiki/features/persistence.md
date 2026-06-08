# Persistence

Deskbrid v1.0.0 persists state in SQLite at
`~/.config/deskbrid/deskbrid.db` using WAL mode.

## Backed-up state

- `clipboard_history`
- `audit_log`
- `blackboard`
- `rules`
- `sessions`
- `macros`
- `schedules`

## Backup

```bash
cp ~/.config/deskbrid/deskbrid.db ~/backups/deskbrid.db.$(date +%Y%m%d_%H%M%S)
```

## Restore

```bash
cp ~/backups/deskbrid.db.<timestamp> ~/.config/deskbrid/deskbrid.db
pkill deskbrid && deskbrid daemon &
```
