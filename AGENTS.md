# Deskbrid

Deskbrid bridges AI agents to any Linux desktop over a Unix socket. Control windows, inject keystrokes, take screenshots, manage clipboards — on GNOME, KDE, Hyprland, or X11.

## Install

```bash
bash <(curl -fsSL https://deskbrid.patchhive.dev/install.sh)
```

Auto-detects distro + DE, installs deps, sets up uinput, downloads binary.

## Quick Start

```bash
deskbrid daemon                               # start daemon
echo '{"type":"system.info","id":"1"}' | nc -U $XDG_RUNTIME_DIR/deskbrid.sock -w 2
deskbrid health                               # check deps
```

## Docs

https://deskbrid.patchhive.dev
