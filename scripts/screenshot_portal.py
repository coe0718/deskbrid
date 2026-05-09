#!/usr/bin/env python3
"""Take a screenshot via XDG Desktop Portal.

The portal saves screenshots to ~/Pictures/Screenshot-N.png automatically.
This script calls the portal, waits for the new file, and copies it to
our temp directory. Works on any Wayland compositor.
"""

import glob
import os
import re
import shutil
import subprocess
import sys
import time
import tempfile

home = os.environ.get("HOME", "/home/coemedia")
pics_dir = os.path.join(home, "Pictures")
out_dir = "/tmp/deskbrid"
os.makedirs(out_dir, exist_ok=True)

def get_max_screenshot_num():
    """Get the highest Screenshot-N number in ~/Pictures/."""
    max_n = 0
    for f in glob.glob(os.path.join(pics_dir, "Screenshot-*.png")):
        m = re.search(r"Screenshot-(\d+)", os.path.basename(f))
        if m:
            n = int(m.group(1))
            if n > max_n:
                max_n = n
    return max_n

def call_portal():
    """Call the portal Screenshot API via dbus-send (no signal handling needed)."""
    cmd = [
        "gdbus", "call", "--session",
        "--dest", "org.freedesktop.portal.Desktop",
        "--object-path", "/org/freedesktop/portal/desktop",
        "--method", "org.freedesktop.portal.Screenshot.Screenshot",
        "",
        "{'interactive': <boolean false>}"
    ]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=10)
    return result.stdout.strip()

before = get_max_screenshot_num()
print(f"Before: Screenshot number {before}", file=sys.stderr)

# Call the portal
handle = call_portal()
print(f"Portal handle: {handle}", file=sys.stderr)

# Wait for new file to appear (portal saves async)
for _ in range(30):
    time.sleep(0.5)
    after = get_max_screenshot_num()
    if after > before:
        src = os.path.join(pics_dir, f"Screenshot-{after}.png")
        if os.path.exists(src):
            ts = int(time.time())
            dst = os.path.join(out_dir, f"screenshot_{ts}.png")
            shutil.copy2(src, dst)
            print(dst)
            sys.exit(0)

print("ERROR: no screenshot appeared after 15s", file=sys.stderr)
sys.exit(1)
