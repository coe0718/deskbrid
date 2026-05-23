# Deskbrid Wiki

**The HAL your Linux desktop agents are missing.**

Deskbrid is a single Rust binary that auto-detects your desktop environment and provides a unified JSON-over-Unix-socket protocol for programmatic desktop control. Works on GNOME, Hyprland, KDE, and X11 desktops.

## Getting Started

- [Installation](installation.md) - Download, build, and set up Deskbrid
- [Quick Start](quick-start.md) - Get running in 5 minutes

## Core Features

- [Windows & Workspaces](features/windows-workspaces.md) - Manage windows and virtual desktops
- [Input Control](features/input.md) - Keyboard and mouse automation
- [Clipboard](features/clipboard.md) - Read/write clipboard content
- [Screenshots & OCR](features/screenshots.md) - Capture and analyze screen content
- [Notifications](features/notifications.md) - Desktop notifications
- [System Information](features/system.md) - System status and power control
- [Media Control](features/media.md) - MPRIS media player integration
- [Audio](features/audio.md) - Audio sink management
- [Network](features/network.md) - WiFi and network status
- [Bluetooth](features/bluetooth.md) - Device discovery and connection
- [File Operations](features/files.md) - File search and watching
- [Services](features/services.md) - systemd service management
- [Terminals](features/terminals.md) - PTY session management
- [Monitors](features/monitors.md) - Display configuration
- [Layout Profiles](features/layout-profiles.md) - Save and restore workspace layouts
- [Accessibility](features/accessibility.md) - AT-SPI tree inspection

## Protocol

- [Protocol Overview](protocol/overview.md) - JSON protocol specification
- [Event Subscription](protocol/events.md) - Real-time event streaming
- [MCP Integration](protocol/mcp.md) - Model Context Protocol support

## Integrations

- [Python Client](integrations/python.md) - Python library usage
- [AI Agents](integrations/agents.md) - Using with AI coding tools

## Development

- [Architecture](development/architecture.md) - System design
- [Backends](backends.md) - Desktop environment backends (do not modify per instructions)