#!/usr/bin/env bash
# aarch64-pi-smoke.sh — cross-build argonaut for aarch64, scp to a
# real-hw Pi, run the non-PID-1 smoke, assert the boot-info markers.
#
# Closes the gap from `scripts/aarch64-sweep.sh` (qemu-user) — that
# script validates aarch64 codegen under emulation; this one
# validates **real hardware** execution (RPi4 / Pi 5 / Apple
# Silicon under Asahi / Graviton / Ampere). Hits the "Real-hardware
# smoke" item that's open under "Native aarch64" in
# `docs/development/roadmap.md`.
#
# Usage:
#   scripts/aarch64-pi-smoke.sh                 # default ssh alias "pi"
#   scripts/aarch64-pi-smoke.sh root@pi.local   # explicit host
#   scripts/aarch64-pi-smoke.sh pi /tmp/argo    # remote stage dir
#
# Requires:
#   - cycc_aarch64 in the toolchain bin dir (renamed from
#     cc5_aarch64 in Cyrius 6.0)
#   - ssh access to the target (key-based, no interactive prompt)
#   - the target arch must be aarch64 (script checks `uname -m`)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

REMOTE="${1:-pi}"
REMOTE_DIR="${2:-/tmp/argonaut-smoke}"
BINARY="${PROJECT_DIR}/build/argonaut-aarch64"

if [ ! -x "$HOME/.cyrius/bin/cycc_aarch64" ]; then
    echo "ERROR: cycc_aarch64 not in toolchain bin dir."
    echo "  Bump cyrius or pull a release that bundles the aarch64 codegen."
    exit 1
fi

if ! command -v ssh >/dev/null 2>&1 || ! command -v scp >/dev/null 2>&1; then
    echo "ERROR: ssh / scp not on PATH."
    exit 1
fi

echo "=== argonaut aarch64 real-hw smoke ==="
echo "  target:    ${REMOTE}"
echo "  stage dir: ${REMOTE_DIR}"
echo ""

# Cross-build (idempotent — cyrius skips work if outputs are fresh).
if [ ! -f "$BINARY" ] || [ "${PROJECT_DIR}/cyrius.cyml" -nt "$BINARY" ]; then
    echo "Building build/argonaut-aarch64 (CYRIUS_DCE=1)..."
    (cd "$PROJECT_DIR" && CYRIUS_DCE=1 cyrius build --aarch64 src/main.cyr "$BINARY" >/dev/null)
fi
[ -f "$BINARY" ] || { echo "ERROR: cross-build did not produce $BINARY"; exit 1; }
file "$BINARY" | grep -q "aarch64" || { echo "ERROR: $BINARY is not an aarch64 ELF"; exit 1; }

echo "  binary:    $(wc -c < "$BINARY") bytes"
echo ""

# Probe the target — fail early on connection / arch issues.
echo "Probing ${REMOTE}..."
REMOTE_ARCH=$(ssh -o BatchMode=yes -o ConnectTimeout=5 "$REMOTE" 'uname -m' 2>/dev/null || true)
if [ -z "$REMOTE_ARCH" ]; then
    echo "ERROR: ssh ${REMOTE} failed (timeout / auth / no such host)."
    echo "  Try \`ssh -v ${REMOTE}\` interactively to diagnose."
    exit 1
fi
echo "  uname -m:  ${REMOTE_ARCH}"
if [ "$REMOTE_ARCH" != "aarch64" ] && [ "$REMOTE_ARCH" != "arm64" ]; then
    echo "ERROR: target is not aarch64 (uname -m = ${REMOTE_ARCH})."
    echo "  This script validates aarch64 binaries; cross-arch smoke "
    echo "  isn't meaningful."
    exit 1
fi

# Stage + run + capture.
echo ""
echo "Staging..."
ssh -o BatchMode=yes "$REMOTE" "mkdir -p '${REMOTE_DIR}'" || {
    echo "ERROR: failed to mkdir ${REMOTE_DIR} on target."
    exit 1
}
scp -o BatchMode=yes -q "$BINARY" "${REMOTE}:${REMOTE_DIR}/argonaut-aarch64" || {
    echo "ERROR: scp failed."
    exit 1
}
ssh -o BatchMode=yes "$REMOTE" "chmod +x '${REMOTE_DIR}/argonaut-aarch64'"

echo "Running..."
LOG=$(mktemp /tmp/argonaut-pi-smoke.XXXXXX.log)
trap "rm -f $LOG" EXIT

ssh -o BatchMode=yes "$REMOTE" "'${REMOTE_DIR}/argonaut-aarch64'" > "$LOG" 2>&1
RC=$?

echo ""
echo "=== runtime output ==="
sed 's/^/  /' "$LOG"
echo ""

echo "=== marker check ==="
fail=0
for marker in \
    "argonaut: init system ready" \
    "argonaut: all systems nominal"; do
    if grep -qF "$marker" "$LOG"; then
        echo "  OK: $marker"
    else
        echo "  FAIL: missing marker — \"$marker\""
        fail=1
    fi
done

if [ "$RC" -ne 0 ]; then
    echo "  FAIL: argonaut exited non-zero (rc=$RC)"
    fail=1
fi

if [ $fail -eq 0 ]; then
    echo ""
    echo "=== AARCH64 PI SMOKE: OK ==="
    echo "  argonaut x86_64 → aarch64 cross-build runs cleanly on real hardware."
    exit 0
else
    echo "=== AARCH64 PI SMOKE: FAIL ==="
    echo "  full log: $LOG (preserved for inspection)"
    trap - EXIT
    exit 1
fi
