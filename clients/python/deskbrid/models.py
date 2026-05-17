from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any


@dataclass(slots=True)
class WindowInfo:
    id: str = ""
    title: str = ""
    app_id: str = ""
    pid: int = 0
    workspace_id: int = 0
    is_focused: bool = False
    is_minimized: bool = False
    geometry: tuple[int, int, int, int] = (0, 0, 0, 0)


@dataclass(slots=True)
class ClipboardContent:
    text: str = ""
    mime_types: list[str] = field(default_factory=list)
    timestamp: int | None = None


@dataclass(slots=True)
class MonitorInfo:
    id: int = 0
    name: str = ""
    width: int = 0
    height: int = 0
    scale: float = 1.0
    primary: bool = False
    enabled: bool = True
    x: int = 0
    y: int = 0
    refresh_rate: float | None = None
    rotation: str = "normal"


@dataclass(slots=True)
class ScreenshotResult:
    path: str = ""
    width: int | None = None
    height: int | None = None


@dataclass(slots=True)
class DaemonInfo:
    desktop: str = ""
    desktop_version: str = ""
    compositor: str = ""
    session_type: str = ""
    monitors: list[MonitorInfo] = field(default_factory=list)
    workspace_count: int = 0
    current_workspace: int = 0
    idle_seconds: int = 0


# ─── Decoders ──────────────────────────────────────

def decode_windows(payload: dict[str, Any]) -> list[WindowInfo]:
    # windows.list returns data as a flat array: [{"id": "3", "title": "...", ...}]
    if isinstance(payload, list):
        items = payload
    elif isinstance(payload, dict):
        items = payload.get("data", payload.get("windows", []))
        if isinstance(items, dict):
            items = [items]
        elif not isinstance(items, list):
            items = []
    else:
        items = []

    return [
        WindowInfo(
            id=str(item.get("id", "")),
            title=str(item.get("title", "")),
            app_id=str(item.get("app_id", "")),
            pid=int(item.get("pid", 0)),
            workspace_id=int(item.get("workspace_id", item.get("workspace", 0))),
            is_focused=bool(item.get("is_focused", item.get("focused", False))),
            is_minimized=bool(item.get("is_minimized", item.get("minimized", False))),
            geometry=_geometry(item.get("geometry")),
        )
        for item in items
    ]


def decode_monitors(payload: dict[str, Any]) -> list[MonitorInfo]:
    # monitor.list returns data as a flat array
    if isinstance(payload, list):
        items = payload
    elif isinstance(payload, dict):
        items = payload.get("data", payload.get("monitors", []))
        if not isinstance(items, list):
            items = []
    else:
        items = []

    return [
        MonitorInfo(
            id=int(item.get("id", 0)),
            name=str(item.get("name", "")),
            width=int(item.get("width", 0)),
            height=int(item.get("height", 0)),
            scale=float(item.get("scale", 1.0)),
            primary=bool(item.get("primary", False)),
            enabled=bool(item.get("enabled", True)),
            x=int(item.get("x", 0)),
            y=int(item.get("y", 0)),
            refresh_rate=_optional_float(item.get("refresh_rate")),
            rotation=str(item.get("rotation", "normal")),
        )
        for item in items
    ]


def decode_clipboard(payload: dict[str, Any]) -> ClipboardContent:
    return ClipboardContent(
        text=str(payload.get("text", "")),
        mime_types=[str(item) for item in payload.get("mime_types", [])],
        timestamp=_optional_int(payload.get("timestamp")),
    )


def decode_info(payload: dict[str, Any]) -> DaemonInfo:
    monitors_raw = payload.get("monitors", [])
    monitors = decode_monitors(monitors_raw) if not isinstance(monitors_raw, list) else [
        MonitorInfo(
            id=int(m.get("id", 0)),
            name=str(m.get("name", "")),
            width=int(m.get("width", 0)),
            height=int(m.get("height", 0)),
            scale=float(m.get("scale", 1.0)),
            primary=bool(m.get("primary", False)),
            enabled=bool(m.get("enabled", True)),
            x=int(m.get("x", 0)),
            y=int(m.get("y", 0)),
            refresh_rate=_optional_float(m.get("refresh_rate")),
            rotation=str(m.get("rotation", "normal")),
        )
        for m in (monitors_raw if isinstance(monitors_raw, list) else [])
    ]

    return DaemonInfo(
        desktop=str(payload.get("desktop", "")),
        desktop_version=str(payload.get("desktop_version", "")),
        compositor=str(payload.get("compositor", "")),
        session_type=str(payload.get("session_type", "")),
        monitors=monitors,
        workspace_count=int(payload.get("workspace_count", 0)),
        current_workspace=int(payload.get("current_workspace", 0)),
        idle_seconds=int(payload.get("idle_seconds", 0)),
    )


def decode_screenshot(payload: dict[str, Any]) -> ScreenshotResult:
    return ScreenshotResult(
        path=str(payload.get("path", "")),
        width=_optional_int(payload.get("width")),
        height=_optional_int(payload.get("height")),
    )


# ─── Helpers ───────────────────────────────────────

def _geometry(value: Any) -> tuple[int, int, int, int]:
    if isinstance(value, (list, tuple)) and len(value) == 4:
        return tuple(int(item) for item in value)  # type: ignore[return-value]
    return (0, 0, 0, 0)


def _optional_int(value: Any) -> int | None:
    if value is None:
        return None
    return int(value)


def _optional_float(value: Any) -> float | None:
    if value is None:
        return None
    return float(value)
