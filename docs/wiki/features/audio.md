# Audio

Control audio sinks and volumes (Linux PulseAudio/PipeWire).

## List Audio Sinks

```bash
deskbrid audio sinks
```

Response:
```json
{
  "type": "response",
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

## List Audio Sources

```bash
deskbrid audio sources
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {
      "name": "alsa_input.pci-0000_00_1f.3.analog-stereo",
      "description": "Built-in Audio Analog Stereo",
      "volume": 100,
      "muted": false,
      "default": true
    }
  ]
}
```

## Set Volume

```bash
deskbrid audio volume 50          # Set to 50%
deskbrid audio volume +10         # Increase by 10%
deskbrid audio volume -5          # Decrease by 5%
deskbrid audio volume 100 --sink alsa_output.pci-0000_00_1f.3.analog-stereo
```

Protocol:
```json
{"type": "audio.set_volume", "volume": 50, "sink": "alsa_output.pci-0000_00_1f.3.analog-stereo"}
```

## Get Volume

```bash
deskbrid audio volume --sink alsa_output.pci-0000_00_1f.3.analog-stereo
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "volume": 75,
    "muted": false
  }
}
```

## Mute Toggle

```bash
deskbrid audio mute           # Toggle mute on default sink
deskbrid audio mute --sink alsa_output.pci-0000_00_1f.3.analog-stereo
```

Protocol:
```json
{"type": "audio.set_mute", "muted": true, "sink": "alsa_output.pci-0000_00_1f.3.analog-stereo"}
```

## Unmute

```bash
deskbrid audio unmute
deskbrid audio unmute --sink alsa_output.pci-0000_00_1f.3.analog-stereo
```

Protocol:
```json
{"type": "audio.set_mute", "muted": false, "sink": "alsa_output.pci-0000_00_1f.3.analog-stereo"}
```

## Set Default Sink

```bash
deskbrid audio set-default --sink alsa_output.usb-headphones.analog-stereo
```

Protocol:
```json
{"type": "audio.set_default_sink", "sink": "alsa_output.usb-headphones.analog-stereo"}
```

## Get Default Sink

```bash
deskbrid audio get-default
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "name": "alsa_output.pci-0000_00_1f.3.analog-stereo",
    "description": "Built-in Audio Analog Stereo"
  }
}
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# List sinks
sinks = client.audio_sinks()
for sink in sinks:
    print(f"{sink['description']}: {sink['volume']}%")

# List sources
sources = client.audio_sources()
for source in sources:
    print(f"{source['description']}: {source['volume']}%")

# Muting
client.audio_set_mute(True, sink="alsa_output.pci-0000_00_1f.3.analog-stereo")

# Setting volume
client.audio_set_volume(50, sink="alsa_output.pci-0000_00_1f.3.analog-stereo")

# Setting default sink
client.audio_set_default_sink("alsa_output.usb-headphones.analog-stereo")
```