"""Clipboard, screenshot, system, and audio tools for the deskbrid Hermes plugin."""
from __future__ import annotations

import json

from ._socket import send_action


def _register_clipboard_tools(ctx):
    """Clipboard read/write tools."""
    ctx.register_tool(
        name="clipboard_read",
        toolset="deskbrid",
        schema={
            "name": "clipboard_read",
            "description": "Read the current clipboard contents.",
            "parameters": {"type": "object", "properties": {}, "required": []},
        },
        handler=lambda params, **kw: json.dumps(
            send_action({"type": "clipboard.read", "id": "1"})
        ),
        description="Read clipboard contents.",
    )

    ctx.register_tool(
        name="clipboard_write",
        toolset="deskbrid",
        schema={
            "name": "clipboard_write",
            "description": "Write text to the system clipboard.",
            "parameters": {
                "type": "object",
                "properties": {"text": {"type": "string", "description": "Text to copy to clipboard"}},
                "required": ["text"],
            },
        },
        handler=lambda params, **kw: json.dumps(
            send_action({"type": "clipboard.write", "id": "1", "text": params.get("text", "")})
        ),
        description="Write text to clipboard.",
    )


def _register_screenshot_tools(ctx):
    """Screenshot tools."""
    ctx.register_tool(
        name="screenshot",
        toolset="deskbrid",
        schema={
            "name": "screenshot",
            "description": "Take a screenshot of the entire desktop as a base64-encoded PNG.",
            "parameters": {"type": "object", "properties": {}, "required": []},
        },
        handler=lambda params, **kw: json.dumps(
            send_action({"type": "screenshot", "id": "1"})
        ),
        description="Take a screenshot of the desktop.",
    )


def _register_system_tools(ctx):
    """System status tools."""
    ctx.register_tool(
        name="system_info",
        toolset="deskbrid",
        schema={
            "name": "system_info",
            "description": "Get system information — hostname, OS, kernel, uptime, memory, CPU.",
            "parameters": {"type": "object", "properties": {}, "required": []},
        },
        handler=lambda params, **kw: json.dumps(
            send_action({"type": "system.info", "id": "1"})
        ),
        description="Get system information.",
    )

    ctx.register_tool(
        name="idle_seconds",
        toolset="deskbrid",
        schema={
            "name": "idle_seconds",
            "description": "Get how long the user has been idle (no mouse/keyboard input) in seconds.",
            "parameters": {"type": "object", "properties": {}, "required": []},
        },
        handler=lambda params, **kw: json.dumps(
            send_action({"type": "system.idle", "id": "1"})
        ),
        description="Get user idle time in seconds.",
    )


def _register_audio_tools(ctx):
    """Audio control tools."""
    ctx.register_tool(
        name="list_audio_sinks",
        toolset="deskbrid",
        schema={
            "name": "list_audio_sinks",
            "description": "List all audio output devices (sinks) with volume and mute status.",
            "parameters": {"type": "object", "properties": {}, "required": []},
        },
        handler=lambda params, **kw: json.dumps(
            send_action({"type": "audio.list_sinks", "id": "1"})
        ),
        description="List audio output devices.",
    )

    ctx.register_tool(
        name="set_volume",
        toolset="deskbrid",
        schema={
            "name": "set_volume",
            "description": "Set the volume of an audio sink. Volume is a float from 0.0 to 1.0.",
            "parameters": {
                "type": "object",
                "properties": {
                    "sink_name": {"type": "string", "description": "Sink name from list_audio_sinks"},
                    "volume": {"type": "number", "description": "Volume level (0.0 - 1.0)"},
                },
                "required": ["sink_name", "volume"],
            },
        },
        handler=lambda params, **kw: json.dumps(
            send_action({
                "type": "audio.set_sink_volume", "id": "1",
                "sink_name": params.get("sink_name", ""),
                "volume": params.get("volume", 0.5),
            })
        ),
        description="Set audio volume.",
    )
