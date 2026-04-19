# FILE ./scripts/env/env.oss.sh
#!/usr/bin/env bash
#
# Code-Command / OSSFS Rust environment bootstrap
# Target layout:
#   - RUSTUP_HOME=/mnt/oss/rustup
#   - CARGO_HOME=/mnt/oss/cargo
#   - PATH includes $CARGO_HOME/bin
#   - Workspace root: /workspace (ephemeral)
#   - Persistent volume: /mnt/oss (OSSFS, ~16 EB logical capacity)
#
# Usage:
#   source ./scripts/env/env.oss.sh
#   # or, from anywhere:
#   #   source /workspace/scripts/env/env.oss.sh

set -euo pipefail

cc_env_log() {
  printf '[cc-env] %s\n' "$*" 1>&2
}

# --- Resolve and export core variables --------------------------------------

export RUSTUP_HOME="/mnt/oss/rustup"
export CARGO_HOME="/mnt/oss/cargo"

# Ensure bin dir exists so PATH entry is meaningful.
if [ ! -d "${CARGO_HOME}/bin" ]; then
  mkdir -p "${CARGO_HOME}/bin"
fi

case ":${PATH}:" in
  *":${CARGO_HOME}/bin:"*)
    # already present
    ;;
  *)
    export PATH="${CARGO_HOME}/bin:${PATH}"
    ;;
esac

# --- OSSFS mount health + capacity checks -----------------------------------

OSS_ROOT="/mnt/oss"

if [ ! -d "${OSS_ROOT}" ]; then
  cc_env_log "ERROR: ${OSS_ROOT} does not exist; OSSFS mount is missing."
  return 1 2>/dev/null || exit 1
fi

# Basic writability test: create and delete a small temp file.
cc_env_log "Checking writability of ${OSS_ROOT} ..."
cc_tmp="${OSS_ROOT}/.cc_env_$$.tmp"
if ! (echo "ok" > "${cc_tmp}" 2>/dev/null); then
  cc_env_log "ERROR: ${OSS_ROOT} is not writable. Check OSSFS mount permissions."
  return 1 2>/dev/null || exit 1
fi
rm -f "${cc_tmp}"

# Capacity check: require > 1 exabyte free (logical).
# df may report TB/PB/EB; we normalize via block count to avoid unit parsing.
#
# We only reject if free blocks are strictly below a 1 EB minimum assuming 1 KiB
# blocks, which is ~1e9 GiB. This is conservative but safe for your described
# 16 EB mount.
cc_env_log "Checking available capacity on ${OSS_ROOT} ..."

# Use POSIX df output (1K blocks where possible).
# Fallback to generic df if -k is unavailable.
if df -k "${OSS_ROOT}" >/dev/null 2>&1; then
  # shellcheck disable=SC2012
  df_line="$(df -k "${OSS_ROOT}" | awk 'NR==2 {print}')"
  # 1K-blocks is usually the second column.
  free_1k="$(printf '%s\n' "${df_line}" | awk '{print $(NF-2)}')"
else
  df_line="$(df "${OSS_ROOT}" | awk 'NR==2 {print}')"
  free_1k="$(printf '%s\n' "${df_line}" | awk '{print $(NF-2)}')"
fi

# Minimum free in 1K-blocks for 1 EB (approx):
# 1 EB = 2^60 bytes ≈ 2^50 KiB = 1,125,899,906,842 1K-blocks.
min_free_1k=1125899906842

# Guard against non-integer or empty output
case "${free_1k}" in
  ''|*[!0-9]*)
    cc_env_log "WARNING: Could not parse free capacity from df output; continuing anyway."
    ;;
  *)
    if [ "${free_1k}" -lt "${min_free_1k}" ]; then
      cc_env_log "ERROR: ${OSS_ROOT} reports less than 1 EB free (${free_1k} 1K-blocks)."
      cc_env_log "       Aborting Rust toolchain use to avoid overcommitting OSSFS volume."
      return 1 2>/dev/null || exit 1
    fi
    ;;
esac

cc_env_log "Environment configured."
cc_env_log "  RUSTUP_HOME=${RUSTUP_HOME}"
cc_env_log "  CARGO_HOME=${CARGO_HOME}"
cc_env_log "  PATH=${PATH}"

# Mark that env has been initialized in this shell (optional convenience).
export CC_OSS_ENV=1
