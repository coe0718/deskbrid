# Audio

List audio sinks and set sink volume via the unified backend protocol.

This page covers the current v1.0.0 audio surface. Older docs in this repo may refer to audio as a separate `services.audio` module; audio is now exposed through unified backend actions and MCP tooling.

## Requirements

- PulseAudio or PipeWire must be active.
- Most GNOME, KDE, Hyprland, and X11 sessions ship with the required `pactl` or PipeWire utilities by default.

## Actions

- `audio.list_sinks`
- `audio.set_sink_volume`

## Usage

### List sinks

Request the current sinks from the daemon. The daemon returns the sink names and status reported by the active backend. If the backend does not expose audio sinks, the response is empty.

```bash
deskbrid audio list_sinks
```

Example response:

```json
{
  "type": "response",
  "id": "audio-1",
  "seq": 1,
  "status": "ok",
  "data": [
    {
      "name": "alsa_output.pci-0000_00_1f.3.analog-stereo",
      "description": "Built-in Audio Analog Stereo",
      "volume": 75,
      "muted": false,
      "default": true
    }
  ]
}
```

### Set sink volume

Set the volume for a named sink.

```bash
deskbrid audio set_sink_volume --sink alsa_output.pci-0000_00_1f.3.analog-stereo --volume 50
```

Expected response:

```json
{
  "type": "response",
  "id": "audio-2",
  "seq": 2,
  "status": "ok",
  "data": {
    "sink": "alsa_output.pci-0000_00_1f.3.analog-stereo",
    "volume": 50
  }
}
```

If the requested sink name is not found, the action returns `NOT_FOUND`.

## MCP

The MCP server exposes audio sink listing and control through tools derived from `audio.list_sinks` and `audio.set_sink_volume`. See `docs/PROTOCOL.md` and `docs/API.md` for the protocol forms used by the server.

## Notes

- Sink identifiers are backend-reported and can change between desktop sessions or device reboots. Re-run `audio.list_sinks` before volume changes if you are scripting around a fixed sink name.
- Desktop panels may briefly lag after a sink volume change, especially on GNOME.
- Some backend targets do not implement this action. Confirm action availability with `system.capabilities` on your desktop.
