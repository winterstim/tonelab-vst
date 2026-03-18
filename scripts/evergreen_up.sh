#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BACKEND_DIR="${ROOT_DIR}/backend"
BACKEND_BIN_DIR="${BACKEND_DIR}/.bin"
BACKEND_BIN_PATH="${BACKEND_BIN_DIR}/tonelab_evergreen_backend"
UI_DIR="${ROOT_DIR}/ui"
WASM_CRATE_MANIFEST="${ROOT_DIR}/wasm-engine-rust/Cargo.toml"
WASM_BUILD_OUTPUT="${ROOT_DIR}/wasm-engine-rust/target/wasm32-unknown-unknown/release/tonelab_wasm_engine.wasm"
WASM_ASSET_TARGET="${ROOT_DIR}/backend/assets/engine.wasm"
SIGN_SCRIPT="${ROOT_DIR}/backend/security/sign_bundle.sh"
BACKUP_SCRIPT="${ROOT_DIR}/scripts/backup_all.sh"
PUBLIC_KEY_FILE="${ROOT_DIR}/backend/security/dev_ed25519_public_key.b64"
TARGET_BUNDLED_DIR="${ROOT_DIR}/target/bundled"

BACKEND_PID_FILE="${BACKEND_PID_FILE:-/tmp/tonelab_evergreen_backend.pid}"
BACKEND_LOG_FILE="${BACKEND_LOG_FILE:-/tmp/tonelab_evergreen_backend.log}"
UI_PID_FILE="${UI_PID_FILE:-/tmp/tonelab_evergreen_ui.pid}"
UI_LOG_FILE="${UI_LOG_FILE:-/tmp/tonelab_evergreen_ui.log}"
PLUGIN_LOG_FILE="${PLUGIN_LOG_FILE:-/tmp/tonelab_vst.log}"
SYNC_URL_DEFAULT="http://localhost:8080/vst/sync"
UI_URL_DEFAULT="http://localhost:5173"

DO_BACKUP=1
DO_INSTALL=1
DO_START_BACKEND=1
DO_START_UI=1
SYNC_URL="${TONELAB_EVERGREEN_SYNC_URL:-$SYNC_URL_DEFAULT}"
UI_URL="${EVERGREEN_UI_URL:-$UI_URL_DEFAULT}"

print_help() {
  cat <<'EOF'
Usage: scripts/evergreen_up.sh [options]

Runs the Evergreen pipeline in sequence:
1) optional backup
2) build Rust wasm DSP bundle (full effects)
3) sign wasm bundle (Ed25519)
4) install plugin via xtask
5) start backend + optional UI dev server

Options:
  --skip-backup         Do not create project backup
  --skip-install        Skip `cargo run -p xtask -- install --release`
  --no-backend          Do not start backend
  --no-ui               Do not start UI dev server
  --sync-url <url>      Override sync URL (default: http://localhost:8080/vst/sync)
  --ui-url <url>        Override UI URL healthcheck (default: http://localhost:5173)
  -h, --help            Show this help
EOF
}

log() {
  printf "[evergreen_up] %s\n" "$*"
}

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --skip-backup)
        DO_BACKUP=0
        ;;
      --skip-install)
        DO_INSTALL=0
        ;;
      --no-backend)
        DO_START_BACKEND=0
        ;;
      --no-ui)
        DO_START_UI=0
        ;;
      --ui-url)
        shift
        [[ $# -gt 0 ]] || { echo "--ui-url requires value" >&2; exit 1; }
        UI_URL="$1"
        ;;
      --sync-url)
        shift
        [[ $# -gt 0 ]] || { echo "--sync-url requires value" >&2; exit 1; }
        SYNC_URL="$1"
        ;;
      -h|--help)
        print_help
        exit 0
        ;;
      *)
        echo "Unknown option: $1" >&2
        print_help
        exit 1
        ;;
    esac
    shift
  done
}

wait_for_sync() {
  local tries=40
  local i
  for ((i=1; i<=tries; i++)); do
    if curl -fsS "${SYNC_URL}" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.5
  done
  return 1
}

