from __future__ import annotations

import asyncio
import contextlib
import json
import os
import threading
import uuid
from concurrent.futures import Future
from typing import Any, Callable

from .events import EventManager
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


class DeskbridError(RuntimeError):
    def __init__(self, code: str, message: str) -> None:
        super().__init__(f"{code}: {message}")
        self.code = code
        self.message = message


def default_socket_path() -> str:
    runtime = os.environ.get("XDG_RUNTIME_DIR", "/run/user/1000")
    return os.path.join(runtime, "deskbrid.sock")


class AsyncDeskbrid:
    def __init__(
        self,
        socket_path: str | None = None,
        reconnect_delay: float = 1.0,
    ) -> None:
        self.socket_path = socket_path or default_socket_path()
        self.reconnect_delay = reconnect_delay
        self._events = EventManager()
        self._reader: asyncio.StreamReader | None = None
        self._writer: asyncio.StreamWriter | None = None
        self._pending: dict[str, asyncio.Future[dict[str, Any]]] = {}
        self._send_lock = asyncio.Lock()
        self._connect_lock = asyncio.Lock()
        self._connected = asyncio.Event()
        self._closed = False
        self._reader_task: asyncio.Task[None] | None = None
        self._reconnect_task: asyncio.Task[None] | None = None
        self._server_info: dict[str, Any] | None = None
        self._closed_event = asyncio.Event()

    @property
    def version(self) -> str:
        if self._server_info:
            data = self._server_info.get("data", {})
            return str(data.get("version", "unknown"))
        return "unknown"

    async def connect(self) -> None:
        should_resubscribe = False
        async with self._connect_lock:
            if self._closed:
                raise DeskbridError("connection_closed", "client is closed")
            if self._writer is not None and not self._writer.is_closing():
                self._connected.set()
                return

            reader, writer = await asyncio.open_unix_connection(self.socket_path)
            try:
                server_msg = await self._read_message_from(reader)
                if server_msg.get("type") != "connected":
                    raise DeskbridError("protocol_error", f"expected connected message, got {server_msg.get('type')}")
            except Exception:
                writer.close()
                with contextlib.suppress(Exception):
                    await writer.wait_closed()
                raise

            self._reader = reader
            self._writer = writer
            self._server_info = server_msg
            self._connected.set()
            self._reader_task = asyncio.create_task(self._read_loop())
            should_resubscribe = bool(self._events.subscribed_events())

        if should_resubscribe:
            await self._resubscribe()

    async def close(self) -> None:
        self._closed = True
        self._closed_event.set()
        self._connected.clear()
        if self._reconnect_task is not None:
            self._reconnect_task.cancel()
            with contextlib.suppress(asyncio.CancelledError):
                await self._reconnect_task
        if self._reader_task is not None:
            self._reader_task.cancel()
            with contextlib.suppress(asyncio.CancelledError):
                await self._reader_task
        await self._drop_connection("connection_closed", "client closed")

    def on(self, event: str) -> Callable[[Callable[[Any], Any]], Callable[[Any], Any]]:
        def decorator(callback: Callable[[Any], Any]) -> Callable[[Any], Any]:
            self._events.add_listener(event, callback)
            if self._connected.is_set():
                asyncio.create_task(self._sync_subscriptions())
            return callback

        return decorator

    async def subscribe(self, *events: str) -> None:
        for event in events:
            self._events.add_listener(event, lambda _payload: None)
        await self._sync_subscriptions()

    async def listen(self) -> None:
        await self.connect()
        await self._closed_event.wait()

    # ─── Actions ───────────────────────────────────────

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
        return list(response.get("profiles", []))

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

    async def info(self) -> DaemonInfo:
        return decode_info(await self._request("system.info"))

    # ─── Request/response internals ────────────────────

    async def _request(self, action_type: str, params: dict[str, Any] | None = None) -> dict[str, Any]:
        request_id = str(uuid.uuid4())
        message: dict[str, Any] = {"type": action_type, "id": request_id}
        if params:
            # Flatten params into the message envelope (daemon expects flat keys)
            for key, value in params.items():
                if key not in ("type", "id"):
                    message[key] = value

        loop = asyncio.get_running_loop()
        future: asyncio.Future[dict[str, Any]] = loop.create_future()
        self._pending[request_id] = future

        try:
            await self._send_message(message)
            result = await future
        except Exception:
            self._pending.pop(request_id, None)
            raise

        status = result.get("status", "error")
        if status != "ok":
            error_body = result.get("error", {})
            raise DeskbridError(
                str(error_body.get("code", "internal_error")),
                str(error_body.get("message", "request failed")),
            )

        data = result.get("data")
        if isinstance(data, dict):
            return data
        if isinstance(data, list):
            return {"data": data}
        return {}

    async def _send_message(self, message: dict[str, Any]) -> None:
        await self.connect()
        async with self._send_lock:
            writer = self._writer
            if writer is None or writer.is_closing():
                await self._schedule_reconnect()
                raise DeskbridError("connection_closed", "socket writer unavailable")
            try:
                writer.write(json.dumps(message).encode("utf-8") + b"\n")
                await writer.drain()
            except (ConnectionError, BrokenPipeError) as exc:
                await self._handle_disconnect("connection_closed", str(exc))
                raise DeskbridError("connection_closed", str(exc)) from exc

    async def _read_loop(self) -> None:
        try:
            while not self._closed:
                message = await self._read_message()
                if message is None:
                    break
                msg_type = message.get("type")
                if msg_type == "event":
                    event_id = str(message.get("id", ""))
                    payload = message.get("data")
                    if isinstance(payload, dict):
                        await self._events.dispatch(event_id, payload)
                elif msg_type == "response":
                    request_id = str(message.get("id", ""))
                    future = self._pending.pop(request_id, None)
                    if future is not None and not future.done():
                        future.set_result(message)
        except asyncio.CancelledError:
            raise
        except Exception as exc:
            await self._handle_disconnect("connection_closed", str(exc))
            return

        if not self._closed:
            await self._handle_disconnect("connection_closed", "socket closed")

    async def _read_message(self) -> dict[str, Any] | None:
        reader = self._reader
        if reader is None:
            return None
        return await self._read_message_from(reader)

    async def _read_message_from(self, reader: asyncio.StreamReader) -> dict[str, Any]:
        line = await reader.readline()
        if not line:
            raise DeskbridError("connection_closed", "socket closed")
        if len(line) > 1024 * 1024:
            raise DeskbridError("protocol_error", "message exceeds 1 MiB")
        payload = json.loads(line.decode("utf-8"))
        if not isinstance(payload, dict):
            raise DeskbridError("protocol_error", "message was not a JSON object")
        return payload

    async def _sync_subscriptions(self) -> None:
        events = self._events.subscribed_events()
        if not events:
            return
        request_id = str(uuid.uuid4())
        await self._send_message({"type": "subscribe", "id": request_id, "events": events})

    async def _resubscribe(self) -> None:
        if self._events.subscribed_events():
            await self._sync_subscriptions()

    async def _schedule_reconnect(self) -> None:
        if self._closed:
            return
        if self._reconnect_task is None or self._reconnect_task.done():
            self._reconnect_task = asyncio.create_task(self._reconnect_loop())

    async def _reconnect_loop(self) -> None:
        while not self._closed and not self._connected.is_set():
            try:
                await self.connect()
                return
            except Exception:
                await asyncio.sleep(self.reconnect_delay)

    async def _handle_disconnect(self, code: str, message: str) -> None:
        await self._drop_connection(code, message)
        await self._schedule_reconnect()

    async def _drop_connection(self, code: str, message: str) -> None:
        self._connected.clear()
        writer = self._writer
        self._reader = None
        self._writer = None
        if writer is not None:
            writer.close()
            with contextlib.suppress(Exception):
                await writer.wait_closed()
        pending = list(self._pending.values())
        self._pending.clear()
        for future in pending:
            if not future.done():
                future.set_exception(DeskbridError(code, message))


