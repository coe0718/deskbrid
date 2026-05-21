from __future__ import annotations

from typing import Any

from .errors import DeskbridError
from .models import (
    ClipboardContent,
    DaemonInfo,
    MonitorInfo,
    ScreenshotResult,
    WindowInfo,
    decode_clipboard,
    decode_info,
    decode_monitors,
    decode_screenshot,
    decode_windows,
)


class AsyncActionsMixin:
    async def type_text(self, text: str) -> None:
        await self._request("input.keyboard", {"action": "type", "text": text})

    async def send_keys(self, keys: list[str]) -> None:
        await self._request("input.keyboard", {"action": "combo", "keys": keys})

    async def mouse_click(self, x: int, y: int, button: str = "left") -> None:
        await self._request("input.mouse", {"action": "click", "x": x, "y": y, "button": button})

    async def mouse_move(self, x: int, y: int) -> None:
        await self._request("input.mouse", {"action": "move", "x": x, "y": y})

    async def mouse_scroll(self, dx: float = 0.0, dy: float = 0.0) -> None:
        await self._request("input.mouse", {"action": "scroll", "dx": dx, "dy": dy})

    async def clipboard_read(self) -> ClipboardContent:
        return decode_clipboard(await self._request("clipboard.read"))

    async def clipboard_write(self, text: str) -> None:
        await self._request("clipboard.write", {"text": text})

    async def screenshot(self, monitor: int | None = None) -> ScreenshotResult:
        params: dict[str, Any] = {}
        if monitor is not None:
            params["monitor"] = monitor
        return decode_screenshot(await self._request("screenshot", params))

    async def screenshot_ocr(
        self,
        path: str | None = None,
        language: str | None = None,
        psm: int | None = None,
        bounding_boxes: bool = False,
        monitor: int | None = None,
        region: dict[str, int] | None = None,
        window_id: str | None = None,
    ) -> dict[str, Any]:
        params: dict[str, Any] = {"bounding_boxes": bounding_boxes}
        for key, value in {
            "path": path,
            "language": language,
            "psm": psm,
            "monitor": monitor,
            "region": region,
            "window_id": window_id,
        }.items():
            if value is not None:
                params[key] = value
        return await self._request("screenshot.ocr", params)

    async def notify(self, title: str, body: str = "", urgency: str = "normal") -> int:
        response = await self._request(
            "notification.send",
            {"app_name": "deskbrid", "title": title, "body": body, "urgency": urgency},
        )
        return int(response.get("notification_id", 0))

    async def list_windows(self) -> list[WindowInfo]:
        return decode_windows(await self._request("windows.list"))

    async def focus_window(
        self,
        *,
        app_id: str | None = None,
        title: str | None = None,
        exact: bool = False,
    ) -> None:
        # Find the window first, then focus by ID
        if not app_id and not title:
            raise DeskbridError("invalid_params", "app_id or title required")
        windows = await self.list_windows()
        target = next(
            (w for w in windows if (app_id and w.app_id == app_id) or (title and title.lower() in w.title.lower())),
            None,
        )
        if not target:
            raise DeskbridError("not_found", f"window not found: {app_id or title}")
        await self._request("windows.focus", {"window_id": target.id})

    async def activate_or_launch(
        self,
        app_id: str,
        command: list[str] | None = None,
        workdir: str | None = None,
        env: dict[str, str] | None = None,
    ) -> dict[str, Any]:
        params: dict[str, Any] = {"app_id": app_id}
        if command:
            params["command"] = command
        if workdir is not None:
            params["workdir"] = workdir
        if env:
            params["env"] = env
        return await self._request("windows.activate_or_launch", params)

    async def list_layout_profiles(self) -> list[dict[str, Any]]:
        response = await self._request("layout_profiles.list")
        if isinstance(response, list):
            return response
        data = response.get("data")
        if isinstance(data, list):
            return list(data)
        profiles = response.get("profiles")
        if isinstance(profiles, list):
            return list(profiles)
        return []

    async def save_layout_profile(self, name: str, overwrite: bool = False) -> dict[str, Any]:
        return await self._request(
            "layout_profiles.save",
            {"name": name, "overwrite": overwrite},
        )

    async def get_layout_profile(self, name: str) -> dict[str, Any]:
        return await self._request("layout_profiles.get", {"name": name})

    async def delete_layout_profile(self, name: str) -> dict[str, Any]:
        return await self._request("layout_profiles.delete", {"name": name})

    async def restore_layout_profile(self, name: str) -> dict[str, Any]:
        return await self._request("layout_profiles.restore", {"name": name})

    async def list_displays(self) -> list[MonitorInfo]:
        return decode_monitors(await self._request("monitor.list"))

    async def set_primary_monitor(self, output: str) -> dict[str, Any]:
        return await self._request("monitor.set_primary", {"output": output})

    async def set_monitor_resolution(
        self,
        output: str,
        width: int,
        height: int,
        refresh_rate: float | None = None,
    ) -> dict[str, Any]:
        params: dict[str, Any] = {"output": output, "width": width, "height": height}
        if refresh_rate is not None:
            params["refresh_rate"] = refresh_rate
        return await self._request("monitor.set_resolution", params)

    async def set_monitor_scale(self, output: str, scale: float) -> dict[str, Any]:
        return await self._request("monitor.set_scale", {"output": output, "scale": scale})

    async def set_monitor_rotation(self, output: str, rotation: str) -> dict[str, Any]:
        return await self._request(
            "monitor.set_rotation",
            {"output": output, "rotation": rotation},
        )

    async def enable_monitor(self, output: str) -> dict[str, Any]:
        return await self._request("monitor.enable", {"output": output})

    async def disable_monitor(self, output: str) -> dict[str, Any]:
        return await self._request("monitor.disable", {"output": output})

    async def wait_for(
        self,
        condition: str,
        params: dict[str, Any] | None = None,
        timeout_ms: int = 30_000,
        interval_ms: int | None = None,
    ) -> dict[str, Any]:
        request: dict[str, Any] = {
            "condition": condition,
            "params": params or {},
            "timeout_ms": timeout_ms,
        }
        if interval_ms is not None:
            request["interval_ms"] = interval_ms
        return await self._request("wait.for", request)

    async def info(self) -> DaemonInfo:
        return decode_info(await self._request("system.info"))

    async def inhibit_system(
        self,
        what: str,
        who: str = "deskbrid",
        why: str | None = None,
        mode: str | None = None,
    ) -> dict[str, Any]:
        params: dict[str, Any] = {"what": what, "who": who}
        if why is not None:
            params["why"] = why
        if mode is not None:
            params["mode"] = mode
        return await self._request("system.inhibit", params)

    async def release_inhibit(self, inhibitor_id: int) -> dict[str, Any]:
        return await self._request("system.release_inhibit", {"inhibitor_id": inhibitor_id})

    async def list_sessions(self) -> list[dict[str, Any]]:
        response = await self._request("system.sessions")
        return list(response.get("sessions", []))

    async def lock_session(self, session_id: str | None = None) -> dict[str, Any]:
        params: dict[str, Any] = {}
        if session_id is not None:
            params["session_id"] = session_id
        return await self._request("system.lock_session", params)

    async def switch_user(self, username: str) -> dict[str, Any]:
        return await self._request("system.switch_user", {"username": username})

    async def check_auth(self, action_id: str) -> dict[str, Any]:
        return await self._request("system.check_auth", {"action_id": action_id})

    async def elevate(self, action_id: str, reason: str | None = None) -> dict[str, Any]:
        params: dict[str, Any] = {"action_id": action_id}
        if reason is not None:
            params["reason"] = reason
        return await self._request("system.elevate", params)

    async def service_status(self, name: str) -> dict[str, Any]:
        return await self._request("service.status", {"name": name})

    async def service_start(self, name: str) -> dict[str, Any]:
        return await self._request("service.start", {"name": name})

    async def service_stop(self, name: str) -> dict[str, Any]:
        return await self._request("service.stop", {"name": name})

    async def service_restart(self, name: str) -> dict[str, Any]:
        return await self._request("service.restart", {"name": name})

    async def service_enable(self, name: str, runtime: bool = False) -> dict[str, Any]:
        return await self._request("service.enable", {"name": name, "runtime": runtime})

    async def service_disable(self, name: str, runtime: bool = False) -> dict[str, Any]:
        return await self._request("service.disable", {"name": name, "runtime": runtime})

    async def service_list(self, unit_type: str | None = None) -> list[dict[str, Any]]:
        params: dict[str, Any] = {}
        if unit_type is not None:
            params["unit_type"] = unit_type
        response = await self._request("service.list", params)
        return list(response.get("units", []))

    async def journal_query(
        self,
        since: int | None = None,
        until: int | None = None,
        unit: str | None = None,
        priority: int | None = None,
        tail: int | None = None,
    ) -> list[str]:
        params: dict[str, Any] = {}
        for key, value in {
            "since": since,
            "until": until,
            "unit": unit,
            "priority": priority,
            "tail": tail,
        }.items():
            if value is not None:
                params[key] = value
        response = await self._request("journal.query", params)
        return list(response.get("lines", []))

    async def timer_list(self) -> list[dict[str, Any]]:
        response = await self._request("timer.list")
        return list(response.get("units", []))

    async def timer_start(self, name: str) -> dict[str, Any]:
        return await self._request("timer.start", {"name": name})

    async def timer_stop(self, name: str) -> dict[str, Any]:
        return await self._request("timer.stop", {"name": name})

    async def terminal_create(
        self,
        shell: str | None = None,
        cwd: str | None = None,
        env: dict[str, str] | None = None,
        rows: int | None = None,
        cols: int | None = None,
    ) -> dict[str, Any]:
        params: dict[str, Any] = {}
        for key, value in {
            "shell": shell,
            "cwd": cwd,
            "env": env,
            "rows": rows,
            "cols": cols,
        }.items():
            if value is not None:
                params[key] = value
        return await self._request("terminal.create", params)

    async def terminal_write(self, terminal_id: str, input: str) -> dict[str, Any]:
        return await self._request(
            "terminal.write",
            {"terminal_id": terminal_id, "input": input},
        )

    async def terminal_read(
        self,
        terminal_id: str,
        max_bytes: int | None = None,
        flush: bool = True,
    ) -> dict[str, Any]:
        params: dict[str, Any] = {"terminal_id": terminal_id, "flush": flush}
        if max_bytes is not None:
            params["max_bytes"] = max_bytes
        return await self._request("terminal.read", params)

    async def terminal_resize(self, terminal_id: str, rows: int, cols: int) -> dict[str, Any]:
        return await self._request(
            "terminal.resize",
            {"terminal_id": terminal_id, "rows": rows, "cols": cols},
        )

    async def terminal_list(self) -> list[dict[str, Any]]:
        response = await self._request("terminal.list")
        return list(response.get("terminals", []))

    async def terminal_kill(
        self,
        terminal_id: str,
        signal: str | None = None,
    ) -> dict[str, Any]:
        params: dict[str, Any] = {"terminal_id": terminal_id}
        if signal is not None:
            params["signal"] = signal
        return await self._request("terminal.kill", params)
