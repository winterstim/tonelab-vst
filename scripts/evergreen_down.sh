#!/usr/bin/env bash
set -euo pipefail

BACKEND_PID_FILE="${BACKEND_PID_FILE:-/tmp/tonelab_evergreen_backend.pid}"
UI_PID_FILE="${UI_PID_FILE:-/tmp/tonelab_evergreen_ui.pid}"

stop_by_pid_file() {
  local label="$1"
  local pid_file="$2"

  if [[ ! -f "${pid_file}" ]]; then
    echo "No ${label} pid file: ${pid_file}"
    return 0
  fi

  local pid
  pid="$(cat "${pid_file}" 2>/dev/null || true)"
  if [[ -z "${pid}" ]]; then
    echo "Empty ${label} pid file: ${pid_file}"
    rm -f "${pid_file}"
    return 0
  fi

  if kill -0 "${pid}" >/dev/null 2>&1; then
    kill "${pid}" || true
    echo "Stopped ${label} pid=${pid}"
  else
    echo "${label} process not running (pid=${pid})"
  fi

  rm -f "${pid_file}"
}

stop_by_pid_file "backend" "${BACKEND_PID_FILE}"
stop_by_pid_file "ui" "${UI_PID_FILE}"
