---
name: deskbrid
description: Desktop control via Deskbrid daemon — inject keystrokes, read clipboard, take screenshots, track windows
---

# Deskbrid Hermes Skill

Use this skill when a Hermes agent needs to interact with the local Linux desktop through a running Deskbrid daemon.

## Requirement

Deskbrid must already be running and listening on its Unix socket before any examples below will work.

## Connect from Hermes

Inside `execute_code`, import the Python client and create a connection:

```python
import deskbrid

client = deskbrid.Deskbrid()
```

Close the client when you are done:

```python
client.close()
```

## Template Helper Functions

Use helpers like these for common operations:

```python
import deskbrid


def connect_client():
    return deskbrid.Deskbrid()


def current_windows(client):
    return client.list_windows()


def current_clipboard_text(client):
    return client.clipboard_read().text


def write_clipboard(client, text):
    client.clipboard_write(text)


def type_into_focused_window(client, text):
    client.type_text(text)


def send_notification(client, title, body=""):
    return client.notify(title, body)
```

## Common Examples

### Check what window is focused

```python
import deskbrid

client = deskbrid.Deskbrid()
try:
    windows = client.list_windows()
    focused = [window for window in windows if window.focused]
    print(focused[0] if focused else "No focused window reported")
finally:
    client.close()
```

`client.list_windows()` is the main entry point for window inspection.

### Type into the terminal

```python
import deskbrid

client = deskbrid.Deskbrid()
try:
    client.type_text("command\n")
finally:
    client.close()
```

### Read or write the clipboard

```python
import deskbrid

client = deskbrid.Deskbrid()
try:
    print(client.clipboard_read().text)
    client.clipboard_write("new clipboard contents")
finally:
    client.close()
```

### Take a screenshot

```python
import deskbrid

client = deskbrid.Deskbrid()
try:
    screenshot_path = client.screenshot()
    print(screenshot_path)
finally:
    client.close()
```

### Send a desktop notification

```python
import deskbrid

client = deskbrid.Deskbrid()
try:
    client.notify("Hermes", "Deskbrid task finished")
finally:
    client.close()
```

## Event Subscription Pattern

If the Hermes task needs to watch the desktop over time, attach handlers with `@client.on` and then call `client.listen()`:

```python
import deskbrid

client = deskbrid.Deskbrid()

@client.on("window:focus")
def on_focus(window):
    print(window.title)

@client.on("clipboard")
def on_clipboard(clip):
    print(clip.text)

client.listen()
```

## Practical Guidance

- Use `client.info()` first if you need to inspect daemon capabilities.
- Use `client.focus_window(...)` before typing if a specific app needs focus.
- Expect input injection to require a GNOME Wayland session with remote desktop access available.
- Prefer short, explicit operations instead of long unverified action chains.
