# Apps

List, search, and inspect installed desktop applications. Apps
are discovered through `.desktop` files on the system.

## Actions

### apps.list

List installed applications, optionally filtered by category or MIME type.

| Parameter        | Type     | Description                                           |
|------------------|----------|-------------------------------------------------------|
| `categories`     | string[] | Filter by desktop categories (e.g., `Development`, `Network`) |
| `mime_types`     | string[] | Filter by supported MIME types (e.g., `text/plain`)   |
| `include_hidden` | bool     | Include applications marked `NoDisplay=true`           |
| `limit`          | uint?    | Max results to return                                 |

```bash
deskbrid apps list
deskbrid apps list --category Development --limit 10
```

```json
{"type": "apps.list", "categories": ["Development"], "limit": 10}
```

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {
      "app_id": "code.desktop",
      "name": "Visual Studio Code",
      "exec": "/usr/bin/code",
      "categories": ["Development", "TextEditor"]
    }
  ]
}
```

### apps.search

Search installed applications by name or keywords.

| Parameter | Type     | Description                 |
|-----------|----------|-----------------------------|
| `query`   | string   | Search query                |
| `limit`   | uint?    | Max results to return       |

```bash
deskbrid apps search "code"
```

```json
{"type": "apps.search", "query": "code", "limit": 5}
```

### apps.get

Get details about a specific application by its app ID.

| Parameter | Type   | Description                          |
|-----------|--------|--------------------------------------|
| `app_id`  | string | Application ID (e.g., `code.desktop`)|

```bash
deskbrid apps get code.desktop
```

```json
{"type": "apps.get", "app_id": "code.desktop"}
```

Response includes full `.desktop` entry data: name, exec, icon, categories, MIME types, and any X-GNOME/X-KDE properties.

## Python Example

```python
from deskbrid import Deskbrid
client = Deskbrid()

# List development apps
dev_apps = client.app_list(categories=["Development"])
for app in dev_apps:
    print(f"{app['name']} ({app['app_id']})")

# Search for a specific app
results = client.app_search(query="terminal")
print(results)

# Get app details
code = client.app_get(app_id="code.desktop")
print(code["exec"])
```

## Requirements

- `.desktop` files are read from standard locations (`/usr/share/applications/`, `~/.local/share/applications/`)
- No external dependencies — pure Freedesktop standards

## Current Status

**Stable** — listing, searching, and getting app details.
