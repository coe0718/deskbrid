# Macro Recording and Replay

Record sequences of desktop interactions (mouse movements, clicks, keyboard input, window operations) and replay them on demand. Use this for automating repetitive workflows, demoing procedures, setting up complex multi-step development environments, or creating reproducible test scenarios for desktop applications.

## Actions

### macro.record.start

Begin recording a new macro. All subsequent desktop input events (mouse, keyboard, window focus changes) are captured until `macro.record.stop` is called.

| Parameter | Type | Description |
|-----------|------|-------------|
| `name` | string | A unique name to identify the macro (e.g. `"deploy-steps"`). Used later for replay and management. |

```bash
deskbrid macro.record.start '{ "name": "deploy-steps" }'
```

```json
{"type": "macro.record.start", "name": "deploy-steps"}
```

**Response:** Returns `{"success": true, "name": "deploy-steps"}`. Recording begins immediately.

### macro.record.stop

Stop the active recording session and save the captured macro events.

| Parameter | Type | Description |
|-----------|------|-------------|
| *None* | | This action takes no parameters. |

```bash
deskbrid macro.record.stop '{}'
```

```json
{"type": "macro.record.stop"}
```

**Response:** Returns `{"success": true, "name": "<macro-name>", "event_count": <int>}` with the number of captured events.

### macro.replay

Replay a previously recorded macro at the specified speed.

| Parameter | Type | Description |
|-----------|------|-------------|
| `name` | string | The name of the macro to replay. |
| `speed` | float | *Optional.* Playback speed multiplier. `1.0` = real-time (default). `2.0` = double speed. `0.5` = half speed. |

```bash
deskbrid macro.replay '{ "name": "deploy-steps", "speed": 1.5 }'
```

```json
{"type": "macro.replay", "name": "deploy-steps", "speed": 1.5}
```

**Response:** Returns `{"success": true, "name": "deploy-steps", "event_count": <int>}` when replay completes.

### macro.list

List all saved macros with metadata.

| Parameter | Type | Description |
|-----------|------|-------------|
| *None* | | This action takes no parameters. |

```bash
deskbrid macro.list '{}'
```

```json
{"type": "macro.list"}
```

**Response:** Returns an array of macro objects, each with `name` (string), `created_at` (ISO timestamp), `updated_at` (ISO timestamp), `event_count` (int), and `duration_seconds` (int, approximate recorded duration).

### macro.export

Export a macro's event data as a portable JSON string. Useful for sharing, version control, or backup.

| Parameter | Type | Description |
|-----------|------|-------------|
| `name` | string | The name of the macro to export. |

```bash
deskbrid macro.export '{ "name": "deploy-steps" }'
```

```json
{"type": "macro.export", "name": "deploy-steps"}
```

**Response:** Returns a JSON object with the macro's full event sequence, including `name`, `events` (array of timestamped input events), `version`, and `metadata`.

### macro.import

Import a previously exported macro from a JSON string.

| Parameter | Type | Description |
|-----------|------|-------------|
| `name` | string | The name to give the imported macro (can differ from the original). |
| `data` | string | The JSON string containing the exported macro data (as produced by `macro.export`). |

```bash
deskbrid macro.import '{ "name": "shared-setup", "data": "{\"events\":[...]}" }'
```

```json
{"type": "macro.import", "name": "shared-setup", "data": "{\"events\":[...]}"}
```

**Response:** Returns `{"success": true, "name": "shared-setup", "event_count": <int>}`.

## Safety Boundary

- Macro replay **simulates real input events** using uinput (or equivalent). This means the agent can move the mouse, click, and type as if a human user were at the keyboard. Treat macro replay with the same security caution as granting remote desktop access.
- Recording captures **all input events** during the session, including keystrokes that may contain passwords, API keys, or other sensitive data. Exported macro files contain this data in plaintext — handle and store them securely.
- A macro recorded on one screen resolution / monitor layout may behave unpredictably when replayed on a different layout, especially for mouse-move events with absolute coordinates.
- Confirmation mode is recommended for `macro.replay` unless the macro is known to be safe.
- Stored macros persist in Deskbrid's data directory — consider encrypting or clearing macros that contain sensitive information.

## Local Development

1. Ensure Deskbrid daemon is running: `deskbrid daemon`
2. Record a simple macro:
   ```bash
   deskbrid macro.record.start '{ "name": "hello-world" }'
   ```
3. Perform a few desktop actions (move mouse, open a text editor, type "hello", close).
4. Stop recording:
   ```bash
   deskbrid macro.record.stop '{}'
   ```
5. List saved macros:
   ```bash
   deskbrid macro.list '{}'
   ```
6. Replay the macro:
   ```bash
   deskbrid macro.replay '{ "name": "hello-world" }'
   ```
7. Export and re-import:
   ```bash
   deskbrid macro.export '{ "name": "hello-world" }' | tee /tmp/macro.json
   deskbrid macro.import '{ "name": "hello-imported", "data": "'"$(cat /tmp/macro.json)"'" }'
   ```

## Configuration

Macros are stored in Deskbrid's data directory under `~/.local/share/deskbrid/macros/` by default. No additional configuration is required. Event capture is provided by the same uinput backend used for keyboard/mouse input features.
