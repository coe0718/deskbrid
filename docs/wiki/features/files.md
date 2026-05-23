# File Operations

Search for files and watch for changes.

## Search Files

```bash
deskbrid files search --pattern "*.rs" --limit 20
deskbrid files search --pattern "README" --hidden --limit 50
deskbrid files search --pattern "*.png" --mime-type image/png
```

Protocol:
```json
{
  "action": "files.search",
  "pattern": "*.rs",
  "include_hidden": false,
  "limit": 20
}
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {"path": "/home/user/project/src/main.rs", "type": "file", "size": 1234},
    {"path": "/home/user/project/src/lib.rs", "type": "file", "size": 5678}
  ]
}
```

## Watch Files

Watch a path for changes:

```bash
deskbrid files watch /home/user/project --recursive
```

Protocol:
```json
{"action": "files.watch", "path": "/home/user/project", "recursive": true}
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# Find all Python files
results = client.files_search(pattern="*.py", limit=100)
for file in results:
    print(file["path"])
```