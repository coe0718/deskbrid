# Deskbrid whole-repository code review

Date: 2026-07-29

Reviewed revision: `6f68f47` (`main` before the fixes in this review)

Scope: Rust daemon/CLI/backends/MCP, Unix and TCP transports, dashboard, permissions and confirmation pipeline, persistence, updater/installer, Python client, GNOME extension, shell scripts, CI/release configuration, and dependency graph.

## Executive summary

The review found nine actionable issues. All nine are fixed in the same change set as this report. The most important issue was that confirmed actions bypassed the normal dispatcher: some specialized actions failed after approval, while successful actions skipped current safety, timeout, routing, and audit behavior. A second high-impact issue allowed a slow subscribed client to accumulate an unbounded per-connection event queue.

No known vulnerable packages remain in `Cargo.lock` after the dependency update. The automated Rust, Python, JavaScript, shell-syntax, dependency-usage, and advisory checks listed below pass.

## Fixed findings

### DB-CR-001 — High — Confirmed actions bypassed normal dispatch

`confirmation.confirm` removed a pending request and called the generic backend executor directly. This bypassed specialized routers for agent, lock, terminal, secrets, session, rule, macro, and other backend-free actions. It also skipped current permission/profile checks, rate limits, action timeouts, auto-suspend accounting, agent-registry accounting, and the normal audit response path.

Impact:

- Valid specialized actions could fail after a user approved them.
- Actions that did execute did not receive the same safety and observability guarantees as ordinary requests.

Fix:

- Added an internal confirmed-dispatch context that re-enters the complete dispatcher and skips only creation of a second confirmation request.
- Added a regression test that approves `agent.list`, proving that a backend-free specialized action completes through the correct router.

### DB-CR-002 — High — Event forwarding could consume unbounded memory

Each client had a bounded Tokio broadcast receiver feeding an unbounded MPSC queue. If a subscribed client stopped reading its socket, the socket writer could block while the forwarding task continued appending serialized events without a memory bound.

Impact: a slow or intentionally stalled subscribed client could grow daemon memory until process or system exhaustion.

Fix: replaced the unbounded hop with a 256-entry bounded channel. Backpressure now reaches the already bounded broadcast receiver, whose lag behavior drops old events instead of growing process memory.

### DB-CR-003 — Medium — Network authentication handshakes had no deadline

The general TCP listener, MCP TCP listener, and dashboard request-header reader waited indefinitely for authentication/request bytes. A client could open connections and drip or withhold bytes, retaining tasks and file descriptors before authentication.

Fix:

- Added a 10-second overall authentication deadline to TCP and MCP TCP handshakes.
- Added a 10-second overall request-line/header deadline to the dashboard, returning HTTP 408 on expiry.
- Added regression tests for stalled TCP authentication and dashboard headers.

### DB-CR-004 — Medium — Dashboard lowercased case-sensitive bearer tokens

Dashboard header parsing lowercased the complete `Authorization` line to recognize the field name. That also lowercased the token value, so any configured token containing uppercase characters could never authenticate. The bearer scheme parser also accepted only two exact casing variants.

Fix: parse the field name case-insensitively while preserving the field value, accept the bearer scheme case-insensitively, and compare the preserved token with the existing constant-time helper. A mixed-case regression test covers both behaviors.

### DB-CR-005 — Medium — Dry runs mutated execution safety state

The dispatcher called auto-suspend burst tracking and macro recording before checking `dry_run`. Repeated simulations could suspend a session, and simulated actions could be persisted into an active macro recording even though the CLI promises validation without execution.

Fix: return the audited dry-run result after authorization checks but before auto-suspend, macro-recording, confirmation-queue, and agent-registry mutations. A regression test proves a dry run no longer consumes an auto-suspend burst slot.

### DB-CR-006 — Medium — Locked build dependency had two high-severity advisories

`cargo audit` reported RUSTSEC-2026-0194 and RUSTSEC-2026-0195 against `quick-xml 0.39.4`, pulled in by `wayland-scanner 0.31.10`. The upstream advisories describe quadratic processing and unbounded namespace allocation. In Deskbrid this is a build-time parser over protocol XML rather than a runtime parser over client input, which reduces project-specific exploitability but still leaves an avoidable vulnerable dependency in the locked graph.