wait_for_ui() {
  local tries=40
  local i
  for ((i=1; i<=tries; i++)); do
    if curl -fsS "${UI_URL}" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.5
  done
  return 1
}

kill_backend_pid() {
  local pid="$1"
  if [[ -z "${pid}" ]]; then
    return 0
  fi
  if ! kill -0 "${pid}" >/dev/null 2>&1; then
    return 0
  fi
  kill "${pid}" >/dev/null 2>&1 || true
  sleep 0.4
  if kill -0 "${pid}" >/dev/null 2>&1; then
    kill -9 "${pid}" >/dev/null 2>&1 || true
  fi
}

is_tonelab_backend_pid() {
  local pid="$1"
  local cmdline
  cmdline="$(ps -p "${pid}" -o command= 2>/dev/null || true)"
  if [[ -z "${cmdline}" ]]; then
    return 1
  fi
  [[ "${cmdline}" == *"tonelab_evergreen_backend"* ]] || [[ "${cmdline}" == *"go-build"*"/backend"* ]]
}

stop_backend_listener() {
  local host_port="${SYNC_URL#*://}"
  host_port="${host_port%%/*}"
  local maybe_port="${host_port##*:}"
  if [[ "${maybe_port}" =~ ^[0-9]+$ ]] && command -v lsof >/dev/null 2>&1; then
    local listener_pids
    listener_pids="$(lsof -tiTCP:"${maybe_port}" -sTCP:LISTEN 2>/dev/null || true)"
    if [[ -n "${listener_pids}" ]]; then
      log "Stopping existing listener(s) on port ${maybe_port}: ${listener_pids}"
      for pid in ${listener_pids}; do
        if is_tonelab_backend_pid "${pid}"; then
          kill_backend_pid "${pid}"
        else
          log "Skipping non-tonelab process on ${maybe_port} (pid=${pid})"
        fi
      done
    fi
  fi
}

start_backend() {
  if [[ -f "${BACKEND_PID_FILE}" ]]; then
    local existing_pid
    existing_pid="$(cat "${BACKEND_PID_FILE}" 2>/dev/null || true)"
    if [[ -n "${existing_pid}" ]] && kill -0 "${existing_pid}" >/dev/null 2>&1; then
      log "Restarting backend (pid=${existing_pid}) to apply fresh wasm/signature..."
      kill_backend_pid "${existing_pid}"
    fi
  fi
  rm -f "${BACKEND_PID_FILE}" >/dev/null 2>&1 || true
  stop_backend_listener

  log "Starting backend..."
  (
    cd "${BACKEND_DIR}"
    mkdir -p "${BACKEND_BIN_DIR}"
    go build -o "${BACKEND_BIN_PATH}" .
    nohup env \
      EVERGREEN_ASSETS_DIR="${BACKEND_DIR}/assets" \
      EVERGREEN_SIGNATURE_FILE="engine.wasm.sig.b64" \
      EVERGREEN_WASM_SIGNATURE_B64="" \
      EVERGREEN_WEB_UI_URL="${UI_URL}" \
      "${BACKEND_BIN_PATH}" >"${BACKEND_LOG_FILE}" 2>&1 &
    echo $! > "${BACKEND_PID_FILE}"
  )

  if ! wait_for_sync; then
    echo "Backend failed to start or sync endpoint is unavailable: ${SYNC_URL}" >&2
    echo "Backend log: ${BACKEND_LOG_FILE}" >&2
    exit 1
  fi
  log "Backend is up. sync=${SYNC_URL}"
}

start_ui_server() {
  if [[ -f "${UI_PID_FILE}" ]]; then
    local existing_pid
    existing_pid="$(cat "${UI_PID_FILE}" 2>/dev/null || true)"
    if [[ -n "${existing_pid}" ]] && kill -0 "${existing_pid}" >/dev/null 2>&1; then
      log "UI dev server already running (pid=${existing_pid})."
      return 0
    fi
  fi

  log "Starting UI dev server..."
  (
    cd "${UI_DIR}"
    nohup npm run dev >"${UI_LOG_FILE}" 2>&1 &
    echo $! > "${UI_PID_FILE}"
  )

  if ! wait_for_ui; then
    echo "UI dev server failed to start or is unavailable: ${UI_URL}" >&2
    echo "UI log: ${UI_LOG_FILE}" >&2
    exit 1
  fi
  log "UI dev server is up. url=${UI_URL}"
}

