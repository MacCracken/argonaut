#!/usr/bin/env bash
# boot-test.sh — boot argonaut in QEMU as PID 1, assert clean
# init + supervisor-loop reached.
#
# Shape adapted from kybernet/qemu/boot-test.sh — different
# binary, different stdout markers, same QEMU invocation shape.
#
# Asserts (greps stdout):
#   "argonaut: init system ready"    — init_new succeeded
#   "all systems nominal"            — full boot info printed
#   "argonaut: pid1 loop ready"      — supervisor loop entered
#                                      (proves sys_getpid() == 1
#                                      detection wired correctly)
#
# Exits 0 on full marker hit; non-zero if any marker missing.
#
# Usage:
#   qemu/boot-test.sh                            # default kernel
#   qemu/boot-test.sh /boot/vmlinuz-linux-lts    # explicit kernel
#   qemu/boot-test.sh "" 10                      # 10s timeout

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
KERNEL="${1:-}"
TIMEOUT="${2:-15}"
INITRAMFS="${SCRIPT_DIR}/initramfs.cpio.gz"

# Hard requirement — bail with a clear install hint rather than
# a cryptic "command not found" mid-pipeline.
if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
    echo "ERROR: qemu-system-x86_64 not on PATH."
    echo "  Arch:    sudo pacman -S qemu-system-x86"
    echo "  Debian:  sudo apt install qemu-system-x86"
    echo "  Fedora:  sudo dnf install qemu-system-x86"
    exit 1
fi

# Auto-pick a kernel — common Arch / Debian / Ubuntu paths.
if [ -z "$KERNEL" ]; then
    for cand in /boot/vmlinuz-linux-lts /boot/vmlinuz-linux /boot/vmlinuz-$(uname -r) /boot/vmlinuz; do
        if [ -f "$cand" ]; then KERNEL="$cand"; break; fi
    done
fi
[ -f "$KERNEL" ] || { echo "ERROR: kernel not found (tried /boot/vmlinuz-linux-lts, vmlinuz-linux, vmlinuz-$(uname -r), vmlinuz). Pass an explicit path as \$1."; exit 1; }

# Build the initramfs if needed.
if [ ! -f "$INITRAMFS" ] || [ "${PROJECT_DIR}/build/argonaut" -nt "$INITRAMFS" ]; then
    bash "${SCRIPT_DIR}/build-initramfs.sh"
fi

INIT_SIZE=$(wc -c < "${PROJECT_DIR}/build/argonaut")
echo "=== argonaut PID-1 BOOT TEST ==="
echo "  kernel:    $KERNEL"
echo "  initramfs: ${INITRAMFS} ($(du -h "$INITRAMFS" | cut -f1))"
echo "  init:      ${INIT_SIZE}B (argonaut)"
echo "  timeout:   ${TIMEOUT}s"
echo ""

LOG=$(mktemp /tmp/argonaut-boot.XXXXXX.log)
trap "rm -f $LOG" EXIT

# `rdinit=/sbin/init` tells the kernel to exec our binary as PID 1
# from the initramfs without trying /init first. `panic=5` ensures
# any kernel panic (e.g. argonaut returning from main when PID 1)
# triggers a clean qemu shutdown after 5s rather than hanging.
# `console=ttyS0` routes kernel + stdio to qemu's serial monitor.
# sakshi's `_sk_clock_init` panics if CPUID 0x80000007 EDX bit 8
# (invariant TSC) is absent. qemu's TCG emulation doesn't expose
# the bit even with `-cpu max,+invtsc`; only KVM + `-cpu host,+invtsc`
# faithfully passes it through. We KVM-accel when /dev/kvm is
# readable and fall back to TCG with a clear "harness needs KVM"
# diagnostic otherwise.
ACCEL_FLAGS="-cpu host,+invtsc -enable-kvm"
if [ ! -r /dev/kvm ]; then
    echo "WARNING: /dev/kvm not readable — running under TCG."
    echo "  sakshi will panic on missing invariant TSC; this run will fail."
    echo "  Add yourself to the 'kvm' group (Arch: usermod -aG kvm \$USER + relog)"
    echo "  or run as root."
    ACCEL_FLAGS="-cpu max,+invtsc"
fi

timeout "$TIMEOUT" qemu-system-x86_64 \
    -kernel "$KERNEL" \
    -initrd "$INITRAMFS" \
    -append "console=ttyS0 panic=5 rdinit=/sbin/init loglevel=4" \
    $ACCEL_FLAGS \
    -m 256M \
    -nographic \
    -no-reboot \
    -serial mon:stdio 2>&1 | tee "$LOG" | grep -E "argonaut:|kernel panic|Attempted to kill init" || true

echo ""
echo "=== marker check ==="

fail=0
for marker in \
    "argonaut: init system ready" \
    "argonaut: all systems nominal" \
    "argonaut: pid1 loop ready"; do
    if grep -qF "$marker" "$LOG"; then
        echo "  OK: $marker"
    else
        echo "  FAIL: missing marker — \"$marker\""
        fail=1
    fi
done

# Kernel panic = argonaut returned from main while PID 1. That's the
# original 1.6.0-gated failure mode — fail loudly if we see it.
if grep -qE "Attempted to kill init|Kernel panic" "$LOG"; then
    echo "  FAIL: kernel panicked — argonaut exited from main while PID 1"
    fail=1
fi

if [ $fail -eq 0 ]; then
    echo "=== BOOT TEST: OK ==="
    exit 0
else
    echo "=== BOOT TEST: FAIL ==="
    echo "  full log: $LOG (preserved for inspection)"
    trap - EXIT
    exit 1
fi
