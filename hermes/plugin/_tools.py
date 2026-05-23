"""Window management and workspace tools for the deskbrid Hermes plugin."""
from __future__ import annotations

import json

from ._socket import send_action


def _register_window_tools(ctx):
    """Window management tools."""
    ctx.register_tool(
        name="list_windows",
        toolset="deskbrid",
        schema={
            "name": "list_windows",
            "description": "List all open windows on the Linux desktop. Returns window IDs, titles, classes, workspace, and geometry.",
            "parameters": {"type": "object", "properties": {}, "required": []},
        },
        handler=lambda params, **kw: json.dumps(
            send_action({"type": "windows.list", "id": "1"})
        ),
        description="List all open windows on the desktop.",
    )

    ctx.register_tool(
        name="focus_window",
        toolset="deskbrid",
        schema={
            "name": "focus_window",
            "description": "Focus (activate) a window by its ID.",
            "parameters": {
                "type": "object",
                "properties": {"window_id": {"type": "string", "description": "Window ID from list_windows"}},
                "required": ["window_id"],
            },
        },
        handler=lambda params, **kw: json.dumps(
            send_action({"type": "windows.focus", "id": "1", "window_id": params.get("window_id", "")})
        ),
        description="Focus a window by its ID.",
    )

    ctx.register_tool(
        name="close_window",
        toolset="deskbrid",
        schema={
            "name": "close_window",
            "description": "Close a window by its ID.",
            "parameters": {
                "type": "object",
                "properties": {"window_id": {"type": "string"}},
                "required": ["window_id"],
            },
        },
        handler=lambda params, **kw: json.dumps(
            send_action({"type": "windows.close", "id": "1", "window_id": params.get("window_id", "")})
        ),
        description="Close a window by its ID.",
    )

    ctx.register_tool(
        name="minimize_window",
        toolset="deskbrid",
        schema={
            "name": "minimize_window",
            "description": "Minimize a window by its ID.",
            "parameters": {
                "type": "object",
                "properties": {"window_id": {"type": "string"}},
                "required": ["window_id"],
            },
        },
        handler=lambda params, **kw: json.dumps(
            send_action({"type": "windows.minimize", "id": "1", "window_id": params.get("window_id", "")})
        ),
        description="Minimize a window.",
    )

    ctx.register_tool(
        name="maximize_window",
        toolset="deskbrid",
        schema={
            "name": "maximize_window",
            "description": "Maximize (or un-maximize) a window.",
            "parameters": {
                "type": "object",
                "properties": {"window_id": {"type": "string"}},
                "required": ["window_id"],
            },
        },
        handler=lambda params, **kw: json.dumps(
            send_action({"type": "windows.maximize", "id": "1", "window_id": params.get("window_id", "")})
        ),
        description="Maximize a window.",
    )

    ctx.register_tool(
        name="move_resize_window",
        toolset="deskbrid",
        schema={
            "name": "move_resize_window",
            "description": "Move and/or resize a window. Coordinates are screen pixels.",
            "parameters": {
                "type": "object",
                "properties": {
                    "window_id": {"type": "string"},
                    "x": {"type": "integer", "description": "New X position"},
                    "y": {"type": "integer", "description": "New Y position"},
                    "width": {"type": "integer", "description": "New width"},
                    "height": {"type": "integer", "description": "New height"},
                },
                "required": ["window_id", "x", "y", "width", "height"],
            },
        },
        handler=lambda params, **kw: json.dumps(
            send_action({
                "type": "windows.move_resize",
                "id": "1",
                "window_id": params.get("window_id", ""),
                "x": params.get("x", 0),
                "y": params.get("y", 0),
                "width": params.get("width", 800),
                "height": params.get("height", 600),
            })
        ),
        description="Move and/or resize a window.",
    )

    ctx.register_tool(
        name="list_workspaces",
        toolset="deskbrid",
        schema={
            "name": "list_workspaces",
            "description": "List all workspaces/virtual desktops with their current state.",
            "parameters": {"type": "object", "properties": {}, "required": []},
        },
        handler=lambda params, **kw: json.dumps(
            send_action({"type": "workspaces.list", "id": "1"})
        ),
        description="List all workspaces/virtual desktops.",
    )

    ctx.register_tool(
        name="switch_workspace",
        toolset="deskbrid",
        schema={
            "name": "switch_workspace",
            "description": "Switch to a specific workspace by number.",
            "parameters": {
                "type": "object",
                "properties": {"workspace_id": {"type": "integer"}},
                "required": ["workspace_id"],
            },
        },
        handler=lambda params, **kw: json.dumps(
            send_action({
                "type": "workspaces.switch",
                "id": "1",
                "workspace_id": params.get("workspace_id", 0),
            })
        ),
        description="Switch to a workspace by number.",
    )
