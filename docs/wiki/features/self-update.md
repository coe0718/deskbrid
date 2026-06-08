# Self-Update

Deskbrid v1.0.0 can update itself from GitHub releases. Update checks run in the
background by default (every 6 hours) and emit `update.available` events.

## Manual update check

```bash
deskbrid update.check {}
```

Response shows current and latest version plus release URL.

## Self-update

```bash
deskbrid self.update {}
```

This downloads, verifies, replaces the binary, and restarts the daemon.

## Environment

```bash
DESKBRID_UPDATE_INTERVAL=21600 deskbrid daemon
DESKBRID_UPDATE_CHECK=false deskbrid daemon
```
