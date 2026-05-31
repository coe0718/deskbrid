# Self-Update

Deskbrid v0.11.0 includes a self-update mechanism that allows the binary to update itself from GitHub releases without external tools.

## Overview

The self-update feature consists of two parts:
1. **Update Check**: Background task that polls GitHub releases for newer versions
2. **Self-Update Command**: Manual command to download, install, and restart the latest binary

Both features use the GitHub Releases API and require no external dependencies.

## Update Check

Deskbrid automatically checks for updates in the background (every 6 hours by default). When a newer version is found, it broadcasts an `update.available` event to all subscribers.

### Event: update.available

```json
{
  "type": "event",
  "event": "update.available",
  "data": {
    "current_version": "v0.11.0",
    "latest_version": "v0.12.0",
    "release_url": "https://github.com/coe0718/deskbrid/releases/tag/v0.12.0",
    "release_notes": "## v0.12.0 - New Features\\n- Added XYZ feature\\n- Improved ABC performance"
  }
}
```

## Manual Update Check

You can manually trigger an update check:

```bash
deskbrid update check
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "current_version": "v0.11.0",
    "latest_version": "v0.11.0",
    "up_to_date": true
  }
}
```

Or if an update is available:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "current_version": "v0.11.0",
    "latest_version": "v0.12.0",
    "up_to_date": false,
    "release_url": "https://github.com/coe0718/deskbrid/releases/tag/v0.12.0"
  }
}
```

## Self-Update Command

To manually update to the latest version:

```bash
deskbrid self-update
```

This will:
1. Fetch the latest release from GitHub
2. Download the appropriate binary for your platform
3. Verify the download (SHA256 checksum)
4. Replace the running binary
5. Restart the daemon with the same arguments

### Output During Update
```
[DESKBRID] Checking for updates...
[DESKBRID] Found newer version: v0.12.0
[DESKBRID] Downloading deskbrid-v0.12.0-x86_64-unknown-linux-gnu.tar.gz
[DESKBRID] Verifying checksum...
[DESKBRID] Extracting binary...
[DESKBRID] Installing to /usr/local/bin/deskbrid
[DESKBRID] Restarting daemon...
[DESKBRID] Update complete! Running v0.12.0
```

## Configuration

The update check interval can be configured via environment variable:
```bash
DESKBRID_UPDATE_INTERVAL=21600 deskbrid daemon  # Check every 6 hours (default)
```

To disable automatic update checks:
```bash
DESKBRID_UPDATE_CHECK=false deskbrid daemon
```

## Notes

- The self-update feature requires internet access to reach github.com
- Update checks are performed via HTTPS to the GitHub Releases API
- Binary verification uses SHA256 checksums provided in the release assets
- The update process preserves your configuration files and schedule.json
- If the update fails, the daemon will continue running with the current version
- You need write permissions to the binary location (typically /usr/local/bin/)

## Python Example

```python
from deskbrid import Deskbrid
import time

client = Deskbrid()

# Check for updates
update_info = client.update_check()
print(f"Current version: {update_info['current_version']}")
print(f"Latest version: {update_info['latest_version']}")
print(f"Up to date: {update_info['up_to_date']}")

if not update_info['up_to_date']:
    print(f"Update available: {update_info['release_url']}")

# Trigger self-update (this will restart the daemon)
# client.self_update()  # Uncomment to actually perform update

# Listen for update events (in a real application, you'd maintain a subscription)
# For demonstration, we'll just check periodically
for i in range(5):
    update_info = client.update_check()
    if not update_info['up_to_date']:
        print("New version available!")
        break
    time.sleep(60)  # Check every minute
```