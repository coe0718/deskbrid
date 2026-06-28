# Audio

List, configure, and control audio sinks and sources via PulseAudio/PipeWire.

## Actions

### audio.list_sinks

List all audio output sinks (speakers, headphones, etc.).

```bash
deskbrid audio.list_sinks
```

No parameters.

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {"id": 0, "name": "alsa_output.pci-0000_00_1f.3.analog-stereo", "description": "Built-in Audio Analog Stereo", "volume": 0.75, "muted": false, "default": true},
    {"id": 1, "name": "bluez_sink.XX_XX_XX_XX_XX_XX.a2dp_sink", "description": "Sony WH-1000XM4", "volume": 0.50, "muted": false, "default": false}
  ]
}
```

### audio.list_sources

List all audio input sources (microphones, line-in).

```bash
deskbrid audio.list_sources
```

No parameters.

```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {"id": 0, "name": "alsa_input.pci-0000_00_1f.3.analog-stereo", "description": "Built-in Audio Analog Stereo", "volume": 0.80, "muted": false, "default": true}
  ]
}
```

### audio.set_sink_volume

Set a specific sink's volume by its numeric ID.

| Parameter | Type   | Description                                  |
|-----------|--------|----------------------------------------------|
| `sink_id` | uint   | Sink numeric ID (from audio.list_sinks)      |
| `volume`  | float  | Volume level, 0.0 (mute) to 1.0 (max)        |

```bash
deskbrid audio.set_sink_volume '{"sink_id": 0, "volume": 0.5}'
```

### audio.get_volume

Get the volume level of a specific sink or source.

| Parameter | Type   | Description                       |
|-----------|--------|-----------------------------------|
| `target`  | string | `"sink"` or `"source"`            |
| `id`      | uint   | Device numeric ID                 |

```bash
deskbrid audio.get_volume '{"target": "sink", "id": 0}'
```

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": {"target": "sink", "id": 0, "volume": 0.5}
}
```

### audio.set_volume

Set the volume of a specific sink or source.

| Parameter | Type   | Description                       |
|-----------|--------|-----------------------------------|
| `target`  | string | `"sink"` or `"source"`            |
| `id`      | uint   | Device numeric ID                 |
| `volume`  | float  | Volume 0.0 to 1.0                 |

```bash
deskbrid audio.set_volume '{"target": "sink", "id": 0, "volume": 0.3}'
```

### audio.mute

Mute or unmute a specific sink or source.

| Parameter | Type   | Description                       |
|-----------|--------|-----------------------------------|
| `target`  | string | `"sink"` or `"source"`            |
| `id`      | uint   | Device numeric ID                 |
| `mute`    | bool   | `true` to mute, `false` to unmute |

```bash
deskbrid audio.mute '{"target": "sink", "id": 0, "mute": true}'
```

### audio.set_default

Set the default sink or source by name.

| Parameter | Type   | Description                                 |
|-----------|--------|---------------------------------------------|
| `target`  | string | `"sink"` or `"source"`                      |
| `name`    | string | Device name (e.g. `alsa_output.pci-...`)     |

```bash
deskbrid audio.set_default '{"target": "sink", "name": "bluez_sink.XX_XX.a2dp_sink"}'
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# List and adjust volume
sinks = client.audio_list_sinks()
for s in sinks:
    if s["default"]:
        client.audio_set_sink_volume(sink_id=s["id"], volume=0.6)

# Mute the default source
sources = client.audio_list_sources()
if sources:
    client.audio_mute(target="source", id=sources[0]["id"], mute=True)
```

## Requirements

- PulseAudio daemon (`pulseaudio`) or PipeWire with `pipewire-pulse` module.
- Uses `pactl` under the hood for PulseAudio, or `pw-cli` for PipeWire.

## Current Status

**Stable** — list sinks/sources, set volume, mute, set default.
