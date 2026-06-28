# Screenshots & OCR

Capture screenshots, analyze with OCR, compare images, and pick colors.

> **Note:** The canonical documentation for screenshots is in
> [screenshot.md](screenshot.md) (includes OCR, diff, region watching).
> Color picking is documented in [color_picker.md](color_picker.md).
> This page is a convenience alias.

## Basic Screenshot

```bash
deskbrid screenshot
deskbrid screenshot --output screenshot.png
deskbrid screenshot --monitor 1
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "path": "/tmp/deskbrid_screenshot_20240115_103000.png"
  }
}
```

Protocol:
```json
{"type": "screenshot", "monitor": 0}
```

Python:
```python
path = client.screenshot()
print(f"Saved to {path.path}")
```

## Screenshot Region

```bash
deskbrid screenshot --region --x 100 --y 100 --width 800 --height 600
```

Protocol:
```json
{
  "type": "screenshot",
  "region": {"x": 100, "y": 100, "width": 800, "height": 600}
}
```

## Screenshot Specific Window

```bash
deskbrid screenshot --window 12345678
```

Protocol:
```json
{"type": "screenshot", "window_id": "12345678"}
```

## OCR (Optical Character Recognition)

Requires Tesseract OCR:

```bash
# Debian/Ubuntu
sudo apt install tesseract-ocr

# Arch
sudo pacman -S tesseract
```

### Basic OCR

```bash
deskbrid screenshot ocr
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "text": "Hello, world!\nThis is some text.",
    "lines": [
      {"text": "Hello, world!", "bbox": [10, 20, 150, 40]},
      {"text": "This is some text.", "bbox": [10, 50, 200, 70]}
    ],
    "confidence": 98.5
  }
}
```

Protocol:
```json
{"type": "screenshot.ocr", "bounding_boxes": false}
```

### OCR with Bounding Boxes

```bash
deskbrid screenshot ocr --bounding-boxes
```

Response:
```json
{
  "text": "Error: file not found",
  "lines": [
    {
      "text": "Error: file not found",
      "bbox": [50, 100, 250, 120]
    }
  ],
  "bounding_boxes": [[50, 100, 250, 120]]
}
```

### OCR Specific Region

```bash
deskbrid screenshot ocr --region --x 100 --y 100 --width 800 --height 600
```

Protocol:
```json
{
  "type": "screenshot.ocr",
  "region": {"x": 100, "y": 100, "width": 800, "height": 600},
  "language": "eng"
}
```

### Language Support

```bash
# Install language data
sudo apt install tesseract-ocr-fra  # French
sudo apt install tesseract-ocr-deu  # German
sudo apt install tesseract-ocr-chi-sim  # Chinese (Simplified)

# Use specific language
deskbrid screenshot ocr --language eng+fra
```

### Page Segmentation Modes (PSM)

```bash
deskbrid screenshot ocr --psm 6  # Assume single uniform block
deskbrid screenshot ocr --psm 7  # Treat as single text line
deskbrid screenshot ocr --psm 8  # Treat as single word
```

PSM values:
- `3` - Auto (default)
- `6` - Uniform block of text
- `7` - Single text line
- `8` - Single word
- `13` - Raw line

Python:
```python
# Basic OCR
result = client.screenshot_ocr()
print(result["text"])

# OCR with region
result = client.screenshot_ocr(
    region={"x": 100, "y": 100, "width": 800, "height": 600},
    language="eng"
)
```

## Screenshot Diff

Compare two screenshots to detect changes.

```bash
deskbrid screenshot diff --before /tmp/before.png --after /tmp/after.png
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "identical": false,
    "diff_pixels": 1250,
    "total_pixels": 2073600,
    "diff_percentage": 0.06
  }
}
```

Protocol:
```json
{
  "type": "screenshot.diff",
  "before_path": "/tmp/before.png",
  "after_path": "/tmp/after.png",
  "tolerance": 0,
  "save_diff": true,
  "diff_path": "/tmp/diff.png"
}
```

### Tolerance

Set tolerance for pixel differences:

```bash
deskbrid screenshot diff --before before.png --after after.png --tolerance 5
```

A tolerance of 5 means pixels differing by less than 5 RGB points are considered identical.

## Color Picker

Get color at specific coordinates:

```bash
deskbrid color pick --x 100 --y 200
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "rgb": [123, 45, 67],
    "hex": "#7b2d43"
  }
}
```

Protocol:
```json
{"type": "color.pick", "x": 100, "y": 200}
```

Python:
```python
# Get color
color = client.color_pick(x=100, y=200)
print(color["hex"])  # "#7b2d43"
```

## AI Agent Example

```json
→ {"type": "screenshot.ocr", "region": {"x": 0, "y": 0, "width": 1920, "height": 1080}}
← {"type": "response", "status": "ok", "data": {"text": "Build succeeded"}}

→ {"type": "screenshot.diff", "before_path": "/tmp/prev.png", "after_path": "/tmp/curr.png"}
← {"type": "response", "status": "ok", "data": {"identical": true}}
```