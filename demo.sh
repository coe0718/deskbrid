#!/usr/bin/env bash
set -Eeuo pipefail

SOCKET_PATH="${XDG_RUNTIME_DIR:-/run/user/1000}/deskbrid/socket"
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
cargo build || fail "cargo build failed"
record_success "cargo build"

log "Starting daemon in background"
./target/debug/deskbrid daemon &
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

log "Subscribing to window:focus and clipboard for 3 seconds"
set +e
timeout 3s ./target/debug/deskbrid subscribe window:focus clipboard
subscribe_status=$?
set -e
if [[ "${subscribe_status}" -ne 0 && "${subscribe_status}" -ne 124 ]]; then
  fail "subscription demo failed"
fi
record_success "event subscription"

log "Running deskbrid info"
INFO_OUTPUT="$(./target/debug/deskbrid info)" || fail "deskbrid info failed"
printf '%s\n' "${INFO_OUTPUT}"
record_success "info"

log "Running inject:type with a GNOME Wayland session requirement note"
log "This step requires a running GNOME Wayland session with input injection permissions"
./target/debug/deskbrid action inject:type '{"text":"hello world"}' || fail "inject:type failed"
record_success "inject:type"

log "Sending test notification"
./target/debug/deskbrid action notification:send '{"summary":"Deskbrid demo","body":"Notification test from demo.sh","urgency":"low"}' || fail "notification:send failed"
record_success "notification:send"

log "Taking screenshot"
SCREENSHOT_OUTPUT="$(./target/debug/deskbrid action screenshot)" || fail "screenshot failed"
printf '%s\n' "${SCREENSHOT_OUTPUT}"
record_success "screenshot"

log "Listing windows"
WINDOWS_OUTPUT="$(./target/debug/deskbrid action window:list)" || fail "window:list failed"
printf '%s\n' "${WINDOWS_OUTPUT}"
record_success "window:list"

log "Shutting down daemon"
cleanup
DAEMON_PID=""
record_success "daemon shutdown"

log "Summary"
for item in "${SUMMARY[@]}"; do
  printf ' - %s\n' "${item}"
done
