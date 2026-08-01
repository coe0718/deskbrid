# Speech / Text-to-Speech

Speak text aloud through the desktop's speech engine. This gives agents an
**audible output channel** — the missing counterpart to `input.type_text`
and `notification.send`. Combine with `screenshot.ocr` to read a window
aloud, or with `presence` events to announce progress only while the user
is at the machine.

## Engines

| Engine | Binary | Notes |
|--------|--------|-------|
| `spd-say` (speech-dispatcher) | `spd-say` | Canonical Linux TTS used by Orca/GNOME; routes through the speech-dispatcher daemon. **Preferred when installed.** |
| `espeak-ng` | `espeak-ng` | Lightweight fallback; works headless and on minimal installs. |

`engine` defaults to `auto`: `spd-say` if present, else `espeak-ng`. You can
force either with the `engine` parameter. Engine availability is reported by
`system.health` as `spd-say` / `espeak-ng` dependency checks.

## Actions

### speech.speak

Speak a string of text aloud.

| Parameter | Type | Description |
|-----------|------|-------------|
| `text` | string | Text to speak (required, non-empty) |
| `voice` | string? | Voice: espeak-ng voice name, or spd-say voice type (`male1`…`male3`, `female1`…`female3`, `child_male`, `child_female`) |
| `rate` | integer? | Speech rate in words per minute (engine default if omitted) |
| `pitch` | integer? | Voice pitch (engine default if omitted) |
| `engine` | string? | `auto` (default), `spd-say`, or `espeak-ng` |
| `wait` | boolean | If `true`, block until the utterance finishes and return its duration |

```bash
deskbrid speak "Build finished, all tests passed."
deskbrid speak "Danger zone" --voice female1 --rate 180 --pitch 60
deskbrid speak "Wait for me" --wait
```

```json
{"type": "speech.speak", "text": "Build finished.", "rate": 160}
```

Fire-and-forget response (`wait: false`, default):

```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "speech_id": "a1b2c3d4-...",
    "engine": "spd-say",
    "pid": 4711,
    "spoken": false,
    "note": "Use speech.stop to cancel"
  }
}
```

Blocking response (`wait: true`):

```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "speech_id": "a1b2c3d4-...",
    "engine": "spd-say",
    "pid": 4711,
    "spoken": true,
    "duration_ms": 2140
  }
}
```

### speech.stop

Cancel all utterances started by deskbrid. Kills tracked children and, when
the speech-dispatcher engine is in use, also sends `spd-say --cancel`.

```bash
deskbrid speak-stop
```

```json
{"type": "speech.stop"}
```

Response: `{"stopped": 2}` (number of utterances cancelled).

### speech.voices

List available voices. With espeak-ng this returns its real voice table;
with only spd-say it returns the standard speech-dispatcher voice types.

```bash
deskbrid speak-voices
```

```json
{"type": "speech.voices"}
```

## Agent patterns

**Read a window aloud** (screen-reader in three calls):

```json
{"type": "screenshot", "window_id": "…"}
{"type": "screenshot.ocr", "path": "…"}
{"type": "speech.speak", "text": "<ocr text>", "wait": true}
```

**Narrate only when the user is present** — subscribe to `presence.active`
/ `presence.idle` events and gate `speech.speak` on the active state,
falling back to `notification.send` when the user is away.
