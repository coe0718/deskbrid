from __future__ import annotations

import asyncio
import inspect
from collections import defaultdict
from typing import Any, Awaitable, Callable, DefaultDict

from .models import decode_event

EventCallback = Callable[[Any], Any]


class EventManager:
    def __init__(self) -> None:
        self._callbacks: DefaultDict[str, list[EventCallback]] = defaultdict(list)

    def add_listener(self, event: str, callback: EventCallback) -> EventCallback:
        self._callbacks[event].append(callback)
        return callback

    def remove_listener(self, event: str, callback: EventCallback) -> None:
        callbacks = self._callbacks.get(event)
        if not callbacks:
            return
        self._callbacks[event] = [item for item in callbacks if item is not callback]
        if not self._callbacks[event]:
            self._callbacks.pop(event, None)

    def subscribed_events(self) -> list[str]:
        return sorted(self._callbacks)

    async def dispatch(self, event: str, payload: dict[str, Any]) -> None:
        callbacks = list(self._callbacks.get(event, ()))
        if not callbacks:
            return

        data = decode_event(event, payload)
        for callback in callbacks:
            result = callback(data)
            if inspect.isawaitable(result):
                await result


async def maybe_await(value: Awaitable[Any] | Any) -> Any:
    if inspect.isawaitable(value):
        return await value
    return value
