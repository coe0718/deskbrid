# Desktop Settings

Read and write desktop environment settings through GSettings/DConf on GNOME,
and equivalent configuration backends on other desktops (KDE, Hyprland, Sway,
COSMIC, Labwc, Niri, Wayfire, X11).

## Actions

### desktop_settings.get

Read a desktop setting by schema and key.

| Parameter | Type   | Description                          |
|-----------|--------|--------------------------------------|
| `schema`  | string | GSettings schema ID                  |
| `key`     | string | Key within the schema                |

```bash
deskbrid desktop.get '{"schema": "org.gnome.desktop.interface", "key": "gtk-theme"}'
```

```json
{"type": "desktop.get", "schema": "org.gnome.desktop.interface", "key": "gtk-theme"}
```

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "value": "Adwaita-dark"
  }
}
```

### desktop_settings.set

Set a desktop setting value.

| Parameter | Type   | Description                          |
|-----------|--------|--------------------------------------|
| `schema`  | string | GSettings schema ID                  |
| `key`     | string | Key within the schema                |
| `value`   | string | Value to set (as string)             |

```bash
deskbrid desktop.set '{"schema": "org.gnome.desktop.interface", "key": "gtk-theme", "value": "Adwaita"}'
```

```json
{
  "type": "desktop.set",
  "schema": "org.gnome.desktop.interface",
  "key": "gtk-theme",
  "value": "Adwaita"
}
```

### desktop_settings.list_schemas

List all available GSettings schemas on the system.

```bash
deskbrid desktop.list_schemas
```

```json
{"type": "desktop.list_schemas"}
```

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "schemas": [
      "org.gnome.desktop.interface",
      "org.gnome.desktop.wm.preferences",
      "org.gnome.shell"
    ]
  }
}
```

## Python Example

```python
from deskbrid import Deskbrid
client = Deskbrid()

# Read a setting
theme = client.desktop_get_setting(
    schema="org.gnome.desktop.interface",
    key="gtk-theme",
)
print(f"Current theme: {theme['value']}")

# Change a setting
client.desktop_set_setting(
    schema="org.gnome.desktop.interface",
    key="gtk-theme",
    value="Adwaita-dark",
)
```

## Desktop Backends

| Desktop     | Backend                          |
|-------------|----------------------------------|
| GNOME       | GSettings / DConf                |
| KDE         | KConfig / kwriteconfig           |
| Hyprland    | hyprctl                           |
| Sway        | swaymsg                           |
| COSMIC      | COSMIC settings backend           |
| Labwc       | Openbox-style config             |
| Niri        | Niri IPC                          |
| Wayfire     | Wayfire config                    |
| X11 (other) | xset / xrandr / xresources       |

## Safety

Changing desktop settings can affect the entire desktop experience. Settings
changes are immediate and may not have undo functionality beyond setting back
to a previous value.

## Current Status

**Stable** — read, write, and schema listing supported.
