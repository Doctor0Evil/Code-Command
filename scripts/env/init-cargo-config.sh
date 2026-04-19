# FILE ./scripts/env/init-cargo-config.sh
#!/usr/bin/env bash
#
# Idempotently create a Cargo configuration tuned for OSSFS-based builds.

set -euo pipefail

: "${CARGO_HOME:=/mnt/oss/cargo}"

CONFIG_DIR="${CARGO_HOME}/.cargo"
CONFIG_PATH="${CONFIG_DIR}/config.toml"

mkdir -p "${CONFIG_DIR}"

cat > "${CONFIG_PATH}" <<'EOF'
# FILE /mnt/oss/cargo/.cargo/config.toml
#
# Code-Command / OSSFS-aware Cargo configuration.
# - Limit build parallelism to avoid I/O saturation on networked storage.
# - Redirect target dir into persistent OSSFS volume.
# - Disable incremental compilation to reduce intermediate churn.

[build]
jobs = 4
target-dir = "/mnt/oss/target"
incremental = false
EOF
