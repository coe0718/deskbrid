# Quick Start

Get Deskbrid running in 5 minutes.

## 1. Install

```bash
curl -LO https://github.com/coe0718/deskbrid/releases/latest/download/deskbrid
chmod +x deskbrid
sudo mv deskbrid /usr/local/bin/
```

## 2. Install Desktop Dependencies

**GNOME:**
```bash
sudo apt install -y grim wl-clipboard
deskbrid setup
```

**Hyprland:**
```bash
sudo pacman -S grim wl-clipboard ydotool
echo 'KERNEL=="uinput", GROUP="input", MODE="0660"' | sudo tee /etc/udev/rules.d/99-input.rules
sudo usermod -aG input $USER
```

**KDE:**
```bash
sudo apt install spectacle imagemagick wl-clipboard ydotool
```

## 3. Start the Daemon

```bash
deskbrid daemon &
```

## 4. Test Basic Commands

```bash
# List open windows
deskbrid windows list

# Read clipboard
deskbrid clipboard read

# Take a screenshot
deskbrid screenshot

# Get system information
deskbrid system info
```

## 5. Control Your Desktop

```bash
# Focus a window
deskbrid windows focus --app code

# Type text
deskbrid input keyboard type "Hello, world!"

# Send key combinations
deskbrid combo Ctrl_L+c
deskbrid combo Super_L+Tab

# Send a notification
deskbrid notify "Deskbrid" "Setup complete!"
```

## 6. Connect AI Agents (Optional)

Configure MCP for Claude Code, Cursor, or other AI coding tools:

```json
{
  "mcpServers": {
    "deskbrid": {
      "command": "deskbrid",
      "args": ["mcp"]
    }
  }
}
```

Then your AI agent can:
- List and focus windows
- Type and click
- Take screenshots with OCR
- Read and write clipboard
- Control media playback
- And much more!

## Next Steps

- [Windows & Workspaces](features/windows-workspaces.md) - Window management
- [Input Control](features/input.md) - Keyboard and mouse automation
- [Protocol](protocol/overview.md) - Programmatic JSON protocol
- [Python Client](integrations/python.md) - Build your own tools