---
name: deskbrid
description: >
  Full control over a Linux desktop — windows, keyboard/mouse input,
  AT-SPI UI inspection, clipboard, screenshots, audio, and system status.
  Use these tools to automate desktop tasks, interact with GUI applications,
  and inspect what's happening on screen.
tools: [list_windows, focus_window, close_window, minimize_window, maximize_window,
  move_resize_window, list_workspaces, switch_workspace, type_text, press_key,
  press_keys, mouse_move, mouse_click, mouse_scroll, click_coordinate,
  clipboard_read, clipboard_write, screenshot, system_info, idle_seconds,
  list_audio_sinks, set_volume]
---

# Deskbrid — Linux Desktop Control

You have full control over the Linux desktop through Deskbrid tools.

## Core Workflow

1. **See**: Use `screenshot` to capture the screen
2. **Find**: `list_windows` for window IDs, AT-SPI tools for UI elements
3. **Act**: Keyboard/mouse tools to interact
4. **Verify**: Another screenshot to confirm

## Windows

- `list_windows` — IDs, titles, classes, geometry for all windows
- `focus_window` — Bring window to foreground
- `close_window`, `minimize_window`, `maximize_window` — Standard ops
- `move_resize_window` — Position and size
- `list_workspaces` / `switch_workspace` — Virtual desktops

## Keyboard

- `type_text` — Type at human speed
- `press_key` — Single key: Return, Escape, Tab, F5, etc.
- `press_keys` — Combos: ['Control_L', 'c'] for copy

**Always click into a text field before typing.**

## Mouse

- `mouse_move` — Absolute coordinates
- `mouse_click` — left/middle/right
- `mouse_scroll` — Wheel (positive dy = down)
- `click_coordinate` — Move + click

## Clipboard

- `clipboard_read` — Get clipboard text
- `clipboard_write` — Put text on clipboard (then paste with Control_L+v)

## Screenshots

- `screenshot` — Full screen as base64 PNG

**Always screenshot after actions to verify. Always screenshot before to understand state.**

## System

- `system_info` — Hostname, OS, kernel, memory, CPU
- `idle_seconds` — Time since last user input

## Audio

- `list_audio_sinks` — Output devices
- `set_volume` — Set volume (0.0-1.0)

## Patterns

### Open an App
1. press_keys(["Super_L"])
2. type_text("firefox")
3. press_key("Return")
4. Wait 2 seconds
5. screenshot()

### Click by Position
1. screenshot()
2. click_coordinate(x=450, y=320)
3. screenshot()

### Fill a Form
1. focus_window("firefox-123")
2. click_coordinate(x=200, y=150)
3. type_text("hello@example.com")
4. press_key("Tab")
5. type_text("password")
6. press_key("Return")

### Read Clipboard
1. press_keys(["Control_L", "a"])
2. press_keys(["Control_L", "c"])
3. clipboard_read()

## Limitations

- Cannot read text from screen (use clipboard patterns or AT-SPI)
- AT-SPI needs app accessibility support (most GTK/Qt apps have it)
- Wayland has stricter input security
