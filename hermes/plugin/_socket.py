"""Unix socket communication with deskbrid daemon + auto-start hook."""
from __future__ import annotations

import json
import logging
import os
import socket
import subprocess
import time

logger = logging.getLogger(__name__)

SOCKET_PATH = os.environ.get(
    "DESKBRID_SOCKET",
    f"/run/user/{os.getuid()}/deskbrid.sock",
)
DEFAULT_TIMEOUT = 10.0


def send_action(action: dict, timeout: float = DEFAULT_TIMEOUT) -> dict:
    """Send a JSON action to deskbrid daemon and return the response.

    Action format: {"type": "windows.list", "id": "1"}
    Response format: {"ok": true, "data": {...}}
    """
    try:
        sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        sock.settimeout(timeout)
        sock.connect(SOCKET_PATH)
        sock.sendall((json.dumps(action) + "\n").encode())
        buf = b""
        while True:
            chunk = sock.recv(4096)
            if not chunk:
                break
            buf += chunk
            if b"\n" in buf:
                break
        sock.close()
        return json.loads(buf.decode().strip())
    except FileNotFoundError:
        return {"ok": False, "error": "deskbrid daemon not running (socket not found)"}
    except socket.timeout:
        return {"ok": False, "error": f"deskbrid timeout after {timeout}s"}
    except Exception as e:
        return {"ok": False, "error": str(e)}


def _ensure_daemon_started(**kwargs):
    """Start deskbrid daemon if the socket doesn't exist yet."""
    if os.path.exists(SOCKET_PATH):
        return

    logger.info("deskbrid socket not found — starting daemon...")
    try:
        subprocess.Popen(
            ["deskbrid", "daemon"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
        )
        for _ in range(20):
            if os.path.exists(SOCKET_PATH):
                logger.info("deskbrid daemon started successfully")
                return
            time.sleep(0.1)
        logger.warning("deskbrid daemon started but socket not found after 2s")
    except FileNotFoundError:
        logger.warning(
            "deskbrid binary not found — install with: "
            "curl -fsSL https://deskbrid.patchhive.dev/install.sh | bash"
        )
