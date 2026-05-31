# System Tray

Deskbrid v0.11.0 includes a system tray icon that provides quick access to common features and update notifications.

## Overview

The system tray icon appears in your desktop's notification area and provides:
- Visual indication that deskbrid is running
- Quick actions for common tasks
- Update notifications when a new version is available
- Access to the daemon's version and status

## Features

### Tray Icon Menu

Right-clicking the tray icon reveals a menu with the following options:

- **Show Version**: Displays the current deskbrid version
- **Check for Updates**: Manually triggers an update check
- **Open Web Dashboard**: Opens the screen recording web dashboard (if active)
- **Restart Daemon**: Restarts the deskbrid daemon
- **Quit**: Stops the deskbrid daemon

### Update Notifications

When a new version is detected via the background update check, the tray icon shows a notification badge or tooltip indicating an update is available. Clicking the notification or selecting "Check for Updates" from the menu will show the update details.

### Web Dashboard Integration

If screen recording is active, the tray menu includes an option to open the web dashboard at http://localhost:4199 (or configured port).

## Python Example

The system tray feature is managed entirely by the daemon and does not have a direct protocol interface. However, you can interact with it indirectly through the available commands:

```python
from deskbrid import Deskbrid

client = Deskbrid()

# Check for updates (triggers the same check as the tray menu)
update_info = client.update_check()
if not update_info['up_to_date']:
    print(f"Update available: {update_info['latest_version']}")

# Restart the daemon (same as tray menu option)
client.system_power(action="restart")  # Note: This is a system power action, not a direct tray control

# Note: There is no direct protocol to control the tray icon, but you can:
# - Use notification.send to show a custom notification
# - Use system.info to get version info
# - Use system.health to check daemon status
```

## Notes

- The system tray uses the `tray-icon` crate and is currently implemented for desktop environments that support system trays (GNOME, KDE, etc.)
- On some minimal window managers or WMs without a tray, the icon may not appear
- The tray icon is designed to be lightweight and unobtrusive
- Update notifications are also broadcast as `update.available` events for integration with other systems