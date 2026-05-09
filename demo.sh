#!/usr/bin/env bash
set -Eeuo pipefail

SOCKET_PATH="${XDG_RUNTIME_DIR:-/run/user/1000}/deskbrid.sock"
DAEMON_PID=""
SUMMARY=()

log() {
  printf '[deskbrid-demo] %s\n' "$*"
}

record_success() {
  SUMMARY+=("$1")
}

fail() {
  log "ERROR: $*"
  exit 1
}

cleanup() {
  if [[ -n "${DAEMON_PID}" ]] && kill -0 "${DAEMON_PID}" 2>/dev/null; then
    log "Stopping daemon"
    kill -INT "${DAEMON_PID}" 2>/dev/null || true
    wait "${DAEMON_PID}" 2>/dev/null || true
  fi
}

trap cleanup EXIT

run_step() {
  local label="$1"
  shift
  log "$label"
  "$@" || fail "$label failed"
  record_success "$label"
}

log "Building daemon"
cargo build --release || fail "cargo build failed"
record_success "cargo build"

log "Starting daemon in background"
./target/release/deskbrid daemon &
DAEMON_PID=$!
record_success "daemon start"

log "Waiting for socket at ${SOCKET_PATH}"
for _ in {1..100}; do
  if [[ -S "${SOCKET_PATH}" ]]; then
    record_success "socket ready"
    break
  fi
  sleep 0.1
done
[[ -S "${SOCKET_PATH}" ]] || fail "socket did not appear within 10 seconds"

log "Running system info"
INFO_OUTPUT="$(./target/release/deskbrid system info)" || fail "system info failed"
printf '%s\n' "${INFO_OUTPUT}"
record_success "system info"

log "Listing windows"
WINDOWS_OUTPUT="$(./target/release/deskbrid windows list)" || fail "windows list failed"
printf '%s\n' "${WINDOWS_OUTPUT}"
record_success "windows list"

log "Listing workspaces"
WORKSPACES_OUTPUT="$(./target/release/deskbrid workspaces list)" || fail "workspaces list failed"
printf '%s\n' "${WORKSPACES_OUTPUT}"
record_success "workspaces list"

log "Sending test notification"
./target/release/deskbrid notification send "deskbrid" "Demo" "Notification test from demo.sh" "normal" || fail "notification send failed"
record_success "notification send"

log "Taking screenshot"
SCREENSHOT_OUTPUT="$(./target/release/deskbrid screenshot)" || fail "screenshot failed"
printf '%s\n' "${SCREENSHOT_OUTPUT}"
record_success "screenshot"

log "Shutting down daemon"
cleanup
DAEMON_PID=""
record_success "daemon shutdown"

log "Summary"
for item in "${SUMMARY[@]}"; do
  printf '  ✓ %s\n' "${item}"
done
