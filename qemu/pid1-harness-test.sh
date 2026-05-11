#!/usr/bin/env bash
# pid1-harness-test.sh — run argonaut as PID 1 with the
# `argonaut.harness=1` cmdline flag set so it exercises M3 +
# L3 end-to-end self-tests inline before powering off. Greps
# the qemu serial output for the harness markers and asserts
# both `m3 ok` and `l3 ok`.
#
# This is the end-to-end validation the 2026-04-26 audit gated
# on the QEMU PID-1 harness: unit-level mock-ups (audit_findings.tcyr
# `audit-m3-reaper-orphans` and `audit-l3-fork-setsid` groups)
# proved the *mechanism* under non-PID-1; this proves the same
# mechanism actually fires when argonaut is the real PID 1.
#
# Usage:
#   qemu/pid1-harness-test.sh                  # default kernel
#   qemu/pid1-harness-test.sh /boot/vmlinuz-…  # explicit kernel
#   qemu/pid1-harness-test.sh "" 30            # 30s timeout

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
KERNEL="${1:-}"
TIMEOUT="${2:-20}"
INITRAMFS="${SCRIPT_DIR}/initramfs.cpio.gz"

if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
    echo "ERROR: qemu-system-x86_64 not on PATH."
    echo "  Arch:    sudo pacman -S qemu-system-x86"
    echo "  Debian:  sudo apt install qemu-system-x86"
    exit 1
fi

if [ -z "$KERNEL" ]; then
    for cand in /boot/vmlinuz-linux-lts /boot/vmlinuz-linux /boot/vmlinuz-$(uname -r) /boot/vmlinuz; do
        if [ -f "$cand" ]; then KERNEL="$cand"; break; fi
    done
fi
[ -f "$KERNEL" ] || { echo "ERROR: kernel not found. Pass an explicit path as \$1."; exit 1; }

if [ ! -f "$INITRAMFS" ] || [ "${PROJECT_DIR}/build/argonaut" -nt "$INITRAMFS" ]; then
    bash "${SCRIPT_DIR}/build-initramfs.sh"
fi

ACCEL_FLAGS="-cpu host,+invtsc -enable-kvm"
if [ ! -r /dev/kvm ]; then
    echo "WARNING: /dev/kvm not readable — running under TCG (sakshi clock_init will panic)."
    ACCEL_FLAGS="-cpu max,+invtsc"
fi

echo "=== argonaut PID-1 HARNESS TEST (M3 + L3 end-to-end) ==="
echo "  kernel:    $KERNEL"
echo "  initramfs: ${INITRAMFS} ($(du -h "$INITRAMFS" | cut -f1))"
echo "  cmdline:   argonaut.harness=1"
echo "  timeout:   ${TIMEOUT}s"
echo ""

LOG=$(mktemp /tmp/argonaut-harness.XXXXXX.log)
trap "rm -f $LOG" EXIT

# `argonaut.harness=1` on the kernel cmdline → /proc/cmdline →
# `pid1_harness_requested()` returns 1 → harness mode runs.
# `panic=5` gives the kernel 5s to abort if argonaut returns from
# main while PID 1 (we want a clean poweroff via sys_reboot
# instead, but `panic=5` is the safety net).
timeout "$TIMEOUT" qemu-system-x86_64 \
    -kernel "$KERNEL" \
    -initrd "$INITRAMFS" \
    -append "console=ttyS0 panic=5 rdinit=/sbin/init argonaut.harness=1 loglevel=3" \
    $ACCEL_FLAGS \
    -m 256M \
    -nographic \
    -no-reboot \
    -serial mon:stdio 2>&1 | tee "$LOG" | grep -E "argonaut:|kernel panic|Attempted to kill init" || true

echo ""
echo "=== marker check ==="

fail=0
# Pipeline filters binary data-segment matches by requiring `^M`
# (CR terminator from qemu's serial output); the binary's
# .rodata strings end in `^@` (NUL) and are dropped.
RUNTIME_OUT=$(cat -v "$LOG" | tr '\r' '\n')

for marker in \
    "argonaut: harness mode" \
    "argonaut: harness m3 ok" \
    "argonaut: harness l3 marker:" \
    "argonaut: harness l3 ok" \
    "argonaut: harness done"; do
    if echo "$RUNTIME_OUT" | grep -aqF "$marker"; then
        echo "  OK: $marker"
    else
        echo "  FAIL: missing marker — \"$marker\""
        fail=1
    fi
done

if echo "$RUNTIME_OUT" | grep -aqE "^argonaut: harness (m3|l3) FAIL"; then
    echo "  FAIL: harness self-test reported failure:"
    echo "$RUNTIME_OUT" | grep -aE "^argonaut: harness (m3|l3) FAIL" | sed 's/^/    /'
    fail=1
fi

if grep -aqE "Attempted to kill init|Kernel panic" "$LOG"; then
    echo "  FAIL: kernel panicked — argonaut exited from main while PID 1"
    fail=1
fi

if [ $fail -eq 0 ]; then
    echo ""
    echo "$RUNTIME_OUT" | grep -aE "argonaut: harness l3 marker:" | head -1 | sed 's/^/  /'
    echo "=== HARNESS TEST: OK (M3 + L3 end-to-end) ==="
    exit 0
else
    echo "=== HARNESS TEST: FAIL ==="
    echo "  full log: $LOG (preserved for inspection)"
    trap - EXIT
    exit 1
fi
