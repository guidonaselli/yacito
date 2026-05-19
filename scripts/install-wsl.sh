#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

echo "⚠️  scripts/install-wsl.sh is deprecated. Redirecting to scripts/install-linux.sh..."
exec "${SCRIPT_DIR}/install-linux.sh" "$@"
