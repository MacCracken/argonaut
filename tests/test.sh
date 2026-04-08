#!/bin/sh
# Argonaut test + benchmark runner
set -e

CC="${1:-$HOME/.cyrius/bin/cc2}"
BUILD="build"
mkdir -p "$BUILD"

TOTAL_PASS=0
TOTAL_FAIL=0

# test_serde unblocked — cc2 bug #12 fixed (v1.11.1+)
for suite in test_types test_init test_lifecycle test_modules test_display test_advanced test_api test_audit test_serde; do
    echo "--- Compiling $suite ---"
    cat "src/${suite}.cyr" | "$CC" > "$BUILD/$suite" 2>/dev/null
    chmod +x "$BUILD/$suite"
    echo "--- Running $suite ---"
    "./$BUILD/$suite"
    RESULT=$?
    TOTAL_FAIL=$((TOTAL_FAIL + RESULT))
    echo ""
done

echo "=== Compiling benchmarks ==="
cat src/bench_main.cyr | "$CC" > "$BUILD/argonaut_bench" 2>/dev/null
chmod +x "$BUILD/argonaut_bench"
echo "=== Running benchmarks ==="
"./$BUILD/argonaut_bench"

exit $TOTAL_FAIL
