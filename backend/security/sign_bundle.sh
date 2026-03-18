#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SECURITY_DIR="${ROOT_DIR}/security"
ASSETS_DIR="${ROOT_DIR}/assets"
KEYS_DIR="${SECURITY_DIR}/keys"

WASM_FILE="${1:-${ASSETS_DIR}/engine.wasm}"
SIG_FILE="${2:-${ASSETS_DIR}/engine.wasm.sig.b64}"
PUBLIC_KEY_B64_FILE="${3:-${SECURITY_DIR}/dev_ed25519_public_key.b64}"
PRIVATE_KEY_FILE="${KEYS_DIR}/dev_ed25519_private.pem"
PUBLIC_KEY_PEM_FILE="${KEYS_DIR}/dev_ed25519_public.pem"
PUBLIC_KEY_DER_FILE="${KEYS_DIR}/dev_ed25519_public.der"
SIGNATURE_BIN_FILE="${KEYS_DIR}/engine.wasm.sig.bin"

mkdir -p "${KEYS_DIR}"

if [[ ! -f "${WASM_FILE}" ]]; then
  echo "WASM bundle not found: ${WASM_FILE}" >&2
  exit 1
fi

if [[ ! -f "${PRIVATE_KEY_FILE}" ]]; then
  openssl genpkey -algorithm Ed25519 -out "${PRIVATE_KEY_FILE}"
fi

openssl pkey -in "${PRIVATE_KEY_FILE}" -pubout -out "${PUBLIC_KEY_PEM_FILE}" >/dev/null 2>&1
openssl pkey -pubin -in "${PUBLIC_KEY_PEM_FILE}" -outform DER -out "${PUBLIC_KEY_DER_FILE}" >/dev/null 2>&1

# SubjectPublicKeyInfo DER for Ed25519 stores a 32-byte raw key payload at the end.
tail -c 32 "${PUBLIC_KEY_DER_FILE}" | base64 | tr -d '\n' > "${PUBLIC_KEY_B64_FILE}"

openssl pkeyutl \
  -sign \
  -inkey "${PRIVATE_KEY_FILE}" \
  -rawin \
  -in "${WASM_FILE}" \
  -out "${SIGNATURE_BIN_FILE}"

base64 < "${SIGNATURE_BIN_FILE}" | tr -d '\n' > "${SIG_FILE}"

echo "Signed bundle: ${WASM_FILE}"
echo "Signature (base64): ${SIG_FILE}"
echo "Public key (base64): ${PUBLIC_KEY_B64_FILE}"
