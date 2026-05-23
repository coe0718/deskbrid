"""Keyboard and mouse input tools for the deskbrid Hermes plugin."""
from __future__ import annotations

import json

from ._socket import send_action


def _register_input_tools(ctx):
    """Keyboard and mouse input tools."""
    ctx.register_tool(
        name="type_text",
        toolset="deskbrid",
        schema={
            "name": "type_text",
            "description": "Type a string of text via keyboard input. Types at human speed.",
            "parameters": {
                "type": "object",
                "properties": {"text": {"type": "string", "description": "Text to type"}},
                "required": ["text"],
            },
        },
        handler=lambda params, **kw: json.dumps(
            send_action({"type": "input.keyboard", "id": "1", "action": "type", "text": params.get("text", "")})
        ),
        description="Type text via keyboard input.",
    )

    ctx.register_tool(
        name="press_key",
        toolset="deskbrid",
        schema={
            "name": "press_key",
            "description": "Press and release a single key by name (e.g., 'Return', 'Escape', 'Tab', 'F5').",
            "parameters": {
                "type": "object",
                "properties": {"key": {"type": "string", "description": "Key name (X11 keysym)"}},
                "required": ["key"],
            },
        },
        handler=lambda params, **kw: json.dumps(
            send_action({"type": "input.keyboard", "id": "1", "action": "key", "key": params.get("key", "")})
        ),
        description="Press a single key.",
    )

    ctx.register_tool(
        name="press_keys",
        toolset="deskbrid",
        schema={
            "name": "press_keys",
            "description": "Press a key combination. Keys pressed in order, released in reverse.",
            "parameters": {
                "type": "object",
                "properties": {
                    "keys": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Keys to press, e.g., ['Control_L', 'c']",
                    },
                },
                "required": ["keys"],
            },
        },
        handler=lambda params, **kw: json.dumps(
            send_action({
                "type": "input.keyboard", "id": "1", "action": "combo",
                "keys": params.get("keys", []),
            })
        ),
        description="Press a key combination.",
    )

    ctx.register_tool(
        name="mouse_move",
        toolset="deskbrid",
        schema={
            "name": "mouse_move",
            "description": "Move the mouse cursor to absolute screen coordinates.",
            "parameters": {
                "type": "object",
                "properties": {
                    "x": {"type": "number", "description": "X coordinate (pixels)"},
                    "y": {"type": "number", "description": "Y coordinate (pixels)"},
                },
                "required": ["x", "y"],
            },
        },
        handler=lambda params, **kw: json.dumps(
            send_action({"type": "input.mouse", "id": "1", "action": "move", "x": params.get("x", 0), "y": params.get("y", 0)})
        ),
        description="Move the mouse cursor.",
    )

    ctx.register_tool(
        name="mouse_click",
        toolset="deskbrid",
        schema={
            "name": "mouse_click",
            "description": "Click a mouse button at the current cursor position.",
            "parameters": {
                "type": "object",
                "properties": {
                    "button": {
                        "type": "string",
                        "enum": ["left", "middle", "right"],
                        "description": "Mouse button to click",
                    },
                },
                "required": ["button"],
            },
        },
        handler=lambda params, **kw: json.dumps(
            send_action({"type": "input.mouse", "id": "1", "action": "click", "button": params.get("button", "left")})
        ),
        description="Click a mouse button.",
    )

    ctx.register_tool(
        name="mouse_scroll",
        toolset="deskbrid",
        schema={
            "name": "mouse_scroll",
            "description": "Scroll the mouse wheel. Positive dy = scroll down, negative = scroll up.",
            "parameters": {
                "type": "object",
                "properties": {
                    "dx": {"type": "number", "description": "Horizontal scroll (default: 0)"},
                    "dy": {"type": "number", "description": "Vertical scroll (positive = down)"},
                },
                "required": [],
            },
        },
        handler=lambda params, **kw: json.dumps(
            send_action({"type": "input.mouse", "id": "1", "action": "scroll", "dx": params.get("dx", 0), "dy": params.get("dy", 0)})
        ),
        description="Scroll the mouse wheel.",
    )

    ctx.register_tool(
        name="click_coordinate",
        toolset="deskbrid",
        schema={
            "name": "click_coordinate",
            "description": "Move to pixel coordinates and click. Combines mouse_move + mouse_click.",
            "parameters": {
                "type": "object",
                "properties": {
                    "x": {"type": "number"},
                    "y": {"type": "number"},
                    "button": {"type": "string", "enum": ["left", "middle", "right"]},
                },
                "required": ["x", "y"],
            },
        },
        handler=lambda params, **kw: json.dumps(
            send_action({
                "type": "input.mouse", "id": "1", "action": "click",
                "x": params.get("x", 0), "y": params.get("y", 0),
                "button": params.get("button", "left"),
            })
        ),
        description="Click at specific screen coordinates.",
    )
