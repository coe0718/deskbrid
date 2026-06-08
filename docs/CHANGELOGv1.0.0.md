# Deskbrid v1.0.0 — What’s New

v1.0.0 marks the first stable release of Deskbrid, a Linux desktop control daemon originally prototyped by an AI agent and refined into a production-ready tool for AI-driven automation. This release solidifies Deskbrid as a reliable, secure, and extensible bridge between autonomous agents and the Linux desktop, featuring a persistent SQLite backend, a pluggable rules engine, granular rate limiting, and a hardened security model. Every core subsystem has been hardened with tests, explicit error handling, and clear boundaries — making it suitable for unattended agent workflows.

## New in v1.0.0

#84 — DB as source of truth. Synchronous writes, std::sync::Mutex, spawn_blocking, PRAGMA user_version schema migrations, 23 new tests. 7 commits, ~500 lines.
#96 — System Pressure/PSI. /proc/pressure/{cpu,memory,io}, dashboard card, unit tests. 3 commits, ~100 lines.
#29 — Secret/Keyring. secret-tool executor, protocol actions, MCP tools, CLI subcommand, dashboard card, confirmation-gated. 3 commits, ~500 lines.
#129 — Per-namespace per-UID rate limiting. 8 namespaces, token buckets, permissions.toml config, wildcard 120/min, UID isolation tests. 1 commit, +285 lines.
#83 — Rules engine v1.0.0. TimeRange timer, VarEquals/VarExists condition evaluator, app_id resolution from window list, zero stubs. 4 commits, ~300 lines.

## Security

Deskbrid v1.0.0 adopts a defense-in-depth security model: permissions default to deny-all on file parse failure, HIGH_RISK_ACTIONS are blocked unless explicitly allowed, all file operations are confined to a user-owned sandbox via path resolution, destructive actions require explicit confirmation through the confirmation system, every action is immutably logged to the audit trail, and per-action, per-UID rate limiting prevents quota exhaustion and credential harvesting. Secret access via the keyring executor is additionally gated by the confirmation system, ensuring no silent extraction.

## From v0.x

Compared to earlier pre-releases, v1.0.0 centralizes state in a SQLite database (replacing ad-hoc in-memory caches), introduces a fully featured rules engine with time-based timers and variable conditions, and hardens the security posture with explicit deny-by-default principles, per-UID isolation, and middleware-style rate limiting. The architectural shift transforms Deskbrid from a prototype agent tool into a auditable, controllable service fit for long-running agent orchestration.