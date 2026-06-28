# Unified Search

Search across multiple data sources in a single query. Results are returned
from any configured search provider — files, applications, settings,
documents, and more.

## Actions

### unified.search

Run a search query across all configured search providers.

| Parameter    | Type        | Description                               |
|--------------|-------------|-------------------------------------------|
| `query`      | string      | Search query text                        |
| `categories` | string[]?   | Filter to specific categories (optional)  |
| `limit`      | uint?       | Max results per category (default: 20)   |

```bash
deskbrid unified.search '{"query": "report", "categories": ["files", "apps"], "limit": 10}'
```

```json
{
  "type": "unified.search",
  "query": "budget",
  "categories": ["files", "apps", "settings"],
  "limit": 10
}
```

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "files": [
      {"path": "/home/user/Documents/budget-2024.xlsx", "type": "file", "score": 0.95},
      {"path": "/home/user/Desktop/budget-notes.txt", "type": "file", "score": 0.80}
    ],
    "apps": [
      {"name": "Gnome Calculator", "desktop_id": "org.gnome.Calculator.desktop", "score": 0.10}
    ],
    "settings": []
  }
}
```

### unified.index

Trigger a re-index of all search providers. This refreshes the search index
with the latest data.

```bash
deskbrid unified.index
```

No parameters. May take several seconds depending on the number of files and
providers configured.

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# Search everything
results = client.unified_search(query="meeting notes", limit=5)

for category, items in results.items():
    if items:
        print(f"\n{category}:")
        for item in items:
            if category == "files":
                print(f"  📄 {item['path']}")
            elif category == "apps":
                print(f"  🚀 {item['name']}")
```

## Requirements

- File search requires the `baloo` (KDE) or `tracker3` (GNOME) desktop
  search daemon, or Deskbrid's built-in file indexer.
- App search uses the desktop's `.desktop` file database.
- Settings search requires D-Bus access to the desktop settings daemon.
- `unified.index` may require write access to the index storage directory.

## Configuration

Search providers and their priorities are configured in `config.toml`:

```toml
[search]
index_path = "/var/lib/deskbrid/search"
providers = ["files", "apps", "settings", "documents"]
max_results_per_provider = 20
reindex_interval_hours = 24
```

## Current Status

**Experimental** — v1.0.0 feature.
