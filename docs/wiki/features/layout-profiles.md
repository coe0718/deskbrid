# Layout Profiles

Save and restore window layout configurations.

## List Profiles

```bash
deskbrid profiles list
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {
      "name": "coding",
      "created": "2024-01-15T10:00:00Z",
      "windows": 3
    },
    {
      "name": "presentation",
      "created": "2024-01-14T15:30:00Z",
      "windows": 1
    }
  ]
}
```

Protocol:
```json
{"action": "layout_profiles.list"}
```

## Save Profile

```bash
deskbrid profiles save coding
deskbrid profiles save presentation --overwrite
```

Protocol:
```json
{"action": "layout_profiles.save", "name": "coding", "overwrite": false}
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "name": "coding",
    "windows_saved": 3
  }
}
```

## Get Profile Details

```bash
deskbrid profiles get coding
```

Protocol:
```json
{"action": "layout_profiles.get", "name": "coding"}
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "name": "coding",
    "created": "2024-01-15T10:00:00Z",
    "monitors": [
      {
        "windows": [
          {
            "app_id": "code",
            "title": "main.rs - project",
            "x": 0,
            "y": 0,
            "width": 1280,
            "height": 1440
          }
        ]
      }
    ]
  }
}
```

## Restore Profile

```bash
deskbrid profiles restore coding
deskbrid profiles restore presentation
```

Protocol:
```json
{"action": "layout_profiles.restore", "name": "coding"}
```

## Delete Profile

```bash
deskbrid profiles delete old-profile
```

Protocol:
```json
{"action": "layout_profiles.delete", "name": "old-profile"}
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# Save current layout
client.save_layout_profile("work", overwrite=True)

# Later, restore it
client.restore_layout_profile("work")
```

## Typical Workflow

1. Arrange your windows the way you like
2. `deskbrid profiles save myprofile`
3. Later or on a different session: `deskbrid profiles restore myprofile`

Profiles include:
- Window positions and sizes
- Workspace assignments
- Monitor layout

## Use Cases

- **Development**: Save your editor + terminal + browser layout
- **Streaming**: Quick switch to presentation mode
- **Coding**: Restore your 3-pane layout with terminals in specific positions