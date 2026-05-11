#!/usr/bin/env bash
# build-initramfs.sh — stage a minimal initramfs with argonaut as
# /sbin/init for the qemu PID-1 harness.
#
# Shape adapted from kybernet/qemu/build-initramfs.sh per the
# "pull pattern, not code" convention in
# docs/development/roadmap.md. Standalone — does not depend on
# kybernet at runtime; only the harness scaffold shape was lifted.
#
# Output: qemu/initramfs.cpio.gz + qemu/initramfs/ staging tree.
#
# Usage:
#   qemu/build-initramfs.sh             # default — argonaut binary
#   qemu/build-initramfs.sh BINARY      # override the init binary

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
INITRAMFS_DIR="${SCRIPT_DIR}/initramfs"
BINARY="${1:-${PROJECT_DIR}/build/argonaut}"

# Build argonaut if the binary is missing or older than the manifest.
if [ ! -f "$BINARY" ] || [ "${PROJECT_DIR}/cyrius.cyml" -nt "$BINARY" ]; then
    echo "Building argonaut (CYRIUS_DCE=1)..."
    (cd "$PROJECT_DIR" && CYRIUS_DCE=1 cyrius build src/main.cyr "$BINARY" >/dev/null)
fi

[ -f "$BINARY" ] || { echo "ERROR: $BINARY not found after build"; exit 1; }

echo "Staging initramfs at ${INITRAMFS_DIR}..."

rm -rf "${INITRAMFS_DIR}"
mkdir -p "${INITRAMFS_DIR}"/{bin,sbin,dev,proc,sys,run,tmp,etc,usr/bin}

# Install argonaut as /sbin/init — kernel hands off to this on rdinit.
cp "$BINARY" "${INITRAMFS_DIR}/sbin/init"
chmod +x "${INITRAMFS_DIR}/sbin/init"

# Bundle busybox for shells / test helpers used by the M3 orphan-reap
# and L3 controlling-TTY test variants. Optional — boot smoke works
# without it; M3/L3 end-to-end tests need it.
BUSYBOX=""
for cand in /usr/lib/initcpio/busybox /usr/bin/busybox /bin/busybox; do
    if [ -x "$cand" ]; then BUSYBOX="$cand"; break; fi
done
if [ -n "$BUSYBOX" ]; then
    cp "$BUSYBOX" "${INITRAMFS_DIR}/bin/busybox"
    chmod +x "${INITRAMFS_DIR}/bin/busybox"
    for cmd in sh ls cat mount ps kill sleep echo dmesg true false awk cut grep printf; do
        ln -sf busybox "${INITRAMFS_DIR}/bin/${cmd}"
    done
    echo "  bundled busybox from $BUSYBOX"

    # Modern distros ship busybox dynamically linked (Arch's
    # /usr/lib/initcpio/busybox needs /lib64/ld-linux-x86-64.so.2 +
    # libc.so.6). The 1.6.2 L3 harness needs an actual execve to
    # validate the setsid → exec chain, so bundle the dynamic
    # loader + libc when the binary requires them. If we detect a
    # static binary (no INTERP segment), skip this step.
    if file "$BUSYBOX" 2>/dev/null | grep -q "dynamically linked"; then
        mkdir -p "${INITRAMFS_DIR}/lib64" "${INITRAMFS_DIR}/usr/lib"
        # Resolve each NEEDED library + the interpreter through
        # the host's ldd output; copy them into the initramfs at
        # the same path so the dynamic loader finds them.
        for lib in $(ldd "$BUSYBOX" 2>/dev/null | awk '/=>/ {print $3} /^\s*\//{print $1}'); do
            [ -n "$lib" ] || continue
            [ -f "$lib" ] || continue
            tgt_dir="${INITRAMFS_DIR}$(dirname "$lib")"
            mkdir -p "$tgt_dir"
            cp "$lib" "$tgt_dir/"
        done
        echo "  bundled dynamic-loader + libc (busybox is dynamically linked)"
    fi
else
    echo "  WARNING: busybox not found — M3/L3 end-to-end harness variants will be unavailable"
fi

# Minimal /etc/hosts so the resolver path doesn't fail on lookup
# during boot-time service definitions. localhost only — the
# harness doesn't exercise external host lookups.
cat > "${INITRAMFS_DIR}/etc/hosts" << 'EOF'
127.0.0.1 localhost
::1       localhost
EOF

# Essential device nodes — kernel won't create these for us inside
# a stripped initramfs. mknod requires CAP_MKNOD; skip silently if
# we don't have it (the harness still runs, console output just
# routes elsewhere).
sudo mknod "${INITRAMFS_DIR}/dev/console" c 5 1 2>/dev/null || true
sudo mknod "${INITRAMFS_DIR}/dev/null"    c 1 3 2>/dev/null || true
sudo mknod "${INITRAMFS_DIR}/dev/ttyS0"   c 4 64 2>/dev/null || true
sudo chmod 666 "${INITRAMFS_DIR}/dev/console" "${INITRAMFS_DIR}/dev/null" "${INITRAMFS_DIR}/dev/ttyS0" 2>/dev/null || true

cd "${INITRAMFS_DIR}"
find . | bsdcpio -o -H newc 2>/dev/null | gzip > "${SCRIPT_DIR}/initramfs.cpio.gz"

INIT_SIZE=$(wc -c < "${INITRAMFS_DIR}/sbin/init")
TOTAL_SIZE=$(du -h "${SCRIPT_DIR}/initramfs.cpio.gz" | cut -f1)
echo "Done: ${SCRIPT_DIR}/initramfs.cpio.gz (${TOTAL_SIZE}, init=${INIT_SIZE}B)"
