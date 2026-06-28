# Process Feature

The Process feature provides direct process lifecycle management on the host system. Agents can list running processes, start new background processes, stop them by PID, send arbitrary Unix signals, check whether a process exists (by PID or name), and block until a process exits. Unlike the Terminal PTY feature, processes started here have no PTY attached — they run as detached background processes with their stdio optionally captured. This is useful for launching servers, running one-shot commands, monitoring process health, and managing system services.

## Actions

### process.list

List all processes visible to the daemon (subject to the daemon's user permissions). Returns process metadata including PID, name, command line, CPU and memory usage, and state.

No parameters required.

```bash
deskbrid process.list {}
```

```json
{"type": "process.list"}
```

**Response format:**

```json
{
  "processes": [
    {
      "pid": 1234,
      "name": "bash",
      "command": "/bin/bash",
      "cpu_percent": 0.5,
      "memory_percent": 0.1,
      "state": "running"
    }
  ]
}
```

### process.start

Start a new background process on the host. The process is spawned directly (no PTY), so it runs detached from any terminal. Stdio can optionally be piped to the daemon for later retrieval. Returns the PID of the spawned process.

| Parameter | Type | Description |
|-----------|------|-------------|
| `command` | string | The executable or script to run (e.g. `"python3"`, `"/usr/bin/sleep"`). |
| `args` | array of strings | Arguments to pass to the command. Each element is a single argument (e.g. `["-m", "http.server", "8080"]`). |
| `cwd` | string | Optional. Working directory for the process. Defaults to the daemon's working directory. |
| `env` | object | Optional. Additional environment variables as key-value pairs (e.g. `{"MY_VAR": "value"}`). Merged with the daemon's environment, with these values taking precedence. |

```bash
deskbrid process.start {
  command: "python3",
  args: ["-m", "http.server", "8080"],
  cwd: "/home/user/projects",
  env: { PYTHONUNBUFFERED: "1" }
}
```

```json
{"type": "process.start", "command": "python3", "args": ["-m", "http.server", "8080"], "cwd": "/home/user/projects", "env": {"PYTHONUNBUFFERED": "1"}}
```

**Response format:**

```json
{
  "pid": 12345
}
```

### process.stop

Stop a running process by PID. Sends `SIGTERM` by default, giving the process a chance to clean up. If the process does not exit after a short grace period, `SIGKILL` is sent. Returns success once the process is confirmed dead.

| Parameter | Type | Description |
|-----------|------|-------------|
| `pid` | uint | The process ID to terminate. |

```bash
deskbrid process.stop { pid: 12345 }
```

```json
{"type": "process.stop", "pid": 12345}
```

### process.signal

Send an arbitrary Unix signal to a process by PID. Useful for custom signal handling beyond simple stop (e.g. `SIGHUP` for re-reading configs, `SIGUSR1`/`SIGUSR2` for application-specific triggers).

| Parameter | Type | Description |
|-----------|------|-------------|
| `pid` | uint | The process ID to signal. |
| `signal` | string | The signal name (case-insensitive, without the `SIG` prefix). Supported values: `HUP`, `INT`, `QUIT`, `ILL`, `TRAP`, `ABRT`, `BUS`, `FPE`, `KILL`, `USR1`, `SEGV`, `USR2`, `PIPE`, `ALRM`, `TERM`, `STKFLT`, `CHLD`, `CONT`, `STOP`, `TSTP`, `TTIN`, `TTOU`, `URG`, `XCPU`, `XFSZ`, `VTALRM`, `PROF`, `WINCH`, `IO`, `PWR`, `SYS`. |

```bash
deskbrid process.signal { pid: 12345, signal: "HUP" }
```

```json
{"type": "process.signal", "pid": 12345, "signal": "HUP"}
```

### process.exists

Check whether a process exists on the system. Accepts either a PID or a process name. When searching by name, matches any process whose command name contains the given string. Returns a boolean indicating existence.

| Parameter | Type | Description |
|-----------|------|-------------|
| `pid` | uint | Optional. The process ID to check. Mutually exclusive with `name`. |
| `name` | string | Optional. A process name substring to search for. Mutually exclusive with `pid`. |

```bash
deskbrid process.exists { pid: 12345 }
```

```bash
deskbrid process.exists { name: "python3" }
```

```json
{"type": "process.exists", "pid": 12345}
```

```json
{"type": "process.exists", "name": "python3"}
```

**Response format:**

```json
{
  "exists": true
}
```

### process.wait

Block (up to an optional timeout) until the specified process exits. Useful when an agent needs to wait for a spawned process to complete before proceeding. Returns the exit status (or a timeout indicator).

| Parameter | Type | Description |
|-----------|------|-------------|
| `pid` | uint | The process ID to wait for. |
| `timeout` | uint | Optional. Maximum time in milliseconds to wait for the process to exit. If omitted, waits indefinitely. |

```bash
deskbrid process.wait { pid: 12345, timeout: 30000 }
```

```json
{"type": "process.wait", "pid": 12345, "timeout": 30000}
```

**Response format:**

```json
{
  "exited": true,
  "exit_code": 0,
  "timed_out": false
}
```

If the timeout expires before the process exits, `timed_out` is `true` and the process continues running.

## Safety Boundary

- Processes run with the full permissions of the daemon user — there is no sandboxing. Spawning `sudo` or privileged commands will only work if the daemon user has password-less sudo or the appropriate capabilities.
- `process.stop` uses a SIGTERM → grace period → SIGKILL escalation. For immediate termination, use `process.signal` with `KILL`.
- If the daemon exits, all child processes started via `process.start` may be re-parented to init (PID 1) unless the daemon explicitly reaps them.
- There is no built-in resource limit (CPU, memory, file descriptors) on spawned processes. Set system-level ulimits or use `systemd-run --scope` for resource-constrained environments.
- Sending signals to PID 1 (`init` or `systemd`) is blocked; most other PIDs are allowed subject to the daemon user's permission.

## Local Development

Start the daemon in verbose mode, then test process lifecycle:

```bash
deskbrid daemon --verbose
```

```bash
# Start a long-running process
echo '{"type":"process.start","command":"sleep","args":["60"]}' | nc -U $XDG_RUNTIME_DIR/deskbrid.sock -w 2

# Check if it exists
echo '{"type":"process.exists","pid":12345}' | nc -U $XDG_RUNTIME_DIR/deskbrid.sock -w 1

# List all processes
echo '{"type":"process.list"}' | nc -U $XDG_RUNTIME_DIR/deskbrid.sock -w 2

# Signal it
echo '{"type":"process.signal","pid":12345,"signal":"STOP"}' | nc -U $XDG_RUNTIME_DIR/deskbrid.sock -w 1
echo '{"type":"process.signal","pid":12345,"signal":"CONT"}' | nc -U $XDG_RUNTIME_DIR/deskbrid.sock -w 1

# Stop it
echo '{"type":"process.stop","pid":12345}' | nc -U $XDG_RUNTIME_DIR/deskbrid.sock -w 2

# Start and wait
echo '{"type":"process.start","command":"sleep","args":["2"]}' | nc -U $XDG_RUNTIME_DIR/deskbrid.sock -w 1
echo '{"type":"process.wait","pid":12346,"timeout":5000}' | nc -U $XDG_RUNTIME_DIR/deskbrid.sock -w 5
```

## Configuration

- **Process tracking**: The daemon keeps a process table of spawned children. Max tracked processes can be configured via `process.max_tracked` (default: 256).
- **Stdio capture**: Whether to capture stdout/stderr of spawned processes (for later retrieval) can be set via `process.capture_stdio` (default: false).
- **Default stop timeout**: Configurable via `process.stop_timeout_ms` (default: 3000ms) — the grace period before SIGKILL.