cleanup_bundled_artifacts() {
  local app_bundle="${TARGET_BUNDLED_DIR}/tonelab_vst.app"
  local vst_bundle="${TARGET_BUNDLED_DIR}/tonelab_vst.vst3"
  rm -rf "${app_bundle}" "${vst_bundle}"

  # Strip AppleDouble sidecar files and Finder metadata that break codesign.
  for path in "${ROOT_DIR}/ui/dist" "${ROOT_DIR}/backend/assets" "${TARGET_BUNDLED_DIR}"; do
    if [[ -d "${path}" ]]; then
      find "${path}" -type f -name "._*" -delete >/dev/null 2>&1 || true
    fi
  done

  if command -v xattr >/dev/null 2>&1; then
    for path in "${ROOT_DIR}/ui/dist" "${ROOT_DIR}/backend/assets" "${TARGET_BUNDLED_DIR}"; do
      if [[ -e "${path}" ]]; then
        xattr -cr "${path}" >/dev/null 2>&1 || true
      fi
    done
  fi
}

export_runtime_env() {
  if [[ ! -f "${PUBLIC_KEY_FILE}" ]]; then
    echo "Public key file not found: ${PUBLIC_KEY_FILE}" >&2
    exit 1
  fi

  export TONELAB_EVERGREEN_SYNC_URL="${SYNC_URL}"
  TONELAB_EVERGREEN_ED25519_PUBLIC_KEY_B64="$(tr -d '\r\n' < "${PUBLIC_KEY_FILE}")"
  export TONELAB_EVERGREEN_ED25519_PUBLIC_KEY_B64
  export TONELAB_EVERGREEN_ALLOW_UNSIGNED="${TONELAB_EVERGREEN_ALLOW_UNSIGNED:-false}"
  export TONELAB_LOG_FILE_PATH="${PLUGIN_LOG_FILE}"
}

main() {
  parse_args "$@"

  need_cmd rustup
  need_cmd cargo
  need_cmd go
  need_cmd curl
  need_cmd openssl
  if [[ "${DO_START_UI}" -eq 1 ]]; then
    need_cmd npm
  fi

  if [[ "${DO_BACKUP}" -eq 1 ]]; then
    log "Creating backup..."
    "${BACKUP_SCRIPT}"
  fi

  log "Building full Rust DSP wasm bundle..."
  rustup target add wasm32-unknown-unknown >/dev/null
  cargo build --manifest-path "${WASM_CRATE_MANIFEST}" --target wasm32-unknown-unknown --release
  cp "${WASM_BUILD_OUTPUT}" "${WASM_ASSET_TARGET}"

  log "Signing wasm bundle..."
  "${SIGN_SCRIPT}"

  export_runtime_env

  if [[ "${DO_INSTALL}" -eq 1 ]]; then
    cleanup_bundled_artifacts
    log "Installing plugin..."
    (
      cd "${ROOT_DIR}"
      TONELAB_EVERGREEN_PUBLIC_KEY_B64="$(tr -d '\r\n' < "${PUBLIC_KEY_FILE}")" cargo run -p xtask -- install --release
    )
  fi

  if [[ "${DO_START_BACKEND}" -eq 1 ]]; then
    start_backend
  fi

  if [[ "${DO_START_UI}" -eq 1 ]]; then
    start_ui_server
  fi

  log "Done."
  log "Backend PID file: ${BACKEND_PID_FILE}"
  log "Backend log: ${BACKEND_LOG_FILE}"
  log "UI PID file: ${UI_PID_FILE}"
  log "UI log: ${UI_LOG_FILE}"
  log "Plugin log: ${PLUGIN_LOG_FILE}"
  log "Open any DAW and scan Tonelab VST3 from your normal plugin path."
}

main "$@"
