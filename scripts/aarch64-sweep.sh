#!/usr/bin/env bash
# aarch64-sweep.sh — build + run the full .tcyr sweep on aarch64
# via qemu-user. Best-effort developer aid; CI ships smoke-only
# until a native aarch64 runner is allocated (see
# docs/architecture/001-cross-arch-aarch64.md).
#
# Usage:
#   scripts/aarch64-sweep.sh            # build + sweep
#   scripts/aarch64-sweep.sh smoke      # smoke only
#
# Requires: cc5_aarch64 in $PATH (cyrius toolchain), qemu-aarch64
# (qemu-user package). Builds drop into /tmp/argonaut-aa/.

set -uo pipefail
# Loop body intentionally tolerates per-test build / run failures —
# they're tallied into the budget. `set -e` removed at top-level
# so a single test miss doesn't abort the sweep.

if ! command -v cyrius >/dev/null 2>&1; then
    echo "FAIL: cyrius not in PATH"; exit 1
fi
if [ ! -x "$HOME/.cyrius/bin/cc5_aarch64" ] && ! command -v cc5_aarch64 >/dev/null 2>&1; then
    echo "FAIL: cc5_aarch64 not in toolchain bin dir"; exit 1
fi
QEMU="$(command -v qemu-aarch64-static 2>/dev/null || command -v qemu-aarch64 2>/dev/null || true)"
if [ -z "$QEMU" ]; then
    echo "FAIL: qemu-aarch64(-static) not on PATH"; exit 1
fi

OUT=/tmp/argonaut-aa
mkdir -p "$OUT"

echo "=== build (aarch64, DCE) ==="
CYRIUS_DCE=1 cyrius build --aarch64 src/main.cyr "$OUT/argonaut" 2>&1 \
    | grep -vE '^  dead:' | tail -5
file "$OUT/argonaut" | grep -q "aarch64" || { echo "FAIL: not aarch64 ELF"; exit 1; }
echo "size: $(wc -c < "$OUT/argonaut") bytes"

echo "=== smoke ==="
"$QEMU" "$OUT/argonaut" | tee "$OUT/smoke.out" | tail -5
grep -q "all systems nominal" "$OUT/smoke.out" || { echo "FAIL: smoke output mismatch"; exit 1; }

if [ "${1:-full}" = "smoke" ]; then
    echo "OK (smoke only)"
    exit 0
fi

echo "=== test sweep ==="
pass=0; fail=0; build_fail=0; asserts=0
known_fail_count=0
for t in tests/tcyr/*.tcyr; do
    name=$(basename "$t" .tcyr)
    bin="$OUT/$name"
    if ! CYRIUS_DCE=1 cyrius build --aarch64 "$t" "$bin" > "$OUT/$name.build.log" 2>&1; then
        build_fail=$((build_fail + 1))
        echo "BUILD-FAIL: $name"
        continue
    fi
    out=$("$QEMU" "$bin" 2>&1 || true)
    sum=$(echo "$out" | grep -E "^[0-9]+ passed," || true)
    if echo "$sum" | grep -q "0 failed"; then
        pass=$((pass + 1))
        n=$(echo "$sum" | grep -oP "^\d+")
        asserts=$((asserts + n))
    else
        fail=$((fail + 1))
        # Known failures under qemu-user / upstream sigil bug —
        # see docs/architecture/001-cross-arch-aarch64.md.
        case "$name" in
            audit_findings|audit_extended)
                known_fail_count=$((known_fail_count + 1))
                echo "KNOWN-FAIL: $name (see arch doc 001)"
                ;;
            *)
                echo "FAIL: $name -> ${sum:-no summary}"
                ;;
        esac
    fi
done

echo "---"
echo "suites: $pass pass, $fail fail ($known_fail_count known), $build_fail build_fail"
echo "assertions: $asserts"
if [ "$fail" -gt "$known_fail_count" ] || [ "$build_fail" -gt 0 ]; then
    exit 1
fi
echo "OK (within known-failure budget)"
