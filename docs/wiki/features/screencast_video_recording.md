# Screencast Video Recording

Record the desktop as a video using the PipeWire-based GNOME ScreenCast portal (xdg-desktop-portal). Use this feature to capture full-screen or region-based video recordings for demos, bug reports, tutorials, or visual context for AI agents that need to observe desktop state over time.

## Actions

### screencast.list

List all currently active screencast recording sessions.

| Parameter | Type | Description |
|-----------|------|-------------|
| *None* | | This action takes no parameters. |

```bash
deskbrid screencast.list '{}'
```

```json
{"type": "screencast.list"}
```

**Response:** Returns an array of active screencast session objects, each with `id` (string), `monitor` (int), `area` (object with x/y/w/h), `fps` (int), `with_audio` (bool), `started_at` (ISO timestamp), and `output_path` (string, if set).

### screencast.start

Start a new screencast recording session via the PipeWire/portal backend.

| Parameter | Type | Description |
|-----------|------|-------------|
| `monitor` | uint | *Optional.* Zero-indexed monitor number to record. If omitted, the primary monitor is used. |
| `area` | object | *Optional.* Rectangular region to record, specified as `{"x": 0, "y": 0, "w": 1920, "h": 1080}`. If omitted, the full monitor area is recorded. |
| `with_audio` | bool | *Optional.* Whether to capture system audio alongside the video. Default: `false`. |
| `fps` | uint | *Optional.* Frames per second for the recording. Default: `30`. Typical values: `15`, `30`, `60`. |

```bash
deskbrid screencast.start '{ "monitor": 0, "with_audio": true, "fps": 30 }'
```

```json
{"type": "screencast.start", "monitor": 0, "with_audio": true, "fps": 30}
```

**Response:** Returns `{"success": true, "id": "<session-id>", "monitor": 0, "fps": 30, "with_audio": true}`.

### screencast.stop

Stop an active screencast recording session and finalize the output file.

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | string | The session ID returned by `screencast.start`. |

```bash
deskbrid screencast.stop '{ "id": "<session-id>" }'
```

```json
{"type": "screencast.stop", "id": "<session-id>"}
```

**Response:** Returns `{"success": true, "id": "<session-id>", "output_path": "/path/to/recording.mp4", "duration_seconds": <float>, "file_size_bytes": <int>}`.

### portal.screencast.start

Start a screencast using the xdg-desktop-portal screencast interface directly. This is an alternative to the PipeWire-native `screencast.start` and may behave differently across desktop environments.

| Parameter | Type | Description |
|-----------|------|-------------|
| `monitor` | uint | *Optional.* Zero-indexed monitor number to record. |
| `area` | object | *Optional.* Recording region `{"x": ..., "y": ..., "w": ..., "h": ...}`. |
| `with_audio` | bool | *Optional.* Capture system audio. Default: `false`. |
| `fps` | uint | *Optional.* Frames per second. Default: `30`. |

```bash
deskbrid portal.screencast.start '{ "monitor": 0 }'
```

```json
{"type": "portal.screencast.start", "monitor": 0}
```

**Response:** Same format as `screencast.start`. Note that portal-based recording may prompt the user for permission via a system dialog depending on the desktop environment.

### portal.screencast.stop

Stop a portal-based screencast session.

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | string | The session ID returned by `portal.screencast.start`. |

```bash
deskbrid portal.screencast.stop '{ "id": "<session-id>" }'
```

```json
{"type": "portal.screencast.stop", "id": "<session-id>"}
```

**Response:** Same format as `screencast.stop`.

## Safety Boundary

- Screencast recording requires the **GNOME ScreenCast portal** (`org.freedesktop.portal.ScreenCast`) which is part of xdg-desktop-portal. This must be installed and running in the user session.
- On GNOME, the first screencast may trigger a **user-facing permission dialog** asking for screen capture authorization. Subsequent sessions may re-prompt depending on the portal's persistent permission settings.
- Recorded video files may contain **sensitive information** visible on screen (passwords, emails, private messages, API keys in terminals). Handle output files with appropriate security.
- Recording with `with_audio: true` captures system audio, which may include microphone input, notification sounds, or media playback.

## Local Development

1. Verify xdg-desktop-portal is running:
   ```bash
   systemctl --user status xdg-desktop-portal
   ```
2. Ensure PipeWire session is active:
   ```bash
   pw-cli info all | head
   ```
3. Start Deskbrid daemon: `deskbrid daemon`
4. Start a test recording:
   ```bash
   deskbrid screencast.start '{ "fps": 15, "with_audio": false }'
   ```
5. Wait a few seconds, then stop:
   ```bash
   deskbrid screencast.list '{}'
   deskbrid screencast.stop '{ "id": "<session-id>" }'
   ```
6. Check the output file (path returned in the stop response):
   ```bash
   ffprobe /path/to/recording.mp4
   ```
7. Test the portal-based alternative:
   ```bash
   deskbrid portal.screencast.start '{ "fps": 15 }'
   ```

## Configuration

- **PipeWire backend** (`screencast.*`): Uses PipeWire's pw-stream API directly. Requires `pipewire` and `pipewire-pulse` to be running. Typically no extra portal prompts.
- **Portal backend** (`portal.screencast.*`): Uses `org.freedesktop.portal.ScreenCast` D-Bus API. May trigger a permission dialog. Configured via the desktop environment's portal settings (GNOME Settings > Privacy > Screen Capture, or KDE System Settings).
- Output files are stored in `~/Videos/` by default, or a temporary directory if `~/Videos/` does not exist. The exact path is returned in the stop response.
- Check `~/.local/share/deskbrid/screencast.log` if recordings fail to start.
