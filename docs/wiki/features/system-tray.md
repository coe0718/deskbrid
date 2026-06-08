# System Tray

Deskbrid v1.0.0 includes a system tray icon on DEs that support it. The tray
provides quick access to common features and update notifications.

- Shows daemon is running
- Menu: Show Version, Check for Updates, Open Web Dashboard,
  Restart Daemon, Quit
- Emits `update.available` events to subscribers

Tray control is managed by the daemon itself, not via a socket action.
