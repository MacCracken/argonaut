#!/bin/sh
# Argonaut test + benchmark runner (v2.0 — .tcyr/.bcyr format)

# ⚠ THIS RUNNER WAS ENTIRELY VACUOUS UNTIL THIS FIX, AND REPORTED SUCCESS.
#
# CC defaulted to `$HOME/.cyrius/bin/cc3`, a binary that has not existed for
# several toolchain releases. The compile step is `cat "$tcyr" | "$CC" > out
# 2>/dev/null`, so the failure was silenced; it produced a **0-byte** file;
# `chmod +x` succeeded on it; and executing an empty file with the exec bit set
# **exits 0** (the kernel hands it to the shell, which runs no commands). So
# TOTAL_FAIL never moved and `tests/test.sh` exited 0 having compiled and run
# NOTHING — 33 "passing" suites, every binary zero bytes.
#
# Three independent guards now, because any one of them alone would have let
# this through:
#   1. the compiler is resolved to something that exists, and its absence is
#      fatal rather than silent;
#   2. compiler stderr is SHOWN, and a compile failure fails the suite;
#   3. the produced binary must be non-empty before it is run.
CC="${1:-}"
if [ -z "$CC" ]; then
    for cand in "$HOME/.cyrius/bin/cycc" "$HOME/.cyrius/bin/cc5" "$HOME/.cyrius/bin/cc3"; do
        if [ -x "$cand" ]; then CC="$cand"; break; fi
    done
fi
if [ -z "$CC" ] || [ ! -x "$CC" ]; then
    echo "ERROR: no cyrius compiler found (tried \$1, cycc, cc5, cc3)." >&2
    echo "       Refusing to report success for tests that cannot be built." >&2
    exit 1
fi
echo "compiler: $CC"

BUILD="build"
mkdir -p "$BUILD"

TOTAL_FAIL=0
TOTAL_RUN=0

# Run .tcyr test suites from tests/tcyr/
for tcyr in tests/tcyr/*.tcyr; do
    suite=$(basename "$tcyr" .tcyr)
    echo "--- Compiling $suite ---"
    # ⚠ `cyrius build`, NOT a pipe into raw cycc.
    #
    # The old form was `cat "$tcyr" | "$CC" > out`, which bypasses cyrius's
    # dependency resolution and include-path setup entirely. The .tcyr chain
    # reaches libro, which reaches sigil's thin bundles, whose own
    # `include "src/sha_ni.cyr"` only resolves under a real dep resolve — so
    # raw cycc could never build these, no matter which compiler binary it was
    # pointed at. `cyrius build` handles it, which is why `cyrius build
    # src/main.cyr` has worked the whole time these tests did not.
    cp "$tcyr" "$BUILD/$suite.cyr"
    if ! cyrius build "$BUILD/$suite.cyr" "$BUILD/test_$suite"; then
        echo "ERROR: $suite failed to compile" >&2
        TOTAL_FAIL=$((TOTAL_FAIL + 1))
        continue
    fi
    # A zero-byte file is +x-able and exits 0. Refuse to count that as a pass.
    if [ ! -s "$BUILD/test_$suite" ]; then
        echo "ERROR: $suite produced an EMPTY binary — not running it" >&2
        TOTAL_FAIL=$((TOTAL_FAIL + 1))
        continue
    fi
    chmod +x "$BUILD/test_$suite"
    echo "--- Running $suite ---"
    "./$BUILD/test_$suite"
    TOTAL_FAIL=$((TOTAL_FAIL + $?))
    TOTAL_RUN=$((TOTAL_RUN + 1))
    echo ""
done

# Run .bcyr benchmarks from tests/bcyr/
for bcyr in tests/bcyr/*.bcyr; do
    bench=$(basename "$bcyr" .bcyr)
    echo "=== Compiling benchmark: $bench ==="
    # Same fix as the test loop: cyrius build, stderr kept, empty binary refused.
    cp "$bcyr" "$BUILD/${bench}_b.cyr"
    if ! cyrius build "$BUILD/${bench}_b.cyr" "$BUILD/${bench}_bench"; then
        echo "ERROR: benchmark $bench failed to compile" >&2
        TOTAL_FAIL=$((TOTAL_FAIL + 1))
        continue
    fi
    if [ ! -s "$BUILD/${bench}_bench" ]; then
        echo "ERROR: benchmark $bench produced an EMPTY binary" >&2
        TOTAL_FAIL=$((TOTAL_FAIL + 1))
        continue
    fi
    chmod +x "$BUILD/${bench}_bench"
    echo "=== Running benchmark: $bench ==="
    "./$BUILD/${bench}_bench"
    echo ""
done

echo "suites run: $TOTAL_RUN, failures: $TOTAL_FAIL"
# A run that executed nothing is a failure, not a pass.
if [ "$TOTAL_RUN" -eq 0 ]; then
    echo "ERROR: no test suite actually ran." >&2
    exit 1
fi
exit $TOTAL_FAIL
