# Screen Recording

Deskbrid v1.0.0 includes PipeWire-based screen recording via the GNOME ScreenCast
portal, with a real-time web dashboard for monitoring and control.

- Real-time video capture via PipeWire
- Web dashboard at `http://localhost:20129` for monitoring and control
- Event broadcasting on `screencast.*` patterns for real-time updates
- MCP-exposed actions for AI agent control

## Start Recording

```bash
deskbrid screencast.start { output_path: "/tmp/recording.mp4" }
```

Parameters:
- `output_path`: file path (required). Supports `%Y%m%d_%H%M%S` strftime.
- `audio`: boolean (default: false)
- `camera`: boolean (default: false)
- `cursor`: boolean (default: true)

## Stop Recording

```bash
deskbrid screencast.stop
```

## Web Dashboard

The dashboard at `http://localhost:20129` shows:
- Live capture preview
- Recording status and duration
- Stop controls
- Audio level visualization
- Frame rate and resolution

## Events

Subscribe to:
- `screencast.started`
- `screencast.stopped`
- `screencast.frame`
- `screencast.error`

```json
{"type":"event.subscribe","events":["screencast.*"]}
```
