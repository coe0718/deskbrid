# Terminal PTY Feature

The Terminal PTY feature provides full pseudoterminal (PTY) session management over the Deskbrid socket. Agents can spawn interactive shell sessions, send input, read output, resize the terminal geometry, list active sessions, and kill sessions. This is useful for running long-lived shell commands, interactive programs (editors, REPLs, TUI tools), and any workflow that needs a persistent terminal environment with proper job control and TTY semantics.

## Actions

### terminal.create

Create a new PTY session backed by a pseudoterminal. The spawned command runs under the daemon's user with the environment of the daemon process unless overridden. Returns a session `id` for subsequent operations.

| Parameter | Type | Description |
|-----------|------|-------------|
| `command` | string | The command to run inside the PTY (e.g. `"/bin/bash"`, `"/usr/bin/python3"`). |
| `cwd` | string | Optional. Working directory for the command. Defaults to the daemon's working directory. |
| `env` | object | Optional. Additional environment variables as key-value pairs (e.g. `{"TERM": "xterm-256color"}`). Merged with, but overriding, the daemon's environment. |
| `rows` | uint | Optional. Initial number of terminal rows. Defaults to 24. |
| `cols` | uint | Optional. Initial number of terminal columns. Defaults to 80. |

```bash
deskbrid terminal.create {
  command: "/bin/bash",
  cwd: "/home/user",
  env: { TERM: "xterm-256color" },
  rows: 24,
  cols: 80
}
```

```json
{"type": "terminal.create", "command": "/bin/bash", "cwd": "/home/user", "env": {"TERM": "xterm-256color"}, "rows": 24, "cols": 80}
```

**Response format:**

```json
{
  "id": "pty_01j3..."
}
```

### terminal.write

Write data (raw bytes) to the stdin of a PTY session. This allows sending typed commands, control characters, and arbitrary input to the running process.

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | string | The PTY session id (returned from `terminal.create`). |
| `data` | string | The raw bytes to write to the PTY's stdin. Include `\n` for newlines and control sequences as needed. |

```bash
deskbrid terminal.write {
  id: "pty_01j3...",
  data: "echo hello\n"
}
```

```json
{"type": "terminal.write", "id": "pty_01j3...", "data": "echo hello\n"}
```

### terminal.read

Read available output from a PTY session's stdout/stderr. This is a polling operation — it returns whatever output has accumulated in the PTY buffer since the last read. Use `timeout` to block briefly for more data.

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | string | The PTY session id. |
| `timeout` | uint | Optional. Maximum time in milliseconds to wait for output before returning. Defaults to 0 (return immediately with whatever is available). |

```bash
deskbrid terminal.read {
  id: "pty_01j3...",
  timeout: 1000
}
```

```json
{"type": "terminal.read", "id": "pty_01j3...", "timeout": 1000}
```

**Response format:**

```json
{
  "data": "hello\r\nuser@host:~$ ",
  "eof": false
}
```

The `eof` field is `true` when the underlying process has exited and no more data will be produced.

### terminal.resize

Resize the PTY's terminal dimensions. This sends a `SIGWINCH` to the foreground process group and updates the winsize struct. Programs like `vim`, `top`, and `less` respond to this by redrawing their UI to the new size.

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | string | The PTY session id. |
| `rows` | uint | New number of rows. |
| `cols` | uint | New number of columns. |

```bash
deskbrid terminal.resize {
  id: "pty_01j3...",
  rows: 40,
  cols: 120
}
```

```json
{"type": "terminal.resize", "id": "pty_01j3...", "rows": 40, "cols": 120}
```

### terminal.list

List all currently active PTY sessions managed by the daemon. Returns session metadata including id, command, PID, dimensions, and elapsed time since creation.

No parameters required.

```bash
deskbrid terminal.list {}
```

```json
{"type": "terminal.list"}
```

**Response format:**

```json
{
  "sessions": [
    {
      "id": "pty_01j3...",
      "command": "/bin/bash",
      "pid": 12345,
      "rows": 24,
      "cols": 80,
      "created_at": "2026-06-27T12:00:00Z"
    }
  ]
}
```

### terminal.kill

Kill a PTY session and its child process tree. Sends `SIGHUP` to the PTY's foreground process group by default, which typically terminates the shell and its children gracefully. After kill, the session is removed from the active list.

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | string | The PTY session id to terminate. |

```bash
deskbrid terminal.kill { id: "pty_01j3..." }
```

```json
{"type": "terminal.kill", "id": "pty_01j3..."}
```

## Safety Boundary

- PTY sessions run as the daemon's user — they have the same file system and process permissions as the daemon itself.
- There is no built-in command sandboxing: a spawned `rm -rf /` will do what it says. Use confirmation mode for destructive commands.
- Unread output accumulates in a ring buffer; if the buffer fills, older output is discarded.
- The daemon will reap zombie processes from terminated PTY sessions, but long-lived child processes (orphaned by the shell) may survive daemon restart.
- Always call `terminal.kill` for sessions you no longer need to prevent resource leaks.
- Concurrent reads and writes on the same PTY from multiple agent connections interleave at the PTY buffer level.

## Local Development

Start the daemon in verbose mode, then create a PTY and interact with it:

```bash
deskbrid daemon --verbose
```

```bash
# Create a bash PTY session
echo '{"type":"terminal.create","command":"/bin/bash","env":{"TERM":"xterm"},"rows":24,"cols":80}' | nc -U $XDG_RUNTIME_DIR/deskbrid.sock -w 2

# Write a command
echo '{"type":"terminal.write","id":"pty_01j3...","data":"echo hello world\n"}' | nc -U $XDG_RUNTIME_DIR/deskbrid.sock -w 1

# Read output
echo '{"type":"terminal.read","id":"pty_01j3...","timeout":500}' | nc -U $XDG_RUNTIME_DIR/deskbrid.sock -w 1

# Kill the session
echo '{"type":"terminal.kill","id":"pty_01j3..."}' | nc -U $XDG_RUNTIME_DIR/deskbrid.sock -w 1
```

## Configuration

- **Buffer size**: The per-session output ring buffer size can be configured in the daemon's config file under `terminal.buffer_size` (default: 64KB).
- **Max sessions**: A global limit on concurrent PTY sessions can be set via `terminal.max_sessions` (default: 32).
