# Terminals

Create and manage PTY sessions.

> **Note:** The canonical documentation for terminal/PTY management is in
> [terminal_pty.md](terminal_pty.md). This page is a convenience alias.

## Create Terminal

```bash
deskbrid terminal create --shell /bin/bash --cwd /home/user/project
deskbrid terminal create --shell zsh --rows 24 --cols 80
deskbrid terminal create --env PATH=/usr/local/bin:/usr/bin --env DEBUG=1
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "terminal_id": "term_123456"
  }
}
```

Protocol:
```json
{
  "type": "terminal.create",
  "shell": "/bin/bash",
  "cwd": "/home/user/project",
  "rows": 24,
  "cols": 80
}
```

## List Terminals

```bash
deskbrid terminal list
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {
      "terminal_id": "term_123456",
      "pid": 12345,
      "shell": "/bin/bash",
      "cwd": "/home/user/project"
    }
  ]
}
```

Protocol:
```json
{"type": "terminal.list"}
```

## Write to Terminal

```bash
deskbrid terminal write term_123456 --input "ls -la\n"
deskbrid terminal write term_123456 --input "echo 'Hello'\n"
```

Protocol:
```json
{"type": "terminal.write", "terminal_id": "term_123456", "input": "ls -la\n"}
```

## Read from Terminal

```bash
deskbrid terminal read term_123456
deskbrid terminal read term_123456 --max-bytes 4096 --flush
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "output": "total 16\ndrwxr-xr-x 2 user user 4096 Jan 15 10:00 .\n-rw-r--r-- 1 user user  123 Jan 15 10:00 main.rs\n"
  }
}
```

Protocol:
```json
{
  "type": "terminal.read",
  "terminal_id": "term_123456",
  "max_bytes": 4096,
  "flush": true
}
```

## Resize Terminal

```bash
deskbrid terminal resize term_123456 --rows 40 --cols 120
```

Protocol:
```json
{
  "type": "terminal.resize",
  "terminal_id": "term_123456",
  "rows": 40,
  "cols": 120
}
```

## Kill Terminal

```bash
deskbrid terminal kill term_123456
deskbrid terminal kill term_123456 --signal SIGKILL
deskbrid terminal kill term_123456 --signal SIGTERM
```

Protocol:
```json
{
  "type": "terminal.kill",
  "terminal_id": "term_123456",
  "signal": "SIGTERM"
}
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# Create a terminal
result = client.terminal_create(cwd="/home/user/project")
term_id = result["terminal_id"]

# Run a command
client.terminal_write(term_id, "npm test\n")

# Read output
output = client.terminal_read(term_id)
print(output["output"])

# Clean up
client.terminal_kill(term_id)
```