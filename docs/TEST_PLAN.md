# Deskbrid v1.0.0 Test Plan

## 1. Persistence Layer

### 1.1 Schema Migrations
- **What is being tested**: The ability to apply schema migrations from an older version to the current schema version (CURRENT_SCHEMA_VERSION = 1) and handle future migrations.
- **Expected behavior**: 
  - When starting with a database of version 0, the migration system should run the v0→v1 migration (which is currently a no-op because tables are created by `init_db`).
  - When starting with a database of version 1, no migrations should run.
  - When starting with a database of an unknown version (e.g., 2), the system should return an error and not start.
- **Failure in production**: 
  - If migrations fail to apply, the database might be in an inconsistent state, leading to potential data loss or corruption. 
  - If the system incorrectly allows an unknown version, it might run with a schema that doesn't match the code, causing runtime errors or data corruption.

### 1.2 Cache/DB Consistency
- **What is being tested**: Consistency between in-memory caches (if any) and the persistent database for all persisted entities (clipboard history, audit log, notifications, macros, blackboard, sessions, rules).
- **Expected behavior**: 
  - Any change made through the daemon's API (e.g., adding a clipboard item, updating a macro) should be immediately reflected in the database and vice versa (after a restart).
  - The database should be the source of truth; after a restart, the daemon should reload all persistent state from the database.
- **Failure in production**: 
  - Inconsistencies could lead to lost user data (e.g., clipboard history disappearing), incorrect audit logs, or macros not being available after restart.
  - This could break user trust and lead to data integrity issues.

### 1.3 Restart Survival
- **What is being tested**: The daemon's ability to persist and restore state across restarts.
- **Expected behavior**: 
  - After a graceful shutdown (SIGTERM) or crash (SIGKILL), upon restart the daemon should recover all persistent state (clipboard history, audit log, notifications, macros, blackboard, sessions, rules) exactly as it was before the shutdown/crash.
  - No data should be lost, and the daemon should resume normal operation.
- **Failure in production**: 
  - Data loss on restart would be unacceptable for a persistent service. Users would lose clipboard history, macros, etc.
  - Inconsistent state after restart could lead to undefined behavior or crashes.

## 2. Security Properties

### 2.1 Permissions Deny-All on Parse Failure
- **What is being tested**: The behavior of the permissions system when the permissions file is missing, malformed, or contains invalid JSON.
- **Expected behavior**: 
  - If the permissions file does not exist, the system should load an allow-all permissions set (as a fallback for first-time use) and log an info message.
  - If the permissions file exists but contains invalid JSON or fails to parse, the system should load a deny-all permissions set to prevent accidental over-permissioning and log an error.
- **Failure in production**: 
  - If a malformed permissions file resulted in allow-all, it could grant excessive permissions to agents, leading to security vulnerabilities (e.g., unintended file access, system control).
  - If the system failed to load any permissions and crashed, it would cause a denial of service.

### 2.2 HIGH_RISK_ACTIONS Wildcard Blocking
- **What is being tested**: The enforcement of the HIGH_RISK_ACTIONS list to block actions deemed high risk unless explicitly allowed in the permissions file.
- **Expected behavior**: 
  - Actions listed in `HIGH_RISK_ACTIONS` (e.g., "shell.execute", "file.write" to sensitive paths) should be blocked by default unless the user has explicitly added an allow rule for that specific action in their permissions file.
  - The permissions check should occur early in the action execution pipeline, before the action is dispatched to the backend.
- **Failure in production**: 
  - If a high-risk action is not blocked, it could allow an agent to perform dangerous operations (e.g., executing arbitrary commands, overwriting system files) without explicit user consent, leading to compromise of the system.

### 2.3 Path Sandbox Traversal Prevention
- **What is being tested**: Prevention of directory traversal attacks (e.g., using `../` sequences) in file-related actions (read, write, etc.).
- **Expected behavior**: 
  - All file paths provided in actions (e.g., `file.read`, `file.write`) should be resolved and checked to ensure they remain within allowed directories (likely the user's home directory or configured sandbox).
  - Attempts to escape the sandbox via `../` or symlinks should be blocked and return an error.
- **Failure in production**: 
  - Path traversal could allow an agent to read or write arbitrary files on the system (e.g., `/etc/passwd`, SSH keys, etc.), leading to information disclosure, privilege escalation, or system compromise.

## 3. Confirmation System

### 3.1 Queue Behavior
- **What is being tested**: The behavior of the pending confirmation queue when actions requiring confirmation are issued.
- **Expected behavior**: 
  - When an action is issued that requires confirmation (based on permissions rules), the daemon should not execute it immediately. Instead, it should generate a confirmation request, store it in the pending confirmations queue (with a unique ID), and return a response asking for user confirmation.
  - The queue should store the action details, requester info, and timestamp.
  - Multiple pending confirmations can coexist in the queue.
- **Failure in production**: 
  - If actions requiring confirmation are executed without going through the queue, it would break the confirmation model and allow dangerous actions to run without user approval.
  - If the queue fails to store confirmations properly, users might not be prompted for confirmation, or they might be prompted for the wrong action.

### 3.2 Expiry Sweep
- **What is being tested**: The background task that sweeps and removes expired pending confirmations.
- **Expected behavior**: 
  - Every `SWEEP_INTERVAL_SECS` (30 seconds), the sweeper should iterate over the pending confirmations queue and remove any entry older than `CONFIRMATION_TTL_MS` (5 minutes).
  - The sweeper should log the number of expired confirmations removed.
  - The sweeper should run continuously in the background without blocking the main daemon loop.
- **Failure in production**: 
  - If expired confirmations are not removed, the queue could grow indefinitely, consuming memory and potentially causing a denial of service.
  - If the sweeper removes non-expired confirmations, users might lose the chance to confirm actions that are still valid, leading to frustration and workflow disruption.

### 3.3 Deny Removes from Queue
- **What is being tested**: The effect of issuing a denial (via `confirmation.deny`) on a pending confirmation.
- **Expected behavior**: 
  - When a user denies a pending confirmation (by ID), the daemon should remove that confirmation from the queue and return a denial status.
  - The denied action should not be executed.
  - The queue should no longer contain that confirmation ID.
- **Failure in production**: 
  - If a denial does not remove the confirmation from the queue, the same confirmation might be presented again, causing confusion.
  - If the denial fails to remove the confirmation and the action is later executed (e.g., via a timeout or bug), it would break the denial guarantee and allow a denied action to run.

---
**Note**: This test plan is intended for Tuck to implement. Tests should be written as unit tests, integration tests, or end-to-end tests as appropriate. Use the existing test structure in `src/daemon/persistence/tests.rs` and similar as a guide.