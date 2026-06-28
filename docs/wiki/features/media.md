# Media Control

Control media playback through MPRIS.

> **Note:** The canonical documentation for MPRIS media control is in
> [mpris_media_control.md](mpris_media_control.md). This page is a convenience
> alias.

## List Players

```bash
deskbrid media list
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {
      "name": "spotify",
      "identity": "Spotify",
      "status": "playing",
      "title": "Bohemian Rhapsody",
      "artist": "Queen",
      "album": "A Night at the Opera",
      "position": 120000,
      "length": 354000
    }
  ]
}
```

Protocol:
```json
{"type": "mpris.list"}
```

## Get Player Status

```bash
deskbrid media get
deskbrid media get spotify
```

Protocol:
```json
{"type": "mpris.get", "player": "spotify"}
```

## Control Playback

```bash
deskbrid media play
deskbrid media pause
deskbrid media playpause
deskbrid media stop
deskbrid media next
deskbrid media previous
```

Protocol:
```json
{"type": "mpris.control", "action": "play", "player": "spotify"}
```

### Seek and Set Position

```bash
deskbrid media control --action seek --offset 10000  # Forward 10s
deskbrid media control --action seek --offset -5000  # Backward 5s
deskbrid media control --action set_position --position 60000  # Jump to 1min
```

Protocol:
```json
{"type": "mpris.control", "action": "set_position", "player": "spotify", "position": 60000}
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# List players
players = client.mpris_list()
for player in players:
    print(f"{player['identity']}: {player['title']}")

# Control playback
client.mpris_control("play")
client.mpris_control("next")
```

## AI Agent Example

```json
→ {"type": "mpris.list"}
← {"type": "response", "status": "ok", "data": {"players": [{"name": "spotify", "status": "playing", "title": "Song 1", ...}]}}

→ {"type": "mpris.control", "action": "pause", "player": "spotify"}
← {"type": "response", "status": "ok"}
```