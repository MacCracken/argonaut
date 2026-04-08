#!/bin/sh
# Argonaut test + benchmark runner (v2.0 — .tcyr/.bcyr format)

CC="${1:-$HOME/.cyrius/bin/cc2}"
BUILD="build"
mkdir -p "$BUILD"

TOTAL_FAIL=0

# Run .tcyr test suites from tests/tcyr/
for tcyr in tests/tcyr/*.tcyr; do
    suite=$(basename "$tcyr" .tcyr)
    echo "--- Compiling $suite ---"
    cat "$tcyr" | "$CC" > "$BUILD/test_$suite" 2>/dev/null
    chmod +x "$BUILD/test_$suite"
    echo "--- Running $suite ---"
    "./$BUILD/test_$suite"
    TOTAL_FAIL=$((TOTAL_FAIL + $?))
    echo ""
done

# Run .bcyr benchmarks from tests/bcyr/
for bcyr in tests/bcyr/*.bcyr; do
    bench=$(basename "$bcyr" .bcyr)
    echo "=== Compiling benchmark: $bench ==="
    cat "$bcyr" | "$CC" > "$BUILD/${bench}_bench" 2>/dev/null
    chmod +x "$BUILD/${bench}_bench"
    echo "=== Running benchmark: $bench ==="
    "./$BUILD/${bench}_bench"
    echo ""
done

exit $TOTAL_FAIL
