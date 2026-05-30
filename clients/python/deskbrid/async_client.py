from __future__ import annotations

import asyncio
import contextlib
import json
import os
import uuid
from typing import Any, Callable

from .actions_async import AsyncActionsMixin
from .errors import DeskbridError
from .events import EventManager

def default_socket_path() -> str:
    runtime = os.environ.get("XDG_RUNTIME_DIR")
    if runtime is None:
        uid = os.getuid()
        runtime = f"/run/user/{uid}"
    return os.path.join(runtime, "deskbrid.sock")


class AsyncDeskbrid(AsyncActionsMixin):
    def __init__(
        self,
        socket_path: str | None = None,
        reconnect_delay: float = 1.0,
        tcp_host: str | None = None,
        tcp_port: int | None = None,
        tcp_token: str | None = None,
    ) -> None:
        self.socket_path = socket_path or default_socket_path()
        self.reconnect_delay = reconnect_delay
        # TCP transport — if tcp_port is set, use TCP instead of Unix socket
        self._tcp_host = tcp_host or os.environ.get("DESKBRID_HOST", "127.0.0.1")
        self._tcp_port = tcp_port or (int(os.environ["DESKBRID_PORT"]) if "DESKBRID_PORT" in os.environ else None)
        self._tcp_token = tcp_token or os.environ.get("DESKBRID_TCP_TOKEN")
        self._use_tcp = self._tcp_port is not None
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

            if self._use_tcp:
                reader, writer = await self._connect_tcp()
            else:
                reader, writer = await self._connect_unix()

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

    async def _connect_unix(self) -> tuple[asyncio.StreamReader, asyncio.StreamWriter]:
        return await asyncio.open_unix_connection(self.socket_path)

    async def _connect_tcp(self) -> tuple[asyncio.StreamReader, asyncio.StreamWriter]:
        if not self._tcp_token:
            raise DeskbridError("connection_closed",
                "DESKBRID_TCP_TOKEN must be set for TCP transport")
        reader, writer = await asyncio.open_connection(self._tcp_host, self._tcp_port)

        # Send auth message
        auth_msg = json.dumps({"type": "auth", "token": self._tcp_token}) + "\n"
        writer.write(auth_msg.encode("utf-8"))
        await writer.drain()

        # Read auth response — daemon either sends error or proceeds to protocol
        auth_line = await reader.readline()
        if not auth_line:
            raise DeskbridError("connection_closed", "TCP connection closed during auth")
        auth_resp = json.loads(auth_line.decode("utf-8"))
        if auth_resp.get("status") == "error":
            msg = auth_resp.get("error", {}).get("message", "authentication failed")
            raise DeskbridError("unauthorized", str(msg))

        return reader, writer

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
