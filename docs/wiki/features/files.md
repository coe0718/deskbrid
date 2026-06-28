# File Operations

Search, read, write, watch, and manage files on the local filesystem.

## Actions

### files.list

List files and directories at a given path.

| Parameter | Type   | Description                     |
|-----------|--------|---------------------------------|
| `path`    | string | Directory path to list           |

```bash
deskbrid files.list /home/user/project
```

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {"name": "src", "path": "/home/user/project/src", "type": "dir", "size": 4096, "modified": "2024-01-15T10:00:00Z"},
    {"name": "main.rs", "path": "/home/user/project/src/main.rs", "type": "file", "size": 1234, "modified": "2024-01-14T15:30:00Z"}
  ]
}
```

### files.read

Read a file's contents with optional byte offset and limit.

| Parameter | Type   | Description                             |
|-----------|--------|-----------------------------------------|
| `path`    | string | Absolute path to the file               |
| `offset`  | uint?  | Byte offset to start reading from (optional, default 0) |
| `limit`   | uint?  | Max bytes to read (optional, default no limit) |

```bash
deskbrid files.read /home/user/project/main.rs
deskbrid files.read /home/user/project/main.rs?offset=100&limit=500
```

```json
{
  "type": "files.read",
  "path": "/home/user/project/main.rs",
  "offset": 100,
  "limit": 500
}
```

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "content": "fn main() {\n    println!(\"Hello\");\n}\n",
    "size": 1234,
    "truncated": false
  }
}
```

### files.write

Write content to a file, creating it if it doesn't exist.

| Parameter | Type    | Description                                 |
|-----------|---------|---------------------------------------------|
| `path`    | string  | Absolute path to the file                   |
| `content` | string  | Content to write                            |
| `append`  | bool    | If true, append to existing content          |

```bash
deskbrid files.write /home/user/project/main.rs "fn main() {}"
deskbrid files.write /home/user/project/log.txt "new log entry" --append
```

```json
{
  "type": "files.write",
  "path": "/home/user/project/log.txt",
  "content": "new log entry\n",
  "append": true
}
```

### files.copy

Copy a file or directory from source to destination.

| Parameter     | Type   | Description           |
|---------------|--------|-----------------------|
| `source`      | string | Source path           |
| `destination` | string | Destination path      |

```bash
deskbrid files.copy main.rs main.backup.rs
deskbrid files.copy /home/user/project /home/user/project-backup
```

```json
{
  "type": "files.copy",
  "source": "/home/user/project/main.rs",
  "destination": "/home/user/project/main.backup.rs"
}
```

### files.move

Move (rename) a file or directory.

| Parameter     | Type   | Description      |
|---------------|--------|------------------|
| `source`      | string | Current path     |
| `destination` | string | New path         |

```bash
deskbrid files.move main.rs src/main.rs
```

```json
{
  "type": "files.move",
  "source": "/home/user/project/main.rs",
  "destination": "/home/user/project/src/main.rs"
}
```

### files.delete

Delete a file or directory.

| Parameter   | Type    | Description                                   |
|-------------|---------|-----------------------------------------------|
| `path`      | string  | Path to delete                                |
| `recursive` | bool    | If true, delete directories recursively        |

```bash
deskbrid files.delete /home/user/project/tmp.txt
deskbrid files.delete /home/user/project/old-dir --recursive
```

```json
{
  "type": "files.delete",
  "path": "/home/user/project/old-dir",
  "recursive": true
}
```

### files.mkdir

Create a directory.

| Parameter | Type    | Description                                     |
|-----------|---------|-------------------------------------------------|
| `path`    | string  | Directory path to create                        |
| `parents` | bool    | If true, create parent directories as needed     |

```bash
deskbrid files.mkdir /home/user/project/src/utils
deskbrid files.mkdir /home/user/project/a/b/c --parents
```

```json
{
  "type": "files.mkdir",
  "path": "/home/user/project/a/b/c",
  "parents": true
}
```

### files.search

Search for files matching a glob pattern, with optional root directory and
result limit.

| Parameter     | Type    | Description                              |
|---------------|---------|------------------------------------------|
| `pattern`     | string  | Glob pattern (e.g. `"*.rs"`, `"**/*.toml"`) |
| `root`        | string? | Root directory to search (default: `/`)    |
| `max_results` | uint    | Maximum number of results to return        |

```bash
deskbrid files.search "*.rs" --root /home/user/project --max-results 20
```

```json
{
  "type": "files.search",
  "pattern": "*.rs",
  "root": "/home/user/project",
  "max_results": 20
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

### files.watch

Watch a path for filesystem changes and receive events via SSE or callback.

| Parameter   | Type           | Description                                   |
|-------------|----------------|-----------------------------------------------|
| `path`      | string         | Directory to watch                             |
| `recursive` | bool           | Watch subdirectories                           |
| `patterns`  | string[]?      | Optional glob filter (e.g. `["*.rs", "*.toml"]`) |

```bash
deskbrid files.watch /home/user/project --recursive
```

```json
{
  "type": "files.watch",
  "path": "/home/user/project",
  "recursive": true,
  "patterns": ["*.rs", "*.toml"]
}
```

### files.unwatch

Stop watching a previously watched path.

| Parameter | Type   | Description      |
|-----------|--------|------------------|
| `path`    | string | Path to unwatch  |

```bash
deskbrid files.unwatch /home/user/project
```

```json
{
  "type": "files.unwatch",
  "path": "/home/user/project"
}
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# List directory
entries = client.files_list("/home/user/project")
for e in entries:
    print(e["name"], e["type"])

# Read file
content = client.files_read("/home/user/project/main.rs")
print(content)

# Write file
client.files_write("/home/user/project/output.txt", "Hello, world!\n")

# Search
results = client.files_search("*.toml", root="/home/user/project", max_results=10)
for r in results:
    print(r["path"])
```

## Requirements

- All paths must be absolute.
- Tokio's `fs` module handles all I/O asynchronously.
- File watching uses the `notify` crate (inotify on Linux).

## Safety Boundary

- Files outside the agent's configured allowed path (if set in
  `permissions.toml`) are rejected.
- Write, delete, and recursive operations may require confirmation mode.
- Symlinks are followed but may be restricted to avoid escapes.

## Current Status

**Stable** — all 10 file operations.
