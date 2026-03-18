#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BACKUP_DIR="${ROOT_DIR}/backups"
STAMP="$(date +"%Y%m%d_%H%M%S")"
ARCHIVE_NAME="tonelab-vst-backup-${STAMP}.tar.gz"
ARCHIVE_PATH="${BACKUP_DIR}/${ARCHIVE_NAME}"

mkdir -p "${BACKUP_DIR}"

tar \
  --exclude="./backups" \
  --exclude="./target" \
  --exclude="./ui/node_modules" \
  -czf "${ARCHIVE_PATH}" \
  -C "${ROOT_DIR}" \
  .

echo "Backup created: ${ARCHIVE_PATH}"
