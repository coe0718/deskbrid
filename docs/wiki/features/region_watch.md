# Region & Text Watch

Poll a screen region for visual changes or OCR text, with configurable
thresholds, auto-save, and event-based notifications.

Two independent watch types are available: **region watches** (pixel-level
change detection) and **text watches** (OCR-based text extraction with
history).

## Constants

| Setting                   | Default | Range        |
|---------------------------|---------|--------------|
| Poll interval             | 500 ms  | 100–10,000   |
| Change threshold          | 1.0%    | 0.0–100.0    |
| Stable duration           | 1,000 ms| ≥ 0          |
| Text history capacity     | 20      | 1–100        |

## Region Watch

Region watches periodically screenshot a screen region and detect pixel-level
changes. They emit events on each change and/or when the region stabilises.

### `region.watch.create`

Create a new region watch that polls a screen region for visual changes.

| Parameter            | Type   | Required | Description                                                    |
|----------------------|--------|----------|----------------------------------------------------------------|
| `name`               | string | yes      | Unique name for this watch                                     |
| `region`             | object | yes      | Screen region `{x, y, width, height}` (see [Region spec](#))  |
| `monitor`            | uint   | no       | Monitor index (default: primary)                               |
| `interval_ms`        | uint   | no       | Poll interval in ms (default: 500, clamped: 100–10,000)        |
| `change_threshold_pct` | float | no      | Min percentage of changed pixels to trigger (default: 1.0)     |
| `notify_on_change`   | bool   | no       | Emit event on each detected change (default: false)            |
| `notify_on_stable`   | bool   | no       | Emit event when region stabilises (default: false)             |
| `stable_duration_ms` | uint   | no       | Duration of no-change before considered stable (default: 1000) |
| `auto_save`          | string | no       | Save changelog screenshots to this directory path              |
| `max_changes`        | uint   | no       | Stop after this many changes (default: no limit)               |
| `tolerance`          | uint   | no       | Pixel comparison tolerance (0–255, default: 0)                 |

```json
{
  "type": "region.watch.create",
  "name": "editor-toolbar",
  "region": { "x": 100, "y": 50, "width": 800, "height": 40 },
  "monitor": 0,
  "interval_ms": 200,
  "change_threshold_pct": 5.0,
  "notify_on_change": true,
  "notify_on_stable": true,
  "stable_duration_ms": 2000
}
```

**Response:**

```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "created": "editor-toolbar",
    "watch": {
      "name": "editor-toolbar",
      "monitor": 0,
      "region": { "x": 100, "y": 50, "width": 800, "height": 40 },
      "interval_ms": 200,
      "change_threshold_pct": 5.0,
      "notify_on_change": true,
      "notify_on_stable": true,
      "stable_duration_ms": 2000,
      "auto_save": null,
      "max_changes": null,
      "tolerance": 0
    }
  }
}
```

### `region.watch.update`

Update the configuration of an existing region watch. Only the fields
provided are changed; omitted fields keep their current values.

| Parameter            | Type   | Required | Description                                                    |
|----------------------|--------|----------|----------------------------------------------------------------|
| `name`               | string | yes      | Name of the watch to update                                    |
| `region`             | object | no       | New screen region                                              |
| `monitor`            | uint   | no       | New monitor index                                              |
| `interval_ms`        | uint   | no       | New poll interval                                              |
| `change_threshold_pct` | float | no      | New change threshold                                           |
| `notify_on_change`   | bool   | no       | Enable/disable change events                                   |
| `notify_on_stable`   | bool   | no       | Enable/disable stable events                                   |
| `stable_duration_ms` | uint   | no       | New stable duration                                            |
| `auto_save`          | string | no       | Change auto-save directory                                     |
| `max_changes`        | uint   | no       | New max changes limit                                          |
| `tolerance`          | uint   | no       | New pixel tolerance                                            |

```json
{
  "type": "region.watch.update",
  "name": "editor-toolbar",
  "interval_ms": 500,
  "notify_on_change": false
}
```

**Response:**

```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "updated": "editor-toolbar",
    "watch": { "...": "updated config ..." }
  }
}
```

### `region.watch.remove`

Remove and stop a region watch.

| Parameter | Type   | Required | Description          |
|-----------|--------|----------|----------------------|
| `name`    | string | yes      | Name of the watch    |

```json
{"type": "region.watch.remove", "name": "editor-toolbar"}
```

**Response:**

```json
{
  "type": "response",
  "status": "ok",
  "data": { "removed": "editor-toolbar" }
}
```

### `region.watch.list`

List all active region watches with their config and runtime status.

```json
{"type": "region.watch.list"}
```

**Response:**

```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "watches": [
      {
        "config": {
          "name": "editor-toolbar",
          "monitor": 0,
          "region": { "x": 100, "y": 50, "width": 800, "height": 40 },
          "interval_ms": 200,
          "change_threshold_pct": 5.0,
          "notify_on_change": true,
          "notify_on_stable": true,
          "stable_duration_ms": 2000,
          "auto_save": null,
          "max_changes": null,
          "tolerance": 0
        },
        "status": {
          "changes_seen": 12,
          "last_changed": 1705312800123,
          "last_stable": 1705312802000
        }
      }
    ],
    "count": 1
  }
}
```

## Text Watch

Text watches periodically OCR a screen region and track text changes with
history. They can trigger on any change, a specific text match, or a
text mismatch.

### `text.watch.create`

Create a text watch that extracts and tracks OCR text from a screen region.

| Parameter          | Type   | Required | Description                                                      |
|--------------------|--------|----------|------------------------------------------------------------------|
| `name`             | string | yes      | Unique name for this watch                                       |
| `region`           | object | yes      | Screen region `{x, y, width, height}`                            |
| `monitor`          | uint   | no       | Monitor index (default: primary)                                 |
| `interval_ms`      | uint   | no       | Poll interval in ms (default: 500, clamped: 100–10,000)          |
| `language`         | string | no       | OCR language hint (e.g., `eng`, `chi_sim`)                       |
| `notify_on_change` | bool   | no       | Emit event when extracted text changes (default: false)          |
| `notify_on_match`  | string | no       | Emit event when text matches this regex pattern                  |
| `notify_on_mismatch` | string | no     | Emit event when text no longer matches this regex pattern        |
| `max_entries`      | uint   | no       | Max OCR history entries to retain (default: 20, max: 100)        |
| `psm`              | uint   | no       | Tesseract page segmentation mode override                        |

```json
{
  "type": "text.watch.create",
  "name": "status-bar",
  "region": { "x": 100, "y": 700, "width": 600, "height": 40 },
  "interval_ms": 1000,
  "notify_on_change": true,
  "notify_on_match": "ready|done|complete"
}
```

**Response:**

```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "created": "status-bar",
    "watch": {
      "name": "status-bar",
      "monitor": null,
      "region": { "x": 100, "y": 700, "width": 600, "height": 40 },
      "interval_ms": 1000,
      "language": null,
      "notify_on_change": true,
      "notify_on_match": "ready|done|complete",
      "notify_on_mismatch": null,
      "max_entries": 20,
      "psm": null
    }
  }
}
```

### `text.watch.remove`

Remove and stop a text watch.

| Parameter | Type   | Required | Description          |
|-----------|--------|----------|----------------------|
| `name`    | string | yes      | Name of the watch    |

```json
{"type": "text.watch.remove", "name": "status-bar"}
```

**Response:**

```json
{
  "type": "response",
  "status": "ok",
  "data": { "removed": "status-bar" }
}
```

### `text.watch.list`

List all active text watches with their config and runtime status (latest OCR
text and history).

```json
{"type": "text.watch.list"}
```

**Response:**

```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "watches": [
      {
        "config": {
          "name": "status-bar",
          "monitor": null,
          "region": { "x": 100, "y": 700, "width": 600, "height": 40 },
          "interval_ms": 1000,
          "language": null,
          "notify_on_change": true,
          "notify_on_match": "ready|done|complete",
          "notify_on_mismatch": null,
          "max_entries": 20,
          "psm": null
        },
        "status": {
          "last_text": "Ready",
          "history": [
            { "timestamp": 1705312800000, "text": "Waiting..." },
            { "timestamp": 1705312805000, "text": "Processing" },
            { "timestamp": 1705312810000, "text": "Ready" }
          ]
        }
      }
    ],
    "count": 1
  }
}
```

## Events

### `region.watch.changed`

Emitted when a region watch detects a visual change (if `notify_on_change` is
enabled).

```json
{
  "type": "region.watch.changed",
  "name": "editor-toolbar",
  "changes_seen": 13,
  "timestamp": 1705312800500
}
```

### `region.watch.stable`

Emitted when a region watch detects stability after a change, waiting for the
configured `stable_duration_ms` (if `notify_on_stable` is enabled).

```json
{
  "type": "region.watch.stable",
  "name": "editor-toolbar",
  "changes_seen": 13,
  "timestamp": 1705312802000
}
```

### `text.watch.changed`

Emitted when a text watch detects different OCR text (if `notify_on_change` is
enabled).

```json
{
  "type": "text.watch.changed",
  "name": "status-bar",
  "text": "Processing",
  "previous_text": "Waiting...",
  "timestamp": 1705312805000
}
```

### `text.watch.matched`

Emitted when OCR text matches the `notify_on_match` pattern.

```json
{
  "type": "text.watch.matched",
  "name": "status-bar",
  "text": "Ready",
  "pattern": "ready|done|complete",
  "timestamp": 1705312810000
}
```

### `text.watch.mismatched`

Emitted when OCR text no longer matches the `notify_on_mismatch` pattern.

```json
{
  "type": "text.watch.mismatched",
  "name": "status-bar",
  "text": "Error",
  "pattern": "ready|done|complete",
  "timestamp": 1705312815000
}
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# Watch a toolbar region for visual changes
result = client.region_watch_create(
    name="toolbar",
    region={"x": 0, "y": 0, "width": 1920, "height": 50},
    notify_on_change=True,
    notify_on_stable=True,
    stable_duration_ms=1500,
)
print(f"Created: {result['created']}")

# List active region watches
watches = client.region_watch_list()
print(f"Active watches: {watches['count']}")

# Watch a status bar for OCR text matching a pattern
result = client.text_watch_create(
    name="status",
    region={"x": 0, "y": 1040, "width": 400, "height": 40},
    notify_on_match=r"build (succeeded|failed)",
)
print(f"Created OCR watch: {result['created']}")

# Clean up when done
client.region_watch_remove(name="toolbar")
client.text_watch_remove(name="status")
```

## Safety

- Watches spawn independent background tasks; they continue running until
  explicitly removed or the daemon exits
- Poll intervals are clamped to 100–10,000 ms to prevent excessive CPU usage
- `max_changes` acts as a one-shot limiter — the watch self-stops after the
  limit is reached
- Auto-saved screenshots accumulate on disk; configure a cleanup policy for
  long-running watches
- Text watch OCR can be CPU-intensive at low intervals; prefer 1,000+ ms for
  production use
