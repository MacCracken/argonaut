#!/usr/bin/env bash
# version.sh — Update the project version in all relevant files.
#
# Usage: ./scripts/version.sh <new-version>
#   e.g.: ./scripts/version.sh 0.90.0
#
# Updates: VERSION, Cargo.toml, Cargo.lock (via cargo check)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

if [[ $# -ne 1 ]]; then
    echo "Usage: $0 <new-version>"
    echo "  e.g.: $0 0.90.0"
    exit 1
fi

NEW_VERSION="$1"

# Validate semver format (basic check)
if ! echo "${NEW_VERSION}" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$'; then
    echo "Error: '${NEW_VERSION}' is not a valid semver version"
    exit 1
fi

OLD_VERSION=$(cat "${PROJECT_DIR}/VERSION")
echo "Updating version: ${OLD_VERSION} → ${NEW_VERSION}"

# 1. VERSION file
echo "${NEW_VERSION}" > "${PROJECT_DIR}/VERSION"

# 2. Cargo.toml — update the version field in [package]
sed -i "s/^version = \"${OLD_VERSION}\"/version = \"${NEW_VERSION}\"/" "${PROJECT_DIR}/Cargo.toml"

# 3. Cargo.lock — regenerate via cargo check
(cd "${PROJECT_DIR}" && cargo check --quiet 2>/dev/null)

echo "Done. Updated files:"
echo "  VERSION:    ${NEW_VERSION}"
echo "  Cargo.toml: $(grep '^version' "${PROJECT_DIR}/Cargo.toml" | head -1)"
echo "  Cargo.lock: synced"
