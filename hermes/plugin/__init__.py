"""Deskbrid plugin — Linux desktop control for Hermes agents.

Three integration modes (auto-detected):
  1. MCP mode (preferred) — deskbrid mcp subprocess, Hermes' built-in MCP client
  2. Direct socket mode — talks to deskbrid daemon over Unix socket
  3. Subprocess mode — spawns deskbrid for one-shot commands (fallback)

Agents get: windows, input, clipboard, screenshots, AT-SPI, audio, system."""
from __future__ import annotations

import logging
from pathlib import Path

from ._socket import _ensure_daemon_started
from ._tools import _register_window_tools
from ._tools_input import _register_input_tools
from ._tools_more import (
    _register_audio_tools,
    _register_clipboard_tools,
    _register_screenshot_tools,
    _register_system_tools,
)

logger = logging.getLogger(__name__)


def register(ctx):
    """Wire deskbrid into Hermes — tools, hooks, and bundled skill."""

    skill_path = Path(__file__).parent / "skills" / "deskbrid.md"
    if skill_path.exists():
        ctx.register_skill("deskbrid", str(skill_path))
        logger.info("Registered bundled skill: deskbrid")

    ctx.register_hook("pre_agent_start", _ensure_daemon_started)

    _register_window_tools(ctx)
    _register_input_tools(ctx)
    _register_clipboard_tools(ctx)
    _register_screenshot_tools(ctx)
    _register_system_tools(ctx)
    _register_audio_tools(ctx)

    ctx.register_cli_command(
        name="deskbrid-setup",
        help="Install and configure deskbrid daemon",
        setup_fn=None,
        handler_fn=_cli_setup,
    )


def _cli_setup(**kwargs) -> str:
    """CLI command: hermes deskbrid-setup"""
    import subprocess

    try:
        result = subprocess.run(
            ["deskbrid", "setup"], capture_output=True, text=True, timeout=30
        )
        return result.stdout or "Setup complete (no output)"
    except FileNotFoundError:
        return "deskbrid not installed. Run: curl -fsSL https://deskbrid.patchhive.dev/install.sh | bash"
    except subprocess.TimeoutExpired:
        return "Setup timed out"