class _LoopThread:
    def __init__(self) -> None:
        self._ready = threading.Event()
        self._loop: asyncio.AbstractEventLoop | None = None
        self._thread = threading.Thread(target=self._run, daemon=True)
        self._thread.start()
        self._ready.wait()

    def _run(self) -> None:
        loop = asyncio.new_event_loop()
        self._loop = loop
        asyncio.set_event_loop(loop)
        self._ready.set()
        loop.run_forever()

    def submit(self, coroutine: Any) -> Future[Any]:
        if self._loop is None:
            raise RuntimeError("event loop not initialized")
        return asyncio.run_coroutine_threadsafe(coroutine, self._loop)

    def stop(self) -> None:
        if self._loop is None:
            return
        self._loop.call_soon_threadsafe(self._loop.stop)
        self._thread.join(timeout=2)


class Deskbrid:
    def __init__(
        self,
        socket_path: str | None = None,
        reconnect_delay: float = 1.0,
    ) -> None:
        self._loop = _LoopThread()
        self._closed_event = threading.Event()
        self._client = self._loop.submit(
            self._create_client(socket_path=socket_path, reconnect_delay=reconnect_delay)
        ).result()

    async def _create_client(
        self,
        socket_path: str | None,
        reconnect_delay: float,
    ) -> AsyncDeskbrid:
        client = AsyncDeskbrid(socket_path=socket_path, reconnect_delay=reconnect_delay)
        await client.connect()
        return client

    @property
    def version(self) -> str:
        return self._client.version

    def close(self) -> None:
        self._loop.submit(self._client.close()).result()
        self._closed_event.set()
        self._loop.stop()

    def on(self, event: str) -> Callable[[Callable[[Any], Any]], Callable[[Any], Any]]:
        def decorator(callback: Callable[[Any], Any]) -> Callable[[Any], Any]:
            self._loop.submit(self._register_listener(event, callback)).result()
            return callback

        return decorator

    async def _register_listener(self, event: str, callback: Callable[[Any], Any]) -> None:
        self._client.on(event)(callback)
        await self._client._sync_subscriptions()

    def listen(self) -> None:
        try:
            self._loop.submit(self._client.connect()).result()
            self._closed_event.wait()
        except KeyboardInterrupt:
            self.close()
            raise

    def type_text(self, text: str) -> None:
        self._loop.submit(self._client.type_text(text)).result()

    def send_keys(self, keys: list[str]) -> None:
        self._loop.submit(self._client.send_keys(keys)).result()

    def mouse_click(self, x: int, y: int, button: str = "left") -> None:
        self._loop.submit(self._client.mouse_click(x=x, y=y, button=button)).result()

    def clipboard_read(self) -> ClipboardContent:
        return self._loop.submit(self._client.clipboard_read()).result()

    def clipboard_write(self, text: str) -> None:
        self._loop.submit(self._client.clipboard_write(text)).result()

    def screenshot(self, monitor: int | None = None) -> str:
        result = self._loop.submit(self._client.screenshot(monitor=monitor)).result()
        return result.path

    def notify(self, title: str, body: str = "", urgency: str = "normal") -> int:
        return self._loop.submit(self._client.notify(title, body, urgency)).result()

    def list_windows(self) -> list[WindowInfo]:
        return self._loop.submit(self._client.list_windows()).result()

    def focus_window(
        self,
        *,
        app_id: str | None = None,
        title: str | None = None,
        exact: bool = False,
    ) -> None:
        self._loop.submit(
            self._client.focus_window(app_id=app_id, title=title, exact=exact)
        ).result()

    def activate_or_launch(
        self,
        app_id: str,
        command: list[str] | None = None,
        workdir: str | None = None,
        env: dict[str, str] | None = None,
    ) -> dict[str, Any]:
        return self._loop.submit(
            self._client.activate_or_launch(
                app_id=app_id,
                command=command,
                workdir=workdir,
                env=env,
            )
        ).result()

    def list_layout_profiles(self) -> list[dict[str, Any]]:
        return self._loop.submit(self._client.list_layout_profiles()).result()

    def save_layout_profile(self, name: str, overwrite: bool = False) -> dict[str, Any]:
        return self._loop.submit(
            self._client.save_layout_profile(name=name, overwrite=overwrite)
        ).result()

    def get_layout_profile(self, name: str) -> dict[str, Any]:
        return self._loop.submit(self._client.get_layout_profile(name)).result()

    def delete_layout_profile(self, name: str) -> dict[str, Any]:
        return self._loop.submit(self._client.delete_layout_profile(name)).result()

    def restore_layout_profile(self, name: str) -> dict[str, Any]:
        return self._loop.submit(self._client.restore_layout_profile(name)).result()

    def list_displays(self) -> list[MonitorInfo]:
        return self._loop.submit(self._client.list_displays()).result()

    def info(self) -> DaemonInfo:
        return self._loop.submit(self._client.info()).result()
