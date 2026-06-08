"""Tests for the Deskbrid Python client — models, decoders, errors.

Run: pytest tests/ -v
"""

import pytest
from deskbrid import (
    ClipboardContent,
    DaemonInfo,
    DeskbridError,
    MonitorInfo,
    WindowInfo,
)
from deskbrid.models import (
    decode_clipboard,
    decode_info,
    decode_monitors,
    decode_screenshot,
    decode_windows,
)


# ── Error ────────────────────────────────────────

def test_deskbrid_error_stores_code_and_message():
    err = DeskbridError("NOT_FOUND", "window 0x42 not found")
    assert err.code == "NOT_FOUND"
    assert err.message == "window 0x42 not found"
    assert "NOT_FOUND: window 0x42 not found" in str(err)


def test_deskbrid_error_is_runtime_error():
    err = DeskbridError("ERR", "boom")
    assert isinstance(err, RuntimeError)


# ── Models ────────────────────────────────────────

def test_window_info_defaults():
    w = WindowInfo()
    assert w.id == ""
    assert w.title == ""
    assert w.app_id == ""
    assert w.pid == 0
    assert w.geometry == (0, 0, 0, 0)
    assert w.is_focused is False
    assert w.is_minimized is False


def test_window_info_populated():
    w = WindowInfo(id="42", title="Test", app_id="firefox", pid=1234, is_focused=True)
    assert w.id == "42"
    assert w.title == "Test"
    assert w.app_id == "firefox"
    assert w.pid == 1234
    assert w.is_focused is True


def test_monitor_info_defaults():
    m = MonitorInfo()
    assert m.id == 0
    assert m.name == ""
    assert m.width == 0
    assert m.height == 0
    assert m.scale == 1.0
    assert m.primary is False
    assert m.rotation == "normal"


def test_clipboard_content_defaults():
    c = ClipboardContent()
    assert c.text == ""
    assert c.mime_types == []
    assert c.timestamp is None


def test_daemon_info_defaults():
    d = DaemonInfo()
    assert d.desktop == ""
    assert d.monitors == []
    assert d.workspace_count == 0


# ── Decoders: decode_windows ──────────────────────

def test_decode_windows_flat_array():
    result = decode_windows([{"id": "1", "title": "Terminal"}])
    assert len(result) == 1
    assert result[0].id == "1"
    assert result[0].title == "Terminal"


def test_decode_windows_empty_list():
    assert decode_windows([]) == []


def test_decode_windows_data_key():
    result = decode_windows({"data": [{"id": "99"}]})
    assert len(result) == 1
    assert result[0].id == "99"


def test_decode_windows_windows_key():
    result = decode_windows({"windows": [{"id": "88"}]})
    assert len(result) == 1
    assert result[0].id == "88"


def test_decode_windows_none():
    assert decode_windows(None) == []  # type: ignore[arg-type]


def test_decode_windows_int():
    assert decode_windows(42) == []  # type: ignore[arg-type]


def test_decode_windows_missing_fields():
    result = decode_windows([{}])
    w = result[0]
    assert w.id == ""
    assert w.title == ""
    assert w.pid == 0


def test_decode_windows_geometry():
    result = decode_windows([{"id": "1", "geometry": [10, 20, 800, 600]}])
    assert result[0].geometry == (10, 20, 800, 600)


# ── Decoders: decode_monitors ─────────────────────

def test_decode_monitors_flat_array():
    result = decode_monitors([{"id": 0, "name": "eDP-1", "width": 1920, "height": 1080}])
    assert len(result) == 1
    m = result[0]
    assert m.id == 0
    assert m.name == "eDP-1"
    assert m.width == 1920
    assert m.height == 1080


def test_decode_monitors_data_key():
    result = decode_monitors({"data": [{"id": 1, "primary": True}]})
    assert len(result) == 1
    assert result[0].id == 1
    assert result[0].primary is True


def test_decode_monitors_empty():
    assert decode_monitors({}) == []


# ── Decoder: decode_clipboard ─────────────────────

def test_decode_clipboard():
    c = decode_clipboard({"text": "hello", "mime_types": ["text/plain"], "timestamp": 1712345678})
    assert c.text == "hello"
    assert c.mime_types == ["text/plain"]
    assert c.timestamp == 1712345678


def test_decode_clipboard_empty():
    c = decode_clipboard({})
    assert c.text == ""
    assert c.mime_types == []
    assert c.timestamp is None


# ── Decoder: decode_info ──────────────────────────

def test_decode_info():
    d = decode_info({
        "desktop": "GNOME",
        "desktop_version": "46",
        "compositor": "mutter",
        "session_type": "wayland",
        "workspace_count": 4,
        "current_workspace": 2,
        "idle_seconds": 120,
        "monitors": [{"id": 0, "name": "eDP-1", "width": 1920, "height": 1080}],
    })
    assert d.desktop == "GNOME"
    assert d.desktop_version == "46"
    assert d.session_type == "wayland"
    assert d.workspace_count == 4
    assert d.current_workspace == 2
    assert d.idle_seconds == 120
    assert len(d.monitors) == 1


def test_decode_info_empty():
    d = decode_info({})
    assert d.desktop == ""
    assert d.monitors == []


# ── Decoder: decode_screenshot ────────────────────

def test_decode_screenshot():
    s = decode_screenshot({"path": "/tmp/shot.png", "width": 1920, "height": 1080})
    assert s.path == "/tmp/shot.png"
    assert s.width == 1920
    assert s.height == 1080


def test_decode_screenshot_empty():
    s = decode_screenshot({})
    assert s.path == ""
    assert s.width is None
    assert s.height is None
