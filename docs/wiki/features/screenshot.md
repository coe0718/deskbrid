# Screenshot Feature

The screenshot feature captures the Linux desktop — full screen, per-monitor, window-specific, or a cropped region — and optionally runs OCR or pixel-diff comparisons. It is the primary way for agents to observe the visual state of the desktop. Use it when you need to read on-screen text, detect UI changes, or capture what the user sees.

## Actions

### screenshot

Capture a screenshot of the full desktop, a specific monitor, a window, or a rectangular region. Returns the file path and dimensions of the captured image.

| Parameter | Type | Description |
|-----------|------|-------------|
| `monitor` | uint (optional) | Zero-indexed monitor ID to capture. Omit for all monitors composited together. |
| `region` | object (optional) | Crop to `{ x, y, width, height }` (pixels). |
| `window_id` | string (optional) | Hex window ID (e.g. from `windows.list`) to capture a specific window. |
| `output` | string (optional) | Save path for the screenshot. If omitted, a temp file is used and the path is returned. |

```bash
deskbrid screenshot
deskbrid screenshot --monitor 0
deskbrid screenshot --region 0 0 800 600
```

```json
{"type": "screenshot", "monitor": 0}
{"type": "screenshot", "region": {"x": 0, "y": 0, "width": 800, "height": 600}}
{"type": "screenshot", "window_id": "0x05a0001a"}
```

Response:
```json
{
  "path": "/tmp/deskbrid/screenshot_1719500000.png",
  "width": 1920,
  "height": 1080,
  "format": "png"
}
```

### screenshot.ocr

Capture an area of the screen (or use an existing image file) and run Tesseract OCR on it. Returns the recognized text, per-word bounding boxes (optional), and confidence score.

| Parameter | Type | Description |
|-----------|------|-------------|
| `path` | string (optional) | Path to an existing image file. If omitted, takes a screenshot first. |
| `language` | string (optional) | Tesseract language (e.g. `"eng"`, `"eng+spa"`). Default: `"eng"`. |
| `psm` | uint (optional) | Tesseract PSM mode (0–13). Default: `3` (fully automatic). |
| `bounding_boxes` | bool (optional) | If true, include per-word coordinates and confidence in the response. |
| `monitor` | uint (optional) | Monitor index to capture (when not using `path`). |
| `region` | object (optional) | Crop region `{ x, y, width, height }` (when not using `path`). |
| `window_id` | string (optional) | Hex window ID (when not using `path`). |

```bash
deskbrid ocr --monitor 0
deskbrid ocr --path /tmp/screenshot.png --language eng
deskbrid ocr --region 100 200 400 100 --boxes
```

```json
{"type": "screenshot.ocr", "monitor": 0}
{"type": "screenshot.ocr", "path": "/tmp/screenshot.png", "language": "eng", "bounding_boxes": true}
```

Response:
```json
{
  "text": "Hello world",
  "confidence": 92.8,
  "words": [
    {"text": "Hello", "x": 10, "y": 20, "width": 30, "height": 15, "confidence": 95.5},
    {"text": "world", "x": 44, "y": 20, "width": 35, "height": 15, "confidence": 90.0}
  ],
  "source_path": "/tmp/deskbrid/screenshot_1719500000.png",
  "language": "eng",
  "psm": 3
}
```

**Requires:** `tesseract-ocr` and the appropriate language packs installed on the system.

### screenshot.diff

Compare two screenshots pixel-by-pixel and report differences. If `after_path` is omitted, a new screenshot is taken as the "after" image. A visual diff image can be saved with the changed pixels highlighted in red.

| Parameter | Type | Description |
|-----------|------|-------------|
| `before_path` | string (required) | Path to the "before" screenshot image. |
| `after_path` | string (optional) | Path to the "after" image. If omitted, a screenshot is taken. |
| `tolerance` | uint (optional) | Per-channel tolerance (0–255). Pixels differing by ≤ this value are considered equal. Default: `0`. |
| `diff_path` | string (optional) | Path to save the visual diff image (PNG). |
| `save_diff` | bool (optional) | If true and no `diff_path` given, saves diff to a temp file. |
| `monitor` | uint (optional) | Monitor index for the "after" screenshot. |
| `region` | object (optional) | Crop region for the "after" screenshot. |
| `window_id` | string (optional) | Hex window ID for the "after" screenshot. |

```bash
deskbrid screenshot diff --before /tmp/before.png --after /tmp/after.png
deskbrid screenshot diff --before /tmp/before.png --tolerance 5 --save-diff
```

```json
{"type": "screenshot.diff", "before_path": "/tmp/before.png", "after_path": "/tmp/after.png"}
{"type": "screenshot.diff", "before_path": "/tmp/before.png", "tolerance": 5, "save_diff": true}
```

Response:
```json
{
  "before_path": "/tmp/before.png",
  "after_path": "/tmp/after.png",
  "diff_path": "/tmp/deskbrid/diff_1719500000.png",
  "width": 1920,
  "height": 1080,
  "total_pixels": 2073600,
  "changed_pixels": 15324,
  "percent_changed": 0.739,
  "changed": true,
  "tolerance": 5,
  "bounding_box": {"x": 320, "y": 180, "width": 400, "height": 250}
}
```

The `bounding_box` field (present only when pixels differ) represents the minimal rectangle enclosing all changed pixels.

## Safety Boundary

- Screenshots may capture sensitive information (passwords, private messages, financial data). Treat screenshot output with the same confidentiality as the user's screen.
- OCR requires `tesseract-ocr` on the system — this is an external dependency not bundled with Deskbrid.
- Large screenshots at high resolutions produce large images. Temp files are cleaned up automatically, but be mindful of disk usage with repeated captures.
- The screenshot diff action with `after_path` omitted will silently capture a new screenshot — consider the implications of triggering captures without explicit user awareness.

## Local Development

```bash
# Install tesseract for OCR testing
sudo apt install tesseract-ocr tesseract-ocr-eng  # Debian/Ubuntu
sudo dnf install tesseract tesseract-langpack-eng   # Fedora

# Take a test screenshot
deskbrid screenshot --output /tmp/test.png

# Run OCR on it
deskbrid ocr --path /tmp/test.png

# Compare two screenshots
deskbrid screenshot diff --before /tmp/a.png --after /tmp/b.png
```

For automated testing, use the mock backend which returns synthetic screenshot results without a display server.

## Configuration

No screenshot-specific configuration options. Temp file paths use the system's temp directory (`/tmp/deskbrid/`). Output format is always PNG.
