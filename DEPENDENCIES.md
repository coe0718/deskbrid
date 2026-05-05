# Dependencies

Deskbrid relies on several system packages and components to function.
Some are optional depending on which features you use.

## Required

| Dependency | Package | Purpose |
|---|---|---|
| GNOME Shell Extension | `extensions/deskbrid@deskbrid/` (in-repo) | Window listing, focus, and state-change signals over DBus |

Install the extension: see [`extensions/deskbrid@deskbrid/`](extensions/deskbrid@deskbrid/).

## Optional

| Dependency | Package | Purpose | Feature |
|---|---|---|---|
| `wl-paste` | `wl-clipboard` | Clipboard change monitoring | Clipboard awareness |
| `gnome-screenshot` | `gnome-screenshot` | Screen capture fallback | Screenshots (non-PipeWire) |
| PipeWire dev headers | `libpipewire-0.3-dev` | High-perf screen recording | Screencast |

## Quick Install

```bash
sudo apt install -y wl-clipboard gnome-screenshot
```

For PipeWire screencast support, build with `--features pipewire`:

```bash
sudo apt install -y libpipewire-0.3-dev
cargo build --release --features pipewire
```
