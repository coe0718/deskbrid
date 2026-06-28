# MPRIS Media Control Feature

The MPRIS (Media Player Remote Interfacing Specification) feature enables agents to discover and control media players that expose the standard D-Bus MPRIS interface — including Spotify, VLC, Rhythmbox, Firefox, Chromium, and most other Linux media applications. Use it to list active players, get the currently playing track with metadata, and control playback (play, pause, skip, stop).

## Actions

### mpris.list

List all active MPRIS media players currently registered on the session D-Bus bus. Returns each player's bus name, display identity, playback status, track metadata, volume, and supported capabilities.

No parameters required.

```bash
deskbrid mpris list
```

```json
{"type": "mpris.list"}
```

Response:
```json
{
  "players": [
    {
      "bus_name": "org.mpris.MediaPlayer2.spotify",
      "player_id": "spotify",
      "identity": "Spotify",
      "playback_status": "Playing",
      "metadata": {
        "xesam:title": "Bohemian Rhapsody",
        "xesam:artist": ["Queen"],
        "xesam:album": "A Night at the Opera",
        "mpris:length": 354000000
      },
      "volume": 0.75,
      "can_play": true,
      "can_pause": true,
      "can_go_next": true,
      "can_go_previous": true
    }
  ],
  "count": 1
}
```

### mpris.get

Get detailed information about a specific MPRIS player, or the first available player if none is specified. Returns the same structure as the per-player entries in `mpris.list`, including full track metadata.

| Parameter | Type | Description |
|-----------|------|-------------|
| `player` | string (optional) | Player identifier — bus name (`"org.mpris.MediaPlayer2.spotify"`), short name (`"spotify"`), or partial match. Omit to target the first available player. |

```bash
deskbrid mpris get spotify
```

```json
{"type": "mpris.get", "player": "spotify"}
{"type": "mpris.get"}
```

Response (same structure as individual player entry in `mpris.list`):
```json
{
  "bus_name": "org.mpris.MediaPlayer2.spotify",
  "player_id": "spotify",
  "identity": "Spotify",
  "playback_status": "Playing",
  "metadata": {
    "xesam:title": "Bohemian Rhapsody",
    "xesam:artist": ["Queen"],
    "xesam:album": "A Night at the Opera",
    "mpris:length": 354000000
  },
  "volume": 0.75,
  "can_play": true,
  "can_pause": true,
  "can_go_next": true,
  "can_go_previous": true
}
```

### mpris.control

Send a playback command to an MPRIS player. If no player is specified, the command is sent to the first available player.

| Parameter | Type | Description |
|-----------|------|-------------|
| `player` | string (optional) | Player identifier (bus name, short name, or partial match). Omit for first available. |
| `action` | string (required) | One of: `"play"`, `"pause"`, `"play_pause"` (or `"toggle"`), `"stop"`, `"next"`, `"previous"` (or `"prev"`). |

```bash
deskbrid mpris control spotify play_pause
deskbrid mpris control --player spotify --action next
```

```json
{"type": "mpris.control", "player": "spotify", "action": "play_pause"}
{"type": "mpris.control", "action": "next"}
```

Response:
```json
{"player": "org.mpris.MediaPlayer2.spotify", "action": "play_pause"}
```

The action is dispatched over D-Bus via the `org.mpris.MediaPlayer2.Player` interface. The response acknowledges which player received the command and which action was called.

## Safety Boundary

- MPRIS actions are inherently non-destructive — they only control media playback state. No files are created, modified, or deleted.
- The `player` parameter supports fuzzy matching (partial name, case-insensitive). If multiple players match, the first match wins. Use the explicit bus name to avoid ambiguity.
- If no MPRIS players are available on the session bus, all actions will return an error (`"no MPRIS players found"`).
- The `"stop"` action is supported but not all players expose it — check `playback_status` or capabilities after issuing the command if you need to verify.

## Local Development

```bash
# List available players
deskbrid mpris list

# Get details on a specific player
deskbrid mpris get spotify

# Control playback — start a media player first
# (spotify, vlc, firefox with media, etc.)
deskbrid mpris control spotify play_pause
deskbrid mpris control spotify next
deskbrid mpris control spotify previous
```

To test without real media players, start any D-Bus MPRIS-compatible application (e.g. `vlc --intf dbus` or `playerctl` mock scripts). The daemon logs D-Bus method calls at debug level — run `deskbrid daemon --debug` to see the raw D-Bus traffic.

## Configuration

No MPRIS-specific configuration options. The feature uses the session D-Bus bus automatically via `zbus`. Players must be running and registered on the session bus under `org.mpris.MediaPlayer2.*` namespaces to be discoverable.
