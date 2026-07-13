#!/usr/bin/env bash
# bench-history.sh — Run benchmarks and append results to CSV history.
#
# Usage: ./scripts/bench-history.sh [label]
#   label  Optional tag for this run (e.g. "baseline", "post-audit")
#
# Output: bench-history.csv (appended, never truncated)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
CSV="${PROJECT_DIR}/bench-history.csv"
LABEL="${1:-$(git -C "${PROJECT_DIR}" rev-parse --short HEAD 2>/dev/null || echo 'unknown')}"
TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

# Header if file doesn't exist
if [[ ! -f "${CSV}" ]]; then
    echo "timestamp,label,benchmark,avg_us,min_us,max_us" > "${CSV}"
fi

echo "=== Argonaut Benchmark Run: ${LABEL} @ ${TIMESTAMP} ==="

# Build bench_main via cyrius (auto-resolves manifest deps + applies DCE).
# Use the toolchain-aware `cyrius build` driver (the underlying compiler
# was cc3 → cc5 → cycc across Cyrius major bumps; the driver wraps it).
cd "${PROJECT_DIR}"
mkdir -p build
CYRIUS_DCE=1 cyrius build src/bench_main.cyr build/argonaut_bench >/dev/null

# Run and capture output
BENCH_OUTPUT=$(./build/argonaut_bench 2>&1 || true)
echo "${BENCH_OUTPUT}"

# Parse output lines like:
#   bench_name: 2.389us avg (min=908ns max=12.432us) [10000 iters]
# Cyrius 6.4.x emits DECIMAL values with mixed units (ns/us/ms); older
# toolchains emitted integer microseconds (4us). Normalize every token to
# microseconds (avg_us/min_us/max_us) with 3-decimal precision so rows stay
# comparable across the whole history regardless of the emitting toolchain.
echo "${BENCH_OUTPUT}" | LC_ALL=C awk -v ts="${TIMESTAMP}" -v lbl="${LABEL}" '
    function tous(tok,   v, u) {
        if (!match(tok, /[0-9.]+/)) { return "" }
        v = substr(tok, RSTART, RLENGTH) + 0
        u = tok; gsub(/[0-9. \t]/, "", u)
        if (u == "ns") { return v / 1000 }
        if (u == "ms") { return v * 1000 }
        if (u == "s")  { return v * 1000000 }
        return v   # "us" or bare
    }
    /avg.*min=.*max=/ {
        name = $0; sub(/:.*/, "", name); gsub(/^[ \t]+|[ \t]+$/, "", name)
        a = $0; sub(/^.*:[ \t]*/, "", a);  sub(/[ \t]*avg.*/, "", a)
        mn = $0; sub(/^.*min=/, "", mn);   sub(/[ \t].*/, "", mn)
        mx = $0; sub(/^.*max=/, "", mx);   sub(/[)].*/, "", mx); gsub(/[ \t]/, "", mx)
        av = tous(a); mnv = tous(mn); mxv = tous(mx)
        if (name != "" && av != "" && mnv != "" && mxv != "") {
            printf "%s,%s,%s,%.3f,%.3f,%.3f\n", ts, lbl, name, av, mnv, mxv
        }
    }
' >> "${CSV}"

echo ""
echo "Results appended to ${CSV}"
