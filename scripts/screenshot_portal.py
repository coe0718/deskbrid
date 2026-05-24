#!/usr/bin/env python3
"""GNOME Wayland screenshot via ScreenCast portal + PipeWire.

Uses a restore_token if available for non-interactive capture.
On first run without a token, prompts the user to grant permission
and saves the token for future use.
"""
import sys, os, json, subprocess
from gi.repository import GLib, Gio

DESKTOP_SERVICE = "org.freedesktop.portal.Desktop"
DESKTOP_PATH = "/org/freedesktop/portal/desktop"
SC_IFACE = "org.freedesktop.portal.ScreenCast"
REQ_IFACE = "org.freedesktop.portal.Request"
TOKEN_FILE = os.path.expanduser("~/.config/deskbrid/gnome_screencast_token")

bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)


def portal_call(iface, method, params, timeout_ms=15000):
    loop = GLib.MainLoop()
    result = {"data": None, "error": None}

    def on_response(conn, sender, path, iface, signal, sp):
        code, data = sp.unpack()
        if code != 0:
            result["error"] = f"Response code {code}"
        result["data"] = data
        loop.quit()

    sid = bus.signal_subscribe(
        DESKTOP_SERVICE, REQ_IFACE, "Response",
        None, None, Gio.DBusSignalFlags.NONE, on_response
    )
    reply = bus.call_sync(
        DESKTOP_SERVICE, DESKTOP_PATH, iface, method, params,
        GLib.VariantType("(o)"), Gio.DBusCallFlags.NONE, timeout_ms, None
    )
    handle = reply.unpack()[0]
    GLib.timeout_add_seconds(timeout_ms // 1000 + 5, loop.quit)
    loop.run()
    bus.signal_unsubscribe(sid)

    if result["error"]:
        raise Exception(result["error"])
    if result["data"] is None:
        raise Exception(f"No response for {method}")
    return result["data"]


def load_token():
    try:
        with open(TOKEN_FILE) as f:
            return json.load(f)
    except (FileNotFoundError, json.JSONDecodeError):
        return None


def save_token(data):
    os.makedirs(os.path.dirname(TOKEN_FILE), exist_ok=True)
    with open(TOKEN_FILE, "w") as f:
        json.dump(data, f, indent=2)
    print(f"Token saved to {TOKEN_FILE}", file=sys.stderr)


def capture_frame(node_id, output_path):
    """Capture a single frame from PipeWire node via gst-launch."""
    pipe_cmd = [
        "gst-launch-1.0", "-q",
        "pipewiresrc", f"path={node_id}",
        "!", "videoconvert",
        "!", "pngenc", "snapshot=true",
        "!", "filesink", f"location={output_path}",
    ]
    proc = subprocess.run(pipe_cmd, capture_output=True, text=True, timeout=10)
    if proc.returncode != 0:
        raise Exception(f"gst-launch failed: {proc.stderr}")
    return output_path


def screenshot(output_path):
    token = f"deskbrid_{os.getpid()}"
    saved = load_token()

    sc = SC_IFACE

    if saved and "restore_token" in saved:
        # Fast path: restore previous session — no dialog needed
        print("Using saved restore token...", file=sys.stderr)
        result = portal_call(sc, "CreateSession",
            GLib.Variant("(a{sv})", ({
                "session_handle_token": GLib.Variant("s", f"{token}_session"),
                "handle_token": GLib.Variant("s", f"{token}_create"),
                "restore_token": GLib.Variant("s", saved["restore_token"]),
                "persist_mode": GLib.Variant("u", 2),
                "application_id": GLib.Variant("s", "com.patchhive.deskbrid"),
            },)))
        session_handle = result["session_handle"]

        # Start without SelectSources (restored session already has sources)
        result = portal_call(sc, "Start",
            GLib.Variant("(osa{sv})", (session_handle, "", {
                "handle_token": GLib.Variant("s", f"{token}_start"),
            })), timeout_ms=10000)
    else:
        # First time: full flow with permission dialog
        print("No restore token — requesting permission...", file=sys.stderr)
        print("A 'Share your screen?' dialog may appear. Click ALLOW.", file=sys.stderr)

        result = portal_call(sc, "CreateSession",
            GLib.Variant("(a{sv})", ({
                "session_handle_token": GLib.Variant("s", f"{token}_session"),
                "handle_token": GLib.Variant("s", f"{token}_create"),
                "persist_mode": GLib.Variant("u", 2),
                "application_id": GLib.Variant("s", "com.patchhive.deskbrid"),
            },)))
        session_handle = result["session_handle"]

        portal_call(sc, "SelectSources",
            GLib.Variant("(oa{sv})", (session_handle, {
                "handle_token": GLib.Variant("s", f"{token}_sel"),
                "types": GLib.Variant("u", 1),  # MONITOR
                "multiple": GLib.Variant("b", False),
            })))

        result = portal_call(sc, "Start",
            GLib.Variant("(osa{sv})", (session_handle, "", {
                "handle_token": GLib.Variant("s", f"{token}_start"),
            })), timeout_ms=60000)

        # Save restore_token for next time
        restore_token = result.get("restore_token")
        if restore_token:
            save_token({"restore_token": restore_token})
        else:
            # Save all keys — the token might be under a different name
            all_keys = {k: str(v) for k, v in (result or {}).items()}
            print(f"WARNING: No restore_token. All keys: {all_keys}", file=sys.stderr)
            # Save everything so we can inspect
            save_token({"full_response": all_keys})

    # Get the PipeWire stream
    streams = result.get("streams")
    if not streams:
        raise Exception("No streams in Start response")

    node_id = streams[0][0] if streams else None
    if not node_id:
        raise Exception("No PipeWire node ID in streams")

    print(f"PipeWire node: {node_id}", file=sys.stderr)

    # Capture a single frame
    capture_frame(node_id, output_path)

    if os.path.exists(output_path):
        size = os.path.getsize(output_path)
        print(f"Screenshot: {output_path} ({size} bytes)")
        return 0
    else:
        print(f"ERROR: No output at {output_path}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(f"Usage: {sys.argv[0]} <output_path>", file=sys.stderr)
        sys.exit(1)
    try:
        sys.exit(screenshot(sys.argv[1]))
    except Exception as e:
        print(f"ERROR: {e}", file=sys.stderr)
        sys.exit(1)
