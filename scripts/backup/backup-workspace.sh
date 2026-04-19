# FILE ./scripts/backup/backup-workspace.sh
#!/usr/bin/env bash
#
# Snapshot /workspace into /mnt/oss/backups with simple retention.
#
# Excludes:
#   - any "target/" directories
#   - any ".git/" directories
#
# Naming:
#   /mnt/oss/backups/workspace-YYYYMMDD-HHMMSS.tar.gz
#
# Retention:
#   Keep the 10 most recent backups, delete older ones.

set -euo pipefail

BACKUP_ROOT="/mnt/oss/backups"
WORKSPACE_ROOT="/workspace"
TIMESTAMP="$(date +%Y%m%d-%H%M%S)"
ARCHIVE_NAME="workspace-${TIMESTAMP}.tar.gz"
ARCHIVE_PATH="${BACKUP_ROOT}/${ARCHIVE_NAME}"

log() {
  printf '[cc-backup] %s\n' "$*" 1>&2
}

if [ ! -d "${WORKSPACE_ROOT}" ]; then
  log "ERROR: Workspace root ${WORKSPACE_ROOT} not found."
  exit 1
fi

mkdir -p "${BACKUP_ROOT}"

log "Creating backup: ${ARCHIVE_PATH}"

(
  cd "${WORKSPACE_ROOT}"
  # Use POSIX tar where possible; exclude patterns are widely supported.
  tar \
    --exclude='*/target/*' \
    --exclude='*/.git/*' \
    -czf "${ARCHIVE_PATH}" \
    .
)

log "Backup created."

# --- Retention policy: keep latest 10 ---------------------------------------

log "Applying retention policy (keep 10 most recent backups) ..."

# List backups sorted by modification time (newest first)
# and delete everything after the 10th entry.
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
