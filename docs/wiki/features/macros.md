# Macros

Record, replay, export, and import action macros.

> **Note:** The canonical documentation for macro recording and replay is in
> [macro_recording_replay.md](macro_recording_replay.md). This page is a
> convenience alias.

## Overview

- Recorded in `fast` mode or `timed` mode.
- Replay supports overriding `mode` per invocation.
- Use `macro.export` / `macro.import` to share macro definitions.

## Record

```bash
deskbrid macro.record.start { name: "my-macro", mode: "timed" }
deskbrid macro.record.stop
```

## Control playback

```bash
deskbrid macro.replay { name: "my-macro", mode: "fast" }
```

## Inspect

```bash
deskbrid macro.list
deskbrid macro.get { name: "my-macro" }
```

## Share

```bash
deskbrid macro.export { name: "my-macro" }
deskbrid macro.import {
  name: "imported-macro",
  data: { /* macro definition */ }
}
```

## Delete

```bash
deskbrid macro.delete { name: "my-macro" }
```

## Example

```bash
deskbrid macro.record.start { name: "greeting", mode: "timed" }
# perform actions in the focused desktop window
deskbrid macro.record.stop

deskbrid macro.replay { name: "greeting" }
deskbrid macro.replay { name: "greeting", mode: "fast" }
```

## Python example

```python
from deskbrid import Deskbrid
client = Deskbrid()

client.macro_record_start(name="test-macro", mode="timed")
# perform actions...
client.macro_record_stop()

macros = client.macro_list()
print(macros)

client.macro_replay(name="test-macro", mode="fast")
```
