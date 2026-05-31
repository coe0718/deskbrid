# Screen Recording

Deskbrid v0.11.0 adds PipeWire-based screen recording capabilities via the GNOME ScreenCast portal, with a real-time web dashboard for monitoring and control.

## Overview

Screen recording uses the PipeWire multimedia framework and GNOME ScreenCast portal for secure, permission-based capture. Features include:
- Real-time video capture via PipeWire
- Web dashboard at http://localhost:4199 for monitoring and control
- Event broadcasting on the subscription bus for real-time updates
- MCP tools for remote control via AI agents

## Start Recording

```bash
deskbrid screencast.start { output_path: "/tmp/recording.mp4" }
```

### Parameters

- `output_path`: File path for the recording (required). Supports strftime format specifiers like `%Y%m%d_%H%M%S`.
- `audio`: Whether to capture audio (boolean, default: false)
- `camera`: Whether to include camera feed (boolean, default: false)
- `cursor`: Whether to show cursor (boolean, default: true)

## Stop Recording

```bash
deskbrid screencast.stop
```

## Web Dashboard

When screen recording is active, a web dashboard is available at:
- URL: http://localhost:4199
- Features:
  - Real-time preview of the capture
  - Recording status and duration
  - Controls to stop recording
  - Audio levels visualization
  - Frame rate and resolution info

The dashboard binds to `0.0.0.0` by default for LAN access (can be changed via configuration).

## Events

Screen recording broadcasts events on the subscription bus:
- `screencast.started` - When recording begins
- `screencast.stopped` - When recording ends
- `screencast.frame` - Periodic frame metadata (timestamp, resolution, etc.)
- `screencast.error` - If an error occurs during capture

### Example Event
```json
{
  "type": "event",
  "event": "screencast.started",
  "data": {
    "output_path": "/tmp/recording.mp4",
    "width": 1920,
    "height": 1080,
    "fps": 30
  }
}
```

## MCP Tools

The following MCP tools are available for screen recording control:
- `screencast_start` - Start recording
- `screencast_stop` - Stop recording
- `screencast_status` - Get recording status

## Python Example

```python
from deskbrid import Deskbrid
import time

client = Deskbrid()

# Start recording with audio
client.screencast_start(
    output_path="/tmp/desktop_recording_%Y%m%d_%H%M%S.mp4",
    audio=True,
    cursor=True
)

print("Recording started... (will run for 10 seconds)")
time.sleep(10)

# Stop recording
client.screencast_stop()
print("Recording saved!")

# Check status via events (simplified example)
# In practice, you'd subscribe to screencast.* events
```

## Configuration

The web dashboard port can be configured via environment variable:
```bash
DESKBRID_WEB_PORT=8080 deskbrid daemon
```

## Notes

- Requires PipeWire and GNOME ScreenCast portal dependencies
- On first use, you'll be prompted to grant screen recording permission
- The web dashboard is intended for monitoring and light control - heavy processing should be done elsewhere
- Audio capture requires additional setup depending on your system (PipeWire configuration)