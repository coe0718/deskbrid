from __future__ import annotations

import asyncio
import threading
from concurrent.futures import Future
from typing import Any, Callable

from .actions_sync import SyncActionsMixin
from .async_client import AsyncDeskbrid


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


class Deskbrid(SyncActionsMixin):
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
