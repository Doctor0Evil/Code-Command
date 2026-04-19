# FILE ./scripts/backup/backup-workspace.sh
#!/usr/bin/env bash
#
# Workspace backup with optional integrity verification.
#
# Modes:
#   ./backup-workspace.sh
#       create backup and apply retention (no verification)
#
#   ./backup-workspace.sh --verify
#       create backup, extract into /mnt/oss/tmp/cc-backup-verify-<id>,
#       run `cargo check --workspace` there, and only keep the backup
#       if verification succeeds.

set -euo pipefail

BACKUP_ROOT="/mnt/oss/backups"
WORKSPACE_ROOT="/workspace"
VERIFY_ROOT="/mnt/oss/tmp"
TIMESTAMP="$(date +%Y%m%d-%H%M%S)"
ARCHIVE_NAME="workspace-${TIMESTAMP}.tar.gz"
ARCHIVE_PATH="${BACKUP_ROOT}/${ARCHIVE_NAME}"

VERIFY_MODE=0

log() {
  printf '[cc-backup] %s\n' "$*" 1>&2
}

usage() {
  log "Usage: $0 [--verify]"
  exit 1
}

while [ "${#}" -gt 0 ]; do
  case "$1" in
    --verify)
      VERIFY_MODE=1
      shift
      ;;
    -*)
      usage
      ;;
    *)
      usage
      ;;
  esac
done

if [ ! -d "${WORKSPACE_ROOT}" ]; then
  log "ERROR: Workspace root ${WORKSPACE_ROOT} not found."
  exit 1
fi

mkdir -p "${BACKUP_ROOT}"

log "Creating backup: ${ARCHIVE_PATH}"

(
  cd "${WORKSPACE_ROOT}"
  tar \
    --exclude='*/target/*' \
    --exclude='*/.git/*' \
    -czf "${ARCHIVE_PATH}" \
    .
)

log "Backup created."

if [ "${VERIFY_MODE}" -eq 1 ]; then
  if ! command -v cargo >/dev/null 2>&1; then
    log "ERROR: cargo not found in PATH; cannot verify backup."
    rm -f "${ARCHIVE_PATH}"
    exit 1
  fi

  mkdir -p "${VERIFY_ROOT}"
  VERIFY_DIR="${VERIFY_ROOT}/cc-backup-verify-${TIMESTAMP}"

  log "Verifying backup by extracting to ${VERIFY_DIR} and running 'cargo check --workspace' ..."
  mkdir -p "${VERIFY_DIR}"

  tar -xzf "${ARCHIVE_PATH}" -C "${VERIFY_DIR}"

  (
    cd "${VERIFY_DIR}"
    if cargo check --workspace; then
      log "Verification succeeded."
    else
      log "ERROR: 'cargo check --workspace' failed on extracted backup."
      log "Removing invalid backup: ${ARCHIVE_PATH}"
      rm -f "${ARCHIVE_PATH}"
      # Keep the extracted directory for inspection.
      exit 1
    fi
  )

  # Optional: clean up verified temp directory to avoid clutter.
  # Comment out the next two lines if you want to keep the verified tree.
  log "Cleaning up verification directory ${VERIFY_DIR}"
  rm -rf "${VERIFY_DIR}"
fi

# --- Retention policy: keep latest 10 ---------------------------------------

log "Applying retention policy (keep 10 most recent backups) ..."

BACKUPS=()
while IFS= read -r path; do
  BACKUPS+=("$path")
done < <(find "${BACKUP_ROOT}" -maxdepth 1 -type f -name 'workspace-*.tar.gz' -printf '%T@ %p\n' | sort -nr | awk '{print $2}')

COUNT="${#BACKUPS[@]}"
if [ "${COUNT}" -le 10 ]; then
  log "Found ${COUNT} backup(s); nothing to prune."
  exit 0
fi

for ((i=10; i<COUNT; i++)); do
  old="${BACKUPS[$i]}"
  log "Removing old backup: ${old}"
  rm -f "${old}"
done

log "Retention policy applied."