Fix: updated `wayland-scanner` to 0.31.11 and `quick-xml` to 0.41.0. `cargo audit` reports no vulnerabilities afterward.

### DB-CR-007 — Low — Dashboard returned incorrect HTTP reason phrases

The response builder recognized only 200 and 404; all other status codes used `Internal Server Error` as the reason phrase. Responses such as `401 Internal Server Error` and `503 Internal Server Error` were misleading to operators and clients.

Fix: added correct reason phrases for every emitted dashboard status and regression coverage for 401 and 503.

### DB-CR-008 — Low — Unused native PipeWire dependencies burdened builds and CI

The optional `pipewire` feature enabled the `pipewire` and `libspa` crates, but no source file referenced the feature or either crate. `cargo machete` reported both as unused. They added a native build dependency, required CI system headers, and substantially enlarged all-features builds without providing a code path. Deskbrid's active capture/audio paths use portals and external desktop tools.

Fix: removed the dead feature and dependencies, pruned their transitive lockfile graph, and removed the unnecessary `libpipewire-0.3-dev` CI package. `cargo machete` now reports no unused dependencies.

### DB-CR-009 — High — AT-SPI action references lost the owning application bus

AT-SPI child traversal correctly receives both an application bus name and an object path. The tree builder used the bus name while collecting node details but returned only the path as `object_ref`. Follow-up action, click, value, and text calls then sent that path to the registry destination rather than to the application that owns the accessible object.

Impact: accessibility snapshots could list application elements, but operations using the returned references could fail or address the wrong destination.

Fix:

- Tree nodes now return an opaque `bus_name|object_path` reference and expose `bus_name` separately for diagnostics.
- All object-reference consumers decode and use the owning bus.
- Legacy bare object paths remain supported and resolve to the registry destination.
- Unit tests cover composite-reference round trips and legacy compatibility.

## Review approach

- Mapped the action parse/serialize/dispatch pipeline and checked high-risk implied permissions.
- Reviewed Unix peer authentication, TCP/MCP token gates, dashboard exposure controls, request bounds, event backpressure, and connection cleanup.
- Reviewed file path expansion/sandbox checks, process and terminal launch points, secrets handling, updater checksum verification, and installer download verification.
- Reviewed async lock use and long-lived/background tasks for unbounded state or waits.
- Checked Rust panic/unsafe/command-execution sites and Python/JavaScript/shell clients and integrations.
- Compared build configuration to actual dependency use and ran vulnerability and unused-dependency analysis.

## Validation

- `cargo test --all-targets --all-features`: 269 unit tests and 1 Linux session smoke test passed.
- `cargo clippy --all-targets --all-features -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- `cargo audit`: no vulnerabilities; two unmaintained transitive-package warnings remain, described below.
- `cargo machete`: no unused dependencies.
- `uv run --isolated --with pytest --project clients/python pytest clients/python/tests -q`: 24 passed.
- `node --check extensions/deskbrid@deskbrid/extension.js`: passed.
- `bash -n` over repository shell scripts: passed.
- `cargo build --release`: passed.

## Residual risks and test limitations

- The review environment cannot exercise real GNOME, KDE, Hyprland, COSMIC, Sway, Niri, Wayfire, Labwc, and X11 sessions simultaneously. Mock/unit coverage and the Linux smoke harness passed, but compositor-specific behavior still requires the documented manual DE matrix.
- Raw TCP and a dashboard bound beyond loopback authenticate with bearer tokens but do not provide transport encryption. Operators should expose them only through a trusted private network, SSH tunnel, VPN, or TLS-terminating proxy.
- The file sandbox canonicalizes paths before filesystem operations. It blocks ordinary traversal and symlink escapes covered by tests, but it is not a descriptor-relative `openat2` sandbox against a hostile same-user process racing path components.
- `cargo audit` warns that transitive crates `instant 0.1.13` (through `notify 7`) and `paste 1.0.15` (through image-codec dependencies) are unmaintained. Neither warning is a known vulnerability. Replacing them requires upstream dependency changes and should be tracked during routine dependency upgrades.
- `shellcheck` was not installed in the review environment; Bash syntax validation passed, and shell code received a manual security/quoting review.

## Conclusion

No unresolved correctness or security defect found by this review remains in the patch. The residual items above are deployment constraints, defense-in-depth limitations, or upstream maintenance warnings rather than regressions introduced by Deskbrid's current logic.
