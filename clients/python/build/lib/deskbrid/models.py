from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any


@dataclass(slots=True)
class WindowInfo:
    title: str = ""
    app_id: str = ""
    pid: int = 0
    workspace: int = 0
    focused: bool = False
    geometry: tuple[int, int, int, int] = (0, 0, 0, 0)
    wm_class: str = ""


@dataclass(slots=True)
class WindowClosedEvent:
    app_id: str = ""
    pid: int = 0


@dataclass(slots=True)
class ClipboardContent:
    text: str = ""
    mime_types: list[str] = field(default_factory=list)
    timestamp: int | None = None


@dataclass(slots=True)
class NotificationEvent:
    app: str = ""
    app_icon: str = ""
    summary: str = ""
    body: str = ""
    urgency: str = "normal"
    id: int = 0


@dataclass(slots=True)
class IdleEvent:
    idle: bool = False
    idle_since: int | None = None
    idle_seconds: int = 0


@dataclass(slots=True)
class AudioNodeEvent:
    id: int = 0
    name: str = ""
    state: str = ""
    volume: float = 0.0
    muted: bool = False


@dataclass(slots=True)
class MonitorInfo:
    id: int = 0
    width: int = 0
    height: int = 0
    scale: float = 1.0
    refresh: int = 0


@dataclass(slots=True)
class ScreenshotResult:
    path: str = ""
    width: int | None = None
    height: int | None = None


@dataclass(slots=True)
class DaemonInfo:
    deskbrid_version: str
    desktop: str
    session_type: str
    capabilities: list[str]


def decode_event(event: str, payload: dict[str, Any]) -> Any:
    if event in {"window:focus", "window:open"}:
        return WindowInfo(
            title=str(payload.get("title", "")),
            app_id=str(payload.get("app_id", "")),
            pid=int(payload.get("pid", 0)),
            workspace=int(payload.get("workspace", 0)),
            focused=bool(payload.get("focused", False)),
            geometry=_geometry(payload.get("geometry")),
            wm_class=str(payload.get("wm_class", "")),
        )
    if event == "window:close":
        return WindowClosedEvent(
            app_id=str(payload.get("app_id", "")),
            pid=int(payload.get("pid", 0)),
        )
    if event == "clipboard":
        return ClipboardContent(
            text=str(payload.get("text", "")),
            mime_types=[str(item) for item in payload.get("mime_types", [])],
            timestamp=_optional_int(payload.get("timestamp")),
        )
    if event == "notifications":
        return NotificationEvent(
            app=str(payload.get("app", "")),
            app_icon=str(payload.get("app_icon", "")),
            summary=str(payload.get("summary", "")),
            body=str(payload.get("body", "")),
            urgency=str(payload.get("urgency", "normal")),
            id=int(payload.get("id", 0)),
        )
    if event == "idle":
        return IdleEvent(
            idle=bool(payload.get("idle", False)),
            idle_since=_optional_int(payload.get("idle_since")),
            idle_seconds=int(payload.get("idle_seconds", 0)),
        )
    if event == "audio:node":
        return AudioNodeEvent(
            id=int(payload.get("id", 0)),
            name=str(payload.get("name", "")),
            state=str(payload.get("state", "")),
            volume=float(payload.get("volume", 0.0)),
            muted=bool(payload.get("muted", False)),
        )
    return payload


def decode_windows(payload: dict[str, Any]) -> list[WindowInfo]:
    return [
        WindowInfo(
            title=str(item.get("title", "")),
            app_id=str(item.get("app_id", "")),
            pid=int(item.get("pid", 0)),
            workspace=int(item.get("workspace", 0)),
            focused=bool(item.get("focused", False)),
            geometry=_geometry(item.get("geometry")),
            wm_class=str(item.get("wm_class", "")),
        )
        for item in payload.get("windows", [])
    ]


def decode_monitors(payload: dict[str, Any]) -> list[MonitorInfo]:
    return [
        MonitorInfo(
            id=int(item.get("id", 0)),
            width=int(item.get("width", 0)),
            height=int(item.get("height", 0)),
            scale=float(item.get("scale", 1.0)),
            refresh=int(item.get("refresh", 0)),
        )
        for item in payload.get("monitors", [])
    ]


def decode_clipboard(payload: dict[str, Any]) -> ClipboardContent:
    return ClipboardContent(
        text=str(payload.get("text", "")),
        mime_types=[str(item) for item in payload.get("mime_types", [])],
        timestamp=_optional_int(payload.get("timestamp")),
    )


def decode_info(payload: dict[str, Any]) -> DaemonInfo:
    return DaemonInfo(
        deskbrid_version=str(payload.get("deskbrid_version", "")),
        desktop=str(payload.get("desktop", "")),
        session_type=str(payload.get("session_type", "")),
        capabilities=[str(item) for item in payload.get("capabilities", [])],
    )


def decode_screenshot(payload: dict[str, Any]) -> ScreenshotResult:
    return ScreenshotResult(
        path=str(payload.get("path", "")),
        width=_optional_int(payload.get("width")),
        height=_optional_int(payload.get("height")),
    )


def _geometry(value: Any) -> tuple[int, int, int, int]:
    if isinstance(value, (list, tuple)) and len(value) == 4:
        return tuple(int(item) for item in value)  # type: ignore[return-value]
    return (0, 0, 0, 0)


def _optional_int(value: Any) -> int | None:
    if value is None:
        return None
    return int(value)
